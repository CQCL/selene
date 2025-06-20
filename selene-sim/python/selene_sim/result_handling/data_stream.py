import socket
import datetime
from pathlib import Path
from abc import ABC, abstractmethod
from typing import BinaryIO
import struct
from selectors import DefaultSelector, EVENT_READ
from dataclasses import dataclass
from selene_sim.exceptions import SeleneStartupError, SeleneRuntimeError


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
            self.logfile.close()
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
        connect_wait_limit: datetime.timedelta | None = None,
        read_wait_limit: datetime.timedelta | None = None,
        logfile: Path | None = None,
        shot_offset: int = 0,
        shot_increment: int = 1,
    ):
        self.host = host
        self.port = port
        self.connect_wait_limit = connect_wait_limit
        self.read_wait_limit = read_wait_limit
        self.done = False
        self.selector = DefaultSelector()
        self.server_socket: socket.socket | None = None
        self.clients: list[TCPClient] = []
        self.clients_by_fileno: dict[int, int] = {}
        self.logfile = logfile
        self.current_shot = shot_offset
        self.shot_increment = shot_increment
        self.current_shot_client: TCPClient | None = None

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

    def _sync(self, timeout: float = 0) -> bool:
        events = self.selector.select(timeout=timeout)
        for key, _ in events:
            if key.fileobj == self.server_socket:
                self._accept_new_connection()
                self._update_current_shot_client()
            else:
                assert isinstance(key.fileobj, socket.socket)
                client_index = self.clients_by_fileno[key.fileobj.fileno()]
                client = self.clients[client_index]
                client.sync()
                if not client.is_open:
                    self.selector.unregister(key.fileobj)
        return len(events) > 0

    def read_chunk(self, length: int) -> bytes:
        if self.done:
            return b""
        start_time = datetime.datetime.now()
        while self.current_shot_client is None:
            timeout = (
                self.connect_wait_limit.total_seconds()
                if self.connect_wait_limit
                else 0
            )
            if not self._sync(timeout=timeout) and self.connect_wait_limit is not None:
                raise SeleneStartupError(
                    f"Timed out ({self.connect_wait_limit}) waiting for a client to connect for shot {self.current_shot}",
                    "",  # stdout is injected when this exception is caught
                    "",  # stderr is injected when this exception is caught
                )
        while (
            self.current_shot_client.is_open
            and not self.current_shot_client.has_bytes(length)
        ):
            timeout = (
                self.read_wait_limit.total_seconds() if self.read_wait_limit else 0
            )
            if not self._sync(timeout=timeout) and self.read_wait_limit is not None:
                if datetime.datetime.now() - start_time > self.read_wait_limit:
                    raise SeleneRuntimeError(
                        "Timed out waiting for a client to send data",
                        "",  # stdout is injected when this exception is caught
                        "",  # stderr is injected when this exception is caught
                    )
        return self.current_shot_client.take(length)

    def next_shot(self):
        self.current_shot += self.shot_increment
        self.current_shot_client = None
        self._update_current_shot_client()


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
