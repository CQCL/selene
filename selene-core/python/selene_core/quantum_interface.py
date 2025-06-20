from abc import ABC, abstractmethod
from dataclasses import dataclass
from pathlib import Path
from .build_utils import BuildPlanner


@dataclass
class QuantumInterface(ABC):
    """
    An abstract base class for quantum interfaces.

    This class provides a blueprint for implementing quantum interfaces that
    interact with various quantum hardware or simulators. It ensures that all
    derived classes implement the necessary methods to retrieve the path to
    the shared object file and any additional library paths required.
    """

    @property
    @abstractmethod
    def library_file(self) -> Path:
        """
        Returns the path to the object file for the quantum interface. It
        may be a shared object, static object, or any other file type. The
        [build planner](/selene-sim/python/selene_sim/builder/build_planner.py)
        should be provided the information on how to use this file to map
        a user program to the selene interface.
        """
        pass

    @property
    def link_flags(self) -> list[str]:
        """
        Returns the flags to be used when linking the interface against the selene
        executable, if any.
        """
        return []

    @property
    def library_search_dirs(self) -> list[Path]:
        """
        Returns a list of additional library paths required by this interface.
        These paths will be provided in the environment when the emulator
        is run, through PATH on Windows, LD_LIBRARY_PATH on Linux, or
        DYLD_LIBRARY_PATH on MacOS.
        """
        return []

    def register_build_steps(self, planner: BuildPlanner):
        """
        Registers the build steps for this interface with the provided
        [build planner](/selene-sim/python/selene_sim/builder/build_planner.py).
        This method should be overridden by subclasses to add specific
        build steps.
        """
        pass
