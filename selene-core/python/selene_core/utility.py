from abc import ABC, abstractmethod
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Utility(ABC):
    """
    An abstract base class for link-time "utility" plugins.

    Simulator plugins' python API should provide a specialisation of this class
    that provides the path to files which should be linked in with the selene executable
    """

    @property
    @abstractmethod
    def library_file(self) -> Path:
        """
        Utilities expose symbols that user programs can call into directly,
        rather than via the Selene interface itself. They are implemented as
        a compiled library, which you should provide through this property.
        """
        pass

    @property
    def link_flags(self) -> list[str]:
        """
        Returns the flags to be used when linking the plugin against the selene
        executable, if any. This is likely to include rpath entries, but may also
        include other flags.
        """
        return []

    @property
    def library_search_dirs(self) -> list[Path]:
        """
        Returns the paths to any additional libraries required by the plugin.
        """
        return []
