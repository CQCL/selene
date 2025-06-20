import platform
from dataclasses import dataclass
from pathlib import Path

from selene_core import ErrorModel


@dataclass
class ExampleErrorModel(ErrorModel):
    """
    A plugin for simulating an example error model.
    """

    flip_probability: float = 0.0
    angle_mutation: float = 0.0

    def get_init_args(self):
        return [
            f"--flip_probability={self.flip_probability}",
            f"--angle_mutation={self.angle_mutation}",
        ]

    @property
    def library_file(self):
        libdir = Path(__file__).parent / "_dist/lib/"
        match platform.system():
            case "Linux":
                return libdir / "libselene_error_model_example.so"
            case "Darwin":
                return libdir / "libselene_error_model_example.dylib"
            case "Windows":
                return libdir / "selene_error_model_example.dll"
            case _:
                raise RuntimeError(f"Unsupported platform: {platform.system()}")
