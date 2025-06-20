import enum
import platform
from dataclasses import dataclass
from pathlib import Path

from selene_core import QuantumInterface, BuildPlanner

from . import build


class LogLevel(enum.Enum):
    """Enum for log levels."""

    QUIET = 0
    DEBUG = 1
    DIAGNOSTIC = 2


@dataclass
class HeliosInterface(QuantumInterface):
    log_level: LogLevel = LogLevel.QUIET

    @property
    def library_file(self):
        # two libraries are currently provided: one for normal runs,
        # and one for diagnostic runs. The latter prints quantum calls
        # to stderr, and can be selected by setting the diagnostic_mode
        # attribute to True.
        lib_name = "helios_selene_interface"
        match self.log_level:
            case LogLevel.QUIET:
                lib_name += ""
            case LogLevel.DEBUG:
                lib_name += "_debug"
            case LogLevel.DIAGNOSTIC:
                lib_name += "_diagnostic"
            case _:
                raise ValueError("Invalid log level")
        lib_dir = Path(__file__).parent / "_dist/lib/"
        match platform.system():
            case "Linux":
                return lib_dir / f"lib{lib_name}.a"
            case "Darwin":
                return lib_dir / f"lib{lib_name}.a"
            case "Windows":
                return lib_dir / f"{lib_name}.lib"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")

    def register_build_steps(self, planner: BuildPlanner):
        planner.add_step(build.SeleneCompileHUGRToLLVMIRStringStep)
        planner.add_step(build.SeleneCompileHUGRToLLVMBitcodeStringStep)
