from abc import abstractmethod
from dataclasses import dataclass
from pathlib import Path

from .plugin import SeleneComponent


@dataclass
class Runtime(SeleneComponent):
    """
    An abstract base class for runtime plugins.

    Runtime plugins' python API should provide a specialisation of this class
    that provides a way to set any user-settable arguments, and provides the
    path to the appropriate shared object file that gets loaded at runtime
    by selene.

    About the runtime component:

    A quantum runtime acts as the classical controller of a hybrid quantum computer.
    It accepts quantum operations from the user through function calls, and is
    responsible for scheduling operations effectively before being sent, in batches
    to a quantum device.

    It may act lazily for many operations, e.g. queueing up pure quantum gates,
    waiting for a measurement value to be requested by the user program (for result
    reporting or conditional branching) before deciding how to schedule the queue
    of gates. It may perform optimizations such as squashing, rebasing, or eliding
    gates, and it may do this on-the-fly as operations arrive, or during the flushing
    stage.

    In the specific context of ion trap quantum computers, there are other things
    that a runtime may wish to perform, such as routing ions, deciding on optimal
    order of gates to minimise transport or error rates, and so on. While Selene
    itself does not understand such commands, it provides a message channel from
    the runtime to the error model through an opaque "custom operation" type. Through
    this mechanism, a runtime designed for specific hardware can 'talk' to an error
    model designed for the same hardware, allowing, for example:
    - noise to accumulated through transport
    - noise to be dependent on relative locations of qubits
    - specific timing information to be taken into account
    """

    @property
    @abstractmethod
    def library_file(self) -> Path:
        """
        The quantum runtime is provided to selene via a shared object file,
        loaded at runtime. Implement this function to provide the path
        to the shared object file for your plugin.
        """
        pass

    @property
    def library_search_dirs(self) -> list[Path]:
        """
        Some runtimes may require additional dynamic libraries to be made
        available, away from the standard system paths. Provide their paths
        through this function if necessary. The default implementation will
        return an empty list.

        Each path provided by this property will be added to the environment that
        the selene emulator runs in when this runtime is used. This is done by
        appending to PATH on Windows, LD_LIBRARY_PATH on Linux, and
        DYLD_LIBRARY_PATH on MacOS, within the emulator's run environment.
        """
        return []

    @abstractmethod
    def get_init_args(self) -> list[str]:
        """
        Runtime plugins must implement a selene_runtime_init function
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
