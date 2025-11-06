import os
import platform
from pathlib import Path
from subprocess import Popen, TimeoutExpired
import yaml


from .exceptions import SeleneRuntimeError


class SeleneProcess:
    """
    Represents a single selene process, e.g. a single run of the selene
    executable. It is responsible for managing the environment that the
    process is invoked within, providing configuration, spawning the process
    and waiting for it to finish.
    """

    configuration: dict
    run_directory: Path
    library_search_dirs: list[Path]
    stdout: Path
    stderr: Path
    process: Popen | None

    def __init__(
        self,
        executable: Path,
        library_search_dirs: list[Path],
        run_directory: Path,
        configuration: dict,
    ):
        self.executable = executable
        self.library_search_dirs = library_search_dirs
        self.configuration = configuration
        self.run_directory = run_directory
        (self.run_directory / "configuration.yaml").write_text(
            yaml.safe_dump(configuration)
        )
        self.stdout = run_directory / "stdout.txt"
        self.stderr = run_directory / "stderr.txt"

    def get_environment(self) -> dict:
        """
        Get the environment variables for the process.
        """
        env = os.environ.copy()
        additional_dirs = os.pathsep.join(str(p) for p in self.library_search_dirs)
        path_name = ""
        match platform.system():
            case "Linux":
                path_name = "LD_LIBRARY_PATH"
            case "Darwin":
                path_name = "DYLD_LIBRARY_PATH"
            case "Windows":
                path_name = "PATH"
        if path_name in env:
            env[path_name] += os.pathsep + additional_dirs
        else:
            env[path_name] = additional_dirs
        return env

    def __del__(self):
        self.terminate()

    def terminate(self, expected_natural_exit: bool = False):
        """
        Terminate the process.

        If expected_natural_exit is True, wait briefly (1 second) for the process to
        exit naturally before forcing termination. This is useful in scenarios
        where the process is expected to exit on its own, but we want to ensure it
        does not linger.
        """
        if self.process is None:
            return
        if expected_natural_exit:
            try:
                self.process.wait(timeout=1)
                return
            except TimeoutExpired:
                pass
        self.process.terminate()
        try:
            self.process.wait(timeout=2)
        except TimeoutExpired:
            self.process.kill()

    def spawn(self):
        """
        Spawn the process and return the Popen object.
        """
        argv = [
            str(self.executable),
            "--configuration",
            str(self.run_directory / "configuration.yaml"),
        ]
        self.process = Popen(
            argv,
            stdout=open(self.stdout, "w"),
            stderr=open(self.stderr, "w"),
            env=self.get_environment(),
        )
        return self.process

    def wait(self, check_return_code: bool = True):
        """
        Wait for the process to finish and return the return code.
        """
        if self.process is None:
            raise SeleneRuntimeError(
                message="Process has not been spawned yet", stdout="", stderr=""
            )
        self.process.wait()
        if check_return_code and self.process.returncode:
            raise SeleneRuntimeError(
                message="Error running user program",
                stdout=self.stdout.read_text(),
                stderr=self.stderr.read_text(),
            )


class SeleneProcessList:
    """
    Manages a list of processes, allowing them to be spawned and waited
    on in a batch-like manner.
    """

    processes: list[SeleneProcess]

    def __init__(self):
        self.processes = []

    def add(self, process: SeleneProcess):
        self.processes.append(process)

    def spawn(self):
        for process in self.processes:
            process.spawn()

    def find(self, shot: int) -> SeleneProcess | None:
        for process in self.processes:
            if shot in process.configuration["shots"]:
                return process
        return None

    def wait(self, check_return_code: bool = True):
        for process in self.processes:
            process.wait(check_return_code)
