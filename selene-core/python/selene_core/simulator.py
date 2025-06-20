from abc import abstractmethod
from dataclasses import dataclass
from pathlib import Path

from .plugin import SeleneComponent


@dataclass
class Simulator(SeleneComponent):
    """
    An abstract base class for simulator plugins.

    Simulator plugins' python API should provide a specialisation of this class
    that provides a way to set any user-settable arguments, and provides the
    path to the appropriate shared object file that gets loaded at runtime
    by selene.

    About the simulator component:

    The simulator is the core of the selene emulator. It is responsible for
    simulating the ideal behaviour of a quantum computer, and is responsible
    for implementing the quantum operations that are requested by the user
    program (after transformations via the runtime and error model). Incoming
    operations will *include* noise added by the error model.
    """

    @property
    @abstractmethod
    def library_file(self):
        """
        The simulator is provided to selene via a shared object file,
        loaded at runtime. Implement this function to provide the path
        to the shared object file for your plugin.
        """
        pass

    @property
    def library_search_dirs(self) -> list[Path]:
        """
        Some simulators may require additional dynamic libraries to be made
        available, away from the standard system paths. Provide their paths
        through this function if necessary. The default implementation will
        return an empty list.

        Each path provided by this property will be added to the environment that
        the selene emulator runs in when this simulator is used. This is done by
        appending to PATH on Windows, LD_LIBRARY_PATH on Linux, and
        DYLD_LIBRARY_PATH on MacOS, within the emulator's run environment.
        """
        return []

    @abstractmethod
    def get_init_args(self):
        """
        Error model plugins must implement a selene_error_model_init function
        which, among other parameters, takes an (argc, argv) style pair of
        generic cli-like arguments.

        This function should return a list of strings that will be passed
        to the plugin through those custom arguments. They should be
        in the format expected by the plugin; if it demands
        `--long=form --arguments`, this function should return
        `["--long=form", "--arguments"]`. If it demands short flags, this
        function should return e.g. `["-sho", "-rt", "form args"]`.

        The author should provide, in their python interface that implements
        this abstract class, a way to set these arguments. Validation should
        use __post_init__ to ensure that incoming arguments are valid.
        """
        pass
