from abc import ABC, abstractmethod
from dataclasses import dataclass
from pathlib import Path


@dataclass
class SeleneComponent(ABC):
    """
    An abstract base class for plugins.

    We assert that every plugin may be provided with a random seed, and any
    random behaviour should be reproducible given the same seed.
    """

    random_seed: int | None = None

    @property
    @abstractmethod
    def library_file(self) -> Path:
        """
        Plugins are made possible by implementing a dynamic library that
        provides the necessary functions for the simulator to interact with
        the plugin.
        This method should provide the path to the shared object file to be
        processed by selene.
        """
        pass

    @property
    def library_search_dirs(self) -> list[Path]:
        """
        Some plugins may require additional dynamic libraries to be made
        available, away from the standard system paths. Provide their paths
        through this function if necessary. The default implementation will
        return an empty list.

        Each path provided by this property will be added to the environment that
        the selene emulator runs in when this plugin is used. This is done by
        appending to PATH on Windows, LD_LIBRARY_PATH on Linux, and
        DYLD_LIBRARY_PATH on MacOS.
        """
        return []

    def get_init_args(self) -> list[str]:
        """
        This function should return a list of strings that will be passed
        to the plugin through those *additional* arguments. They should be
        in the format expected by the plugin; if it demands
        --long=form --arguments, this function should return
        `["--long=form", "--arguments"]`. If it demands short flags, this
        function should return e.g. `["-sho", "-rt", "form args"]`.
        """
        return []
