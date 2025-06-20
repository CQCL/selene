import platform
from dataclasses import dataclass
from pathlib import Path

from selene_core import Runtime


@dataclass
class SimpleRuntimePlugin(Runtime):
    """
    A plugin for running a simple runtime in selene.

    It is a lazy runtime, so when the user program requests a gate to be performed, it is
    not performed immediately, but is stored in a queue. Upon the request for a measurement
    result, operations before and including the measurement are performed in order to
    retrieve the result.
    """

    @property
    def library_file(self):
        libdir = Path(__file__).parent / "_dist/lib/"
        match platform.system():
            case "Linux":
                return libdir / "libselene_simple_runtime.so"
            case "Darwin":
                return libdir / "libselene_simple_runtime.dylib"
            case "Windows":
                return libdir / "selene_simple_runtime.dll"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")

    def get_init_args(self):
        """
        There are no init args for the simple runtime.
        """
        return []
