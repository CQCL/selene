import platform
from dataclasses import dataclass
from pathlib import Path

from selene_core import Simulator


@dataclass
class CoinflipPlugin(Simulator):
    """
    A plugin for using a coinflip simulator in selene. No quantum simulation is performed,
    and all measurements return a random boolean. A bias can be set (default 0.5) to change
    the probability distribution: a bias of x% means that x% of the time the result will be True.

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
                return libdir / "libselene_coinflip_plugin.so"
            case "Darwin":
                return libdir / "libselene_coinflip_plugin.dylib"
            case "Windows":
                return libdir / "selene_coinflip_plugin.dll"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")
