"""`selene_sim` is a Python package that provides emulation
of hybrid quantum circuits.
"""

from .build import BuildMethod, BitcodeString, build
from .instance import SeleneInstance
from .backends import *  # noqa: F403
from .event_hooks import *  # noqa: F403

from .backends import __all__ as backends
from .event_hooks import __all__ as event_hooks

import pathlib

dist_dir = pathlib.Path(__file__).parent / "_dist"

__all__ = (
    ["BuildMethod", "BitcodeString", "build", "SeleneInstance"] + backends + event_hooks
)
