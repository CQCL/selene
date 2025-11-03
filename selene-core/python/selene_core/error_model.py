from abc import abstractmethod
from dataclasses import dataclass
from pathlib import Path

from .plugin import SeleneComponent


@dataclass
class ErrorModel(SeleneComponent):
    """
    An abstract base class for error model plugins.

    Error model plugins' python API should provide a specialisation of this class
    that provides a way to set any user-settable arguments, and provides the
    path to the appropriate shared object file that gets loaded at runtime
    by selene.

    About the error model component:

    An error model is a component of the selene emulator that is responsible
    for simulating the effects of noise and errors that may occur on quantum
    computers. It does this by accepting batches of ideal operations from
    the [runtime](runtime.py) and transforming them into noisy operations,
    either by modifying gate parameters, adding additional operations (e.g.
    an X gate to simulate a bit flip), and removing operations.

    The error model has full access to the underlying simulator, so it has
    the ability to perform measurements, reset qubits, and so on, regardless
    of whether or not such operations were requested from the runtime.

    Except in specific cases, the simulator should be 'ideal', in the sense
    that it does exactly what it is told to do without introducing quantum
    noise of its own. Reality should be taken into account within the error model,
    unless there is specific benefit of doing it within the simulator itself.
    """

    @property
    @abstractmethod
    def library_file(self) -> Path:
        """
        The error model is provided to selene via a shared object file,
        loaded at runtime. Implement this function to provide the path
        to the shared object file for your plugin.
        """
        pass

    @property
    def library_search_dirs(self) -> list[Path]:
        """
        Some error models may require additional dynamic libraries to be made
        available, away from the standard system paths. Provide their paths
        through this function if necessary. The default implementation will
        return an empty list.

        Each path provided by this property will be added to the environment that
        the selene emulator runs in when this error model is used. This is done by
        appending to PATH on Windows, LD_LIBRARY_PATH on Linux, and
        DYLD_LIBRARY_PATH on MacOS, within the emulator's run environment.
        """
        return []

    @abstractmethod
    def get_init_args(self) -> list[str]:
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
        return []
