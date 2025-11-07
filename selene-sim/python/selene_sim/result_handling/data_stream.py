import socket
from pathlib import Path
from abc import ABC, abstractmethod
from typing import BinaryIO
import struct
from selectors import DefaultSelector, EVENT_READ
from dataclasses import dataclass
from selene_sim.exceptions import SeleneStartupError, SeleneTimeoutError
from selene_sim.timeout import Timeout, Timer


class DataStream(ABC):
    """Base for classes capable of streaming results"""

    @abstractmethod
    def read_chunk(self, length: int) -> bytes:
        pass

    @abstractmethod
    def next_shot(self):
        pass


@dataclass
class ClientConfiguration:
    shot_offset: int
    shot_increment: int
    n_shots: int

    def provides_shot(self, shot: int) -> bool:
        if shot > self.shot_offset + self.n_shots * self.shot_increment:
            return False
        if shot < self.shot_offset:
            return False
        if (shot - self.shot_offset) % self.shot_increment != 0:
            return False
        return True

    @staticmethod
    def unpack(data: bytes) -> "ClientConfiguration":
        assert len(data) == 24, "Invalid client configuration data"
        shot_offset, shot_increment, n_shots = struct.unpack("<QQQ", data)
        return ClientConfiguration(shot_offset, shot_increment, n_shots)


class TCPClient:
    def __init__(
        self,
        sock: socket.socket,
        configuration: ClientConfiguration,
        logfile: Path | None = None,
    ):
        self.socket = sock
        self.address = sock.getsockname()
        self.configuration = configuration
        self.receive_buffer = b""
        self.logfile_handle: BinaryIO | None = None
        self.is_open = True
        if logfile is not None:
            self.logfile_handle = logfile.open("wb")

    def sync(self):
        data = self.socket.recv(4096)
        if not data:
            self.is_open = False
            self.close()
            return
        self.receive_buffer += data
        if self.logfile_handle is not None:
            self.logfile_handle.write(data)

    def take(self, length: int) -> bytes:
        result = self.receive_buffer[:length]
        self.receive_buffer = self.receive_buffer[length:]
        return result

    def has_bytes(self, length: int) -> bool:
        return len(self.receive_buffer) >= length

    def close(self):
        if self.logfile_handle is not None:
            self.logfile_handle.close()
        self.socket.close()


