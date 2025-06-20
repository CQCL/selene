import platform
from dataclasses import dataclass
from pathlib import Path

from selene_core import Simulator


@dataclass
class ExampleSimulator(Simulator):
    """
    A plugin for using an example simulator in selene.
    Attributes:
        bias (float): The bias of the coinflip simulator. Must be between 0 and 1 (both inclusive).
                      The greater this value, the more likely a measurement will return True.
    """

    bias: float = 0.5

    def __post_init__(self):
        assert 0 <= self.bias <= 1, "bias must be between 0 and 1 (both inclusive)"

    def get_init_args(self):
        return [
            f"--bias={self.bias}",
        ]

    @property
    def library_file(self):
        libdir = Path(__file__).parent / "_dist/lib/"
        match platform.system():
            case "Linux":
                return libdir / "libselene_example_simulator_plugin.so"
            case "Darwin":
                return libdir / "libselene_example_simulator_plugin.dylib"
            case "Windows":
                return libdir / "selene_example_simulator_plugin.dll"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")
