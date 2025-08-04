import base64
import platform
from dataclasses import dataclass, field
from pathlib import Path

from selene_core import Simulator


def encode_booleans(bool_list: list[bool]) -> str:
    """
    Encodes a list of booleans as a compact base64 string.
    Each boolean is stored as a single bit.
    """
    # Determine the number of bytes needed (each byte holds 8 bits)
    nbytes = (len(bool_list) + 7) // 8
    byte_array = bytearray(nbytes)

    for i, flag in enumerate(bool_list):
        if flag:
            # Set the bit in the appropriate byte (most significant bit first)
            byte_array[i // 8] |= 1 << (7 - (i % 8))

    # Convert the bytes to a base64-encoded string
    return base64.b64encode(byte_array).decode("ascii")


def encode_counts(counts: list[int]) -> str:
    """
    Encodes a list of integers as a compact base64 string.
    Each integer is stored as a 32-bit little-endian integer.
    """
    # Convert the integers to bytes
    byte_array = bytearray()
    for count in counts:
        byte_array.extend(count.to_bytes(4, "little"))

    # Convert the bytes to a base64-encoded string
    return base64.b64encode(byte_array).decode("ascii")


@dataclass
class QuantumReplayPlugin(Simulator):
    """
    A plugin for running a quantum replay simulator in selene. This simulator
    allows a user to predefine the results of measurements for each shot,
    performing postselection on an underlying simulator. A user may choose
    to continue running the simulator when provided measurements are exhausted.

    Attributes:
        simulator (Simulator): The underlying simulator to be used for quantum operations and postselection.
        resume_with_measurement (bool, True by default): If True, the simulator will continue running when the provided measurements are exhausted, using measurement instead of postselection.
        measurements (list[list[bool]): A list of lists of booleans, where each inner
            list represents the boolean measurement results for a single shot.
    """

    simulator: Simulator | None = None
    resume_with_measurement: bool = True
    measurements: list[list[bool]] = field(default_factory=list[list[bool]])

    def __post_init__(self):
        assert self.simulator is not None, (
            "A simulator must be provided to the quantum replay plugin"
        )
        self.random_seed = self.simulator.random_seed
        for shot, shot_measurements in enumerate(self.measurements):
            if not isinstance(shot_measurements, list):
                raise ValueError(
                    "Measurements provided to the quantum replay simulator must be in the form of a list of measurements per shot, i.e. a list of lists"
                )
            for i, measurement in enumerate(shot_measurements):
                match measurement:
                    case bool():
                        continue
                    case int() if measurement in (0, 1):
                        self.measurements[shot][i] = bool(measurement)
                    case _:
                        raise ValueError(
                            f"All measurements provided to the quantum replay simulator must be boolean (True or False) or binary integer (0 or 1). Erroneous value {measurement} of type {type(measurement)} found in shot {shot} at index {i}"
                        )

    def get_init_args(self):
        all_measurements = []
        counts = []
        for shot_measurements in self.measurements:
            counts.append(len(shot_measurements))
            all_measurements.extend(shot_measurements)
        encoded_counts = encode_counts(
            [len(shot_measurements) for shot_measurements in self.measurements]
        )
        encoded_measurements = encode_booleans(all_measurements)
        return (
            [
                f"--counts={encoded_counts}",
                f"--measurements={encoded_measurements}",
                f"--wrapped-path={self.simulator.library_file}",
            ]
            + [f"--wrapped-arg={arg}" for arg in self.simulator.get_init_args()]
            + (["--resume-with-measurement"] if self.resume_with_measurement else [])
        )

    @property
    def library_file(self):
        libdir = Path(__file__).parent / "_dist/lib/"
        match platform.system():
            case "Linux":
                return libdir / "libselene_quantum_replay.so"
            case "Darwin":
                return libdir / "libselene_quantum_replay.dylib"
            case "Windows":
                return libdir / "selene_quantum_replay.dll"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")

    @property
    def library_search_dirs(self):
        return self.simulator.library_search_dirs
