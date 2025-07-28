import platform
from dataclasses import dataclass
from pathlib import Path

from selene_core import ErrorModel


@dataclass
class SimpleLeakagePlugin(ErrorModel):
    """
    A plugin for simulating a simple leakage model. On any 1qb gate, a qubit
    may leak to an auxiliary state with a probability of `p_leak`. On any 2qb
    gate, each qubit may leak to an auxiliary state with a probability of `p_leak`.
    If either qubit in a 2qb gate leaks, the other qubit is then leaked.
    Attributes:
        p_leak (float): The probability of leakage after any operation. Must be
            between 0 and 1 (inclusive).
        leak_measurement_bias (float): The probability of measuring a leaked qubit
            as 1.
    """

    p_leak: float = 0.0
    leak_measurement_bias: float = 0.0

    def __post_init__(self):
        assert 0 <= self.p_leak <= 1, (
            f"error_probability for p_leak ({self.p_leak}) must be between 0 and 1 (both inclusive)"
        )
        assert 0 <= self.leak_measurement_bias <= 1, (
            f"error_probability for leak_measurement_bias ({self.leak_measurement_bias}) must be between 0 and 1 (both inclusive)"
        )

    @property
    def library_file(self):
        libdir = Path(__file__).parent / "_dist/lib/"
        match platform.system():
            case "Linux":
                return libdir / "libselene_simple_leakage_plugin.so"
            case "Darwin":
                return libdir / "libselene_simple_leakage_plugin.dylib"
            case "Windows":
                return libdir / "selene_simple_leakage_plugin.dll"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")

    def get_init_args(self):
        return [
            f"--p-leak={self.p_leak}",
            f"--leak-measurement-bias={self.leak_measurement_bias}",
        ]
