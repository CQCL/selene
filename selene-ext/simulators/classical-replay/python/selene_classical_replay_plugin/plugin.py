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
class ClassicalReplayPlugin(Simulator):
    """
    A plugin for running a classical replay simulator in selene. This simulator
    allows a user to predefine the results of measurements for each shot. No
    quantum operations are performed.

    There are three main usecases of the ClassicalReplayPlugin:
    1. To debug the classical user program without running the quantum simulator.
    2. To perform tests of user code with predetermined results
    3. To validate control flow by ensuring that a set of measurements can, or can
       not, have been made within a complete shot. E.g. consider a user program that
       halts on the first measurement result of |1>. This simulator will raise an error
       if the supplied measurements do not contain a |1> result, or contain results following
       a |1> result.

    Attributes:
        measurements (list[list[bool]): A list of lists of booleans, where each inner
            list represents the boolean measurement results for a single shot.
    """

    measurements: list[list[bool]] = field(default_factory=list[list[bool]])

    def __post_init__(self):
        for shot_measurements in self.measurements:
            if not isinstance(shot_measurements, list):
                raise ValueError(
                    "Measurements provided to the classical replay simulator must be in the form of a list of measurements per shot, i.e. a list of lists"
                )
            for measurement in shot_measurements:
                if not isinstance(measurement, bool):
                    raise ValueError(
                        "All measurements provided to the classical replay simulator must be boolean"
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
        return [f"--counts={encoded_counts}", f"--measurements={encoded_measurements}"]

    @property
    def library_file(self):
        libdir = Path(__file__).parent / "_dist/lib/"
        match platform.system():
            case "Linux":
                return libdir / "libselene_classical_replay.so"
            case "Darwin":
                return libdir / "libselene_classical_replay.dylib"
            case "Windows":
                return libdir / "selene_classical_replay.dll"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")
