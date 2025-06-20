import platform
from dataclasses import dataclass
from pathlib import Path

from selene_core import Simulator


@dataclass
class StimPlugin(Simulator):
    """
    A plugin for using Stim, the stabilizer simulator, as the backend simulator for selene.

    As Stim is a stabilizer simulator, it can only simulate Clifford operations. We provide
    an angle threshold parameter for users to decide how far angles can be away from pi/2 rotations
    on the bloch sphere before they are considered invalid. This is to avoid numerical instability,
    or to inject approximations.

    Attributes:
        angle_threshold (float, default 1e-4): The angle threshold for valid rotations. Must be
            greater than zero, as floating point errors can cause numerical instability.
    """

    angle_threshold: float = 1e-4

    def __post_init__(self):
        assert self.angle_threshold > 0, (
            "angle_threshold must be greater than zero to avoid numerical instability"
        )

    def get_init_args(self):
        return [
            f"--angle-threshold={self.angle_threshold}",
        ]

    @property
    def library_file(self):
        libdir = Path(__file__).parent / "_dist/lib/"
        match platform.system():
            case "Linux":
                return libdir / "libselene_stim_plugin.so"
            case "Darwin":
                return libdir / "libselene_stim_plugin.dylib"
            case "Windows":
                return libdir / "selene_stim_plugin.dll"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")
