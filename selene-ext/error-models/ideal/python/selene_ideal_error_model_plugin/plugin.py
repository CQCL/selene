import platform
from dataclasses import dataclass
from pathlib import Path

from selene_core import ErrorModel


@dataclass
class IdealPlugin(ErrorModel):
    """
    A plugin for simulating an ideal error model in quantum systems. All
    operations provided by the runtime will be executed as-is, without any
    errors. There are no configure options for this plugin.
    """

    def get_init_args(self):
        return []

    @property
    def library_file(self):
        libdir = Path(__file__).parent / "_dist/lib/"
        match platform.system():
            case "Linux":
                return libdir / "libselene_ideal_plugin.so"
            case "Darwin":
                return libdir / "libselene_ideal_plugin.dylib"
            case "Windows":
                return libdir / "selene_ideal_plugin.dll"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")