class TCPStream(DataStream):
    """
    A class that encapsulates a TCP server socket, providing blocking
    methods for reading the result stream from one or more Selene
    instances.
    """

    def __init__(
        self,
        host: str = "localhost",
        port: int = 0,
        timeout: Timeout = Timeout(),
        logfile: Path | None = None,
        shot_offset: int = 0,
        shot_increment: int = 1,
    ):
        self.host = host
        self.port = port
        self.done = False
        self.selector = DefaultSelector()
        self.server_socket: socket.socket | None = None
        self.clients: list[TCPClient] = []
        self.clients_by_fileno: dict[int, int] = {}
        self.logfile = logfile
        self.current_shot = shot_offset
        self.shot_increment = shot_increment
        self.current_shot_client: TCPClient | None = None
        self.overall_timer = Timer(timeout.overall)
        self.shot_timer = Timer(timeout.per_shot)
        self.read_timer = Timer(timeout.per_result)
        self.connect_timer = Timer(timeout.backend_startup)

    def __enter__(self):
        self.server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.server_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.server_socket.bind((self.host, self.port))
        self.server_socket.listen()
        self.selector.register(self.server_socket, EVENT_READ)
        (self.host, self.port) = self.server_socket.getsockname()
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self.selector.close()
        if self.server_socket:
            self.server_socket.close()
        for client in self.clients:
            client.close()
        self.done = True

    def get_uri(self):
        assert self.server_socket is not None, "get_uri called on an unopened stream"
        host, port = self.server_socket.getsockname()
        return f"tcp://{self.host}:{self.port}"

    def read_chunk(self, length: int) -> bytes:
        if self.done:
            return b""

        if self.current_shot_client is None:
            self._wait_for_current_shot_client()

        assert self.current_shot_client is not None

        if not self.current_shot_client.has_bytes(length):
            self._wait_for_current_shot_bytes(length)

        return self.current_shot_client.take(length)

    def next_shot(self):
        self.current_shot += self.shot_increment
        self.current_shot_client = None
        self._update_current_shot_client()
        self.shot_timer.reset()

    def _update_current_shot_client(self):
        """
        Get the client that should receive the given shot.
        """
        if self.current_shot_client is None:
            for client in self.clients:
                if client.configuration.provides_shot(self.current_shot):
                    self.current_shot_client = client
                    return
            self.current_shot_client = None

    def _accept_new_connection(self) -> None:
        assert self.server_socket is not None, (
            "accept_new_connection called on an unopened stream"
        )
        client_socket, address = self.server_socket.accept()
        self.selector.register(client_socket, EVENT_READ)
        registration = b""
        while len(registration) < 24:
            registration += client_socket.recv(24 - len(registration))
        shot_configuration = ClientConfiguration.unpack(registration)
        client_logfile: Path | None = None
        if self.logfile is not None:
            offset_str = str(shot_configuration.shot_offset).replace("-", "_")
            increment_str = str(shot_configuration.shot_increment).replace("-", "_")
            shot_str = str(shot_configuration.n_shots).replace("-", "_")
            client_logfile = self.logfile.with_suffix(
                f"{self.logfile.suffix}.{offset_str}.{increment_str}.{shot_str}.log"
            )
        self.clients_by_fileno[client_socket.fileno()] = len(self.clients)
        self.clients.append(
            TCPClient(client_socket, shot_configuration, client_logfile)
        )

    def _sync(self, timeout: float | None = None) -> bool:
        """
        Synchronize the TCP host, pulling in any new connections, disconnections,
        and data from all clients.
        """
        events = self.selector.select(timeout=timeout)
        for key, _ in events:
            if key.fileobj == self.server_socket:
                self._accept_new_connection()
                # if the new client provides results for the current shot,
                # update the current shot client to point to it
                self._update_current_shot_client()
            else:
                assert isinstance(key.fileobj, socket.socket)
                # we have an existing client socket that is ready to read
                client_index = self.clients_by_fileno[key.fileobj.fileno()]
                client = self.clients[client_index]
                client.sync()
                if not client.is_open:
                    self.selector.unregister(key.fileobj)
        return len(events) > 0

    def _timer_expiry_str(self) -> str:
        expired_names = []
        for name, timer in (
            ("backend_startup", self.connect_timer),
            ("per_shot", self.shot_timer),
            ("per_result", self.read_timer),
            ("overall", self.overall_timer),
        ):
            if timer.has_expired():
                expired_names.append(f"'{name}'")

        if expired_names:
            return f"Expired timers: {', '.join(expired_names)}"
        else:
            return "No expired timers"

    def _wait_for_current_shot_client(self):
        """
        Wait for a client to connect that provides results for the current shot.
        Typically this should only hit the first shot but on very quick simulations
        we could see multiple shots being processed before the final client connects.

        If a backend_startup timeout is set, use it to limit the wait time,
        such that this function will raise an Exception if the timeout is exceeded.

        If no backend_startup timeout is set, this function will block indefinitely
        until a client connects that provides results for the current shot.
        """

        self.connect_timer.reset()

        while self.current_shot_client is None:
            # Poll for new connections, passively accepting data through existing
            # clients to prevent their processes from blocking in the case of a
            # long startup time of the current shot's backend.
            timeout = Timer.min_remaining_seconds(
                [self.connect_timer, self.shot_timer, self.overall_timer]
            )
            if timeout is not None and timeout <= 0:
                # we have already exceeded the timeout of one of the timers.
                break
            self._sync(timeout=timeout)

        if self.current_shot_client is None:
            raise SeleneStartupError(
                f"Timed out waiting for a client to connect for shot {self.current_shot}: {self._timer_expiry_str()}",
                "",  # stdout is injected when this exception is caught
                "",  # stderr is injected when this exception is caught
            )

    def _wait_for_current_shot_bytes(self, length: int):
        """
        Wait for the current shot client to have at least `length` bytes of data
        to read from.

        If a per_result timeout is set, use it to limit the wait time.
        """
        assert self.current_shot_client is not None

        self.read_timer.reset()
        while (
            self.current_shot_client.is_open
            and not self.current_shot_client.has_bytes(length)
        ):
            timeout = Timer.min_remaining_seconds(
                [
                    self.read_timer,
                    self.shot_timer,
                    self.overall_timer,
                ]
            )
            if timeout is not None and timeout <= 0:
                # we have already exceeded the timeout of one of the timers.
                break
            self._sync(timeout=timeout)

        if self.current_shot_client.is_open and not self.current_shot_client.has_bytes(
            length
        ):
            raise SeleneTimeoutError(
                f"Timed out waiting for shot results: {self._timer_expiry_str()}",
                "",  # stdout is injected when this exception is caught
                "",  # stderr is injected when this exception is caught
            )


class FileStream(DataStream):
    def __init__(self, filename: Path, verbose: bool = False):
        self.handle = filename.open("rb")
        self.done = False

    def __del__(self):
        try:
            self.handle.close()
        except Exception:
            pass

    def try_read(self, length: int) -> bytes:
        result = self.handle.read(length)
        return result

    def read_chunk(self, length: int) -> bytes:
        result = self.try_read(length)
        if len(result) < length:
            self.done = True
        return result

    def next_shot(self):
        pass
