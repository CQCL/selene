import platform
from dataclasses import dataclass
from pathlib import Path

from selene_core import ErrorModel


@dataclass
class DepolarizingPlugin(ErrorModel):
    """
    A plugin for simulating depolarizing error models in quantum systems.
    This class represents a depolarizing error model with configurable error
    probabilities for single-qubit gates, two-qubit gates, measurement errors,
    and initialization errors. It ensures that all error probabilities are
    within the valid range of [0, 1].
    Attributes:
        p_1q (float): The error probability for single-qubit gates. Must be
            between 0 and 1 (inclusive).
        p_2q (float): The error probability for two-qubit gates. Must be
            between 0 and 1 (inclusive).
        p_meas (float): The error probability for measurement operations. Must
            be between 0 and 1 (inclusive).
        p_init (float): The error probability for initialization operations.
            Must be between 0 and 1 (inclusive).
    """

    p_1q: float = 0.0
    p_2q: float = 0.0
    p_meas: float = 0.0
    p_init: float = 0.0

    def __post_init__(self):
        assert 0 <= self.p_1q <= 1, (
            f"error_probability for p_1q ({self.p_q1}) must be between 0 and 1 (both inclusive)"
        )
        assert 0 <= self.p_2q <= 1, (
            f"error_probability for p_2q ({self.p_q2}) must be between 0 and 1 (both inclusive)"
        )
        assert 0 <= self.p_meas <= 1, (
            f"error_probability for p_meas ({self.p_meas}) must be between 0 and 1 (both inclusive)"
        )
        assert 0 <= self.p_init <= 1, (
            f"error_probability for p_init ({self.p_init}) must be between 0 and 1 (both inclusive)"
        )

    @property
    def library_file(self):
        libdir = Path(__file__).parent / "_dist/lib/"
        match platform.system():
            case "Linux":
                return libdir / "libselene_depolarizing_plugin.so"
            case "Darwin":
                return libdir / "libselene_depolarizing_plugin.dylib"
            case "Windows":
                return libdir / "selene_depolarizing_plugin.dll"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")

    def get_init_args(self):
        return [
            f"--p-1q={self.p_1q}",
            f"--p-2q={self.p_2q}",
            f"--p-meas={self.p_meas}",
            f"--p-init={self.p_init}",
        ]
