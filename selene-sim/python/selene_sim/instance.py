import os
import platform
import shutil
from pathlib import Path
from typing import Iterator
from subprocess import Popen
from dataclasses import dataclass
import yaml


from selene_core import SeleneComponent, Simulator, ErrorModel, Runtime

from .backends import SimpleRuntime, IdealErrorModel
from .result_handling import TaggedResult
from .event_hooks import EventHook, NoEventHook
from .exceptions import SeleneRuntimeError
from .result_handling import ResultStream, TCPStream, parse_shot
from .timeout import Timeout, TimeoutInput


@dataclass
class ShotSpec:
    """
    Describes how a simulation run should progress through shot numbers.
    This can be customised to interleave multiple runs.
    """

    count: int
    offset: int = 0
    increment: int = 1

    def __contains__(self, shot: int) -> bool:
        delta = shot - self.offset
        if delta % self.increment != 0:
            return False
        return 0 <= (delta // self.increment) < self.count


yaml.SafeDumper.add_representer(
    ShotSpec,
    lambda dumper, data: dumper.represent_mapping(
        "tag:yaml.org,2002:map",
        {
            "count": data.count,
            "offset": data.offset,
            "increment": data.increment,
        },
    ),
)


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

    def wait(self):
        """
        Wait for the process to finish and return the return code.
        """
        if self.process is None:
            raise SeleneRuntimeError(
                message="Process has not been spawned yet", stdout="", stderr=""
            )
        self.process.wait()
        if self.process.returncode:
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

    def wait(self):
        for process in self.processes:
            process.wait()


@dataclass
class SeleneInstance:
    """
    Represents a selene instance, which is a wrapper around the selene executable
    generated from a user program by build.py's `build` function.
    """

    root: Path
    artifacts: Path
    runs: Path
    executable: Path
    library_search_dirs: list[Path]

    def __post_init__(self):
        """
        Setup the run directory ready for invocation.
        """
        self.artifacts.mkdir(parents=True, exist_ok=True)
        self.runs.mkdir(exist_ok=True)

    def write_manifest(self, data: dict):
        """
        Writes the build manifest corresponding to this instance
        for future reference.
        """
        with open(self.root / "selene.yaml", "w") as fh:
            yaml.safe_dump(data, fh)

    @staticmethod
    def _get_component_config(
        component: SeleneComponent, default_seed: int | None
    ) -> dict:
        """
        Helper method to generate a dictionary form of the configuration
        required for a selene component (e.g. runtime, error model, simulator).

        This is later written to a yaml file ready for selene loading.
        """
        full_python_name = ".".join(
            (component.__class__.__module__, component.__class__.__qualname__)
        )
        result = {
            "name": full_python_name,
            "file": component.library_file,
            "args": component.get_init_args(),
        }
        if component.random_seed is not None:
            result["seed"] = component.random_seed
        elif default_seed is not None:
            result["seed"] = default_seed
        return result

    def _create_new_run_directory(self) -> Path:
        """
        Creates and returns the run directory for the next user program
        invocation.
        """
        i = 1
        while (self.runs / f"{i:04d}").exists():
            i += 1
        run_dir = self.runs / f"{i:04d}"
        run_dir.mkdir()
        return run_dir

    def delete_files(self):
        """
        Deletes the files associated with this instance, including the
        executable, artifacts, and run directories.
        """
        shutil.rmtree(self.root)

    def delete_run_directories(self):
        """
        Deletes the run directories associated with this instance.
        This does not delete the artifacts directory or the executable.
        """
        for run_dir in self.runs.iterdir():
            if run_dir.is_dir():
                shutil.rmtree(run_dir)

    def _check_health(self):
        """
        Checks the health of the instance, ensuring that the root directory
        and executable exist. If they do not, raises a FileNotFoundError.

        If the runs directory does not exist, it is created.
        """
        if not self.root.exists():
            raise FileNotFoundError(
                f"This Selene instance's root directory does not exist ({self.root}). "
                "This is likely because the instance has been deleted. Please rebuild."
            )
        if not self.executable.exists():
            raise FileNotFoundError(
                f"This Selene instance's executable does not exist ({self.root}). "
                f"This is likely because the file has been deleted manually. Please rebuild."
            )

        if not self.runs.exists():
            print("Warning: The runs directory does not exist. Creating it again.")
            self.runs.mkdir(parents=True, exist_ok=True)

    def run_shots(
        self,
        simulator: Simulator,
        n_qubits: int,
        n_shots: int = 1,
        error_model: ErrorModel = IdealErrorModel(),
        runtime: Runtime = SimpleRuntime(),
        event_hook: EventHook = NoEventHook(),
        verbose: bool = False,
        timeout: TimeoutInput = None,
        results_logfile: Path | None = None,
        random_seed: int | None = None,
        shot_offset: int = 0,
        shot_increment: int = 1,
        n_processes: int = 1,
        parse_results: bool = True,
    ) -> Iterator[Iterator[TaggedResult]]:
        """
        Run the compiled program through multiple selene shots.
        Args:
            simulator: The simulator plugin to use
            n_qubits: The maximum number of qubits to simulate
            n_shots: The number of shots to run
            error_model: The error model plugin to use (if any)
            runtime: The runtime plugin to use
            event_hook: Event hook that configures Selene to output
                        additional information to the results stream,
                        and handles the output.
            verbose: Whether to print verbose output for diagnostics
            timeout: Timeout configuration for various aspects of the
                     emulation run. If a timedelta is provided directly,
                     it is assumed to be the time limit of the overall
                     run. If None is provided, no timeout is set. If
                     a float is provided, it is assumed to be the time limit
                     in seconds of the overall run.
            results_logfile: The file to write the results to (if any)
            random_seed: The random seed to use for the simulator, error model,
                         and runtime if they have not been set explicitly. On
                         each shot, the random seed will be incremented by 1.
            parse_results:
                Whether to interpret tags in the result stream.
                If True (default), tags will be stripped, interpreted,
                and routed according to their type, e.g:
                - ("USER:INT:example", 42) will be interpreted as
                    an output variable "example"
                - ("METRICS:INT:user_program:qalloc_count", 5) will
                    be interpreted as a metric with the name "qalloc_count"
                    in the "user_program" namespace, and given to a MetricStore
                    event hook WITHOUT yielding it as a result.
                If False, no stripping or routing will be done: everything
                in the result stream is passed directly to the result iterator,
                e.g:
                - ("USER:INT:example", 42) will be yielded as-is,
                  without interpretation.
                - ("METRICS:INT:user_program:qalloc_count", 5) will
                  also be yielded as-is, without interpretation.
                Setting to True provides the high level Selene interface, and
                using False allows for Selene to be used as an intermediate
                component for use with an external result stream handler.
        """

        self._check_health()
        n_processes = max(1, n_processes)
        n_processes = min(n_processes, n_shots)

        timeout = Timeout.resolve_input(timeout)

        processes = SeleneProcessList()
        library_search_dirs = self.library_search_dirs.copy()
        for component in (simulator, error_model, runtime):
            library_search_dirs.extend(component.library_search_dirs)
        global_configuration = {
            "event_hooks": {flag: True for flag in event_hook.get_selene_flags()},
            "n_qubits": n_qubits,
            "simulator": self._get_component_config(simulator, random_seed),
            "error_model": self._get_component_config(error_model, random_seed),
            "runtime": self._get_component_config(runtime, random_seed),
        }
        with TCPStream(
            timeout=timeout,
            logfile=results_logfile,
            shot_offset=shot_offset,
            shot_increment=shot_increment,
        ) as data_stream:
            global_configuration["output_stream"] = data_stream.get_uri()

            for i in range(n_processes):
                shot_spec = ShotSpec(
                    offset=shot_offset + i * shot_increment,
                    increment=shot_increment * n_processes,
                    count=n_shots // n_processes + (n_shots % n_processes > i),
                )
                run_directory = self._create_new_run_directory()
                artifact_directory = run_directory / "artifacts"
                artifact_directory.mkdir(parents=True, exist_ok=True)
                configuration = global_configuration | {
                    "shots": shot_spec,
                    "artifact_dir": artifact_directory,
                }
                processes.add(
                    SeleneProcess(
                        executable=self.executable,
                        library_search_dirs=library_search_dirs,
                        configuration=configuration,
                        run_directory=run_directory,
                    )
                )

            processes.spawn()
            for i in range(n_shots):
                shot_idx = shot_offset + (i * shot_increment)
                if verbose:
                    print(f"Processing shot {shot_idx}")
                if data_stream.done:
                    raise Exception(
                        "Results stream has ended before all shots are processed"
                    )
                result_stream = ResultStream(data_stream)
                event_hook.on_new_shot()
                relevant_process = processes.find(shot_idx)
                assert relevant_process is not None
                yield parse_shot(
                    result_stream,
                    event_hook,
                    parse_results,
                    relevant_process.stdout,
                    relevant_process.stderr,
                )

        processes.wait()

    def run(
        self,
        simulator: Simulator,
        n_qubits: int,
        runtime: Runtime = SimpleRuntime(),
        error_model: ErrorModel = IdealErrorModel(),
        event_hook: EventHook = NoEventHook(),
        verbose: bool = False,
        timeout: TimeoutInput = None,
        results_logfile=None,
        random_seed: int | None = None,
        shot_offset: int = 0,
        parse_results: bool = True,
    ) -> Iterator[TaggedResult]:
        """
        Run the compiled program through a single selene shot.

        Args:
            simulator: The simulator plugin to use
            n_qubits: The maximum number of qubits to simulate
            runtime: The runtime plugin to use
            error_model: The error model plugin to use (if any)
            event_hook: Event hook that configures Selene to output
                        additional information to the results stream,
                        and handles the output.
            verbose: Whether to print verbose output for diagnostics
            timeout: The maximum time to wait for the program to complete
            results_logfile: The file to write the results to (if any)
            random_seed: The random seed to use for the simulator, error model,
                         and runtime if they have not been set explicitly
        """
        shot_generator = self.run_shots(
            simulator=simulator,
            runtime=runtime,
            error_model=error_model,
            n_qubits=n_qubits,
            n_shots=1,
            event_hook=event_hook,
            verbose=verbose,
            timeout=timeout,
            results_logfile=results_logfile,
            random_seed=random_seed,
            shot_offset=shot_offset,
            parse_results=parse_results,
        )
        # We cannot simply yield from the shot generator, as this can
        # cause lifetime issues with the run_shots generator.
        # Keep it in scope with a context manager.
        from contextlib import contextmanager

        @contextmanager
        def one_shot_context():
            shot = next(shot_generator)
            try:
                yield shot
            finally:
                try:
                    next(shot_generator)
                except StopIteration:
                    pass

        with one_shot_context() as context:
            yield from context
