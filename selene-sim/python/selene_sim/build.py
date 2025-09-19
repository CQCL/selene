from pathlib import Path
from typing import Iterable, Sequence, Any, TYPE_CHECKING
import logging
import tempfile
import random
import string
import os
from enum import Enum

from selene_core import QuantumInterface, Utility
from selene_core.build_utils import (
    BuildPlanner,
    Artifact,
    LibDep,
    BuildCtx,
    DEFAULT_BUILD_PLANNER,
)
from selene_core.build_utils.builtins import SeleneExecutableKind

from .instance import SeleneInstance

if TYPE_CHECKING:
    from selene_core.build_utils import Step

_log = logging.getLogger(__name__)


class BuildMethod(Enum):
    """
    An enumeration of the different methods that can be used to build a
    compiled user program as an object file before linking with selene.

    The methods are as follows:
    - VIA_LLVM_BITCODE: Compile the HUGR package to LLVM bitcode, then
        link the bitcode to a native object
    - VIA_LLVM_IR: Compile the HUGR package to LLVM IR, then link the IR
        to a native object

    These methods should produce identical objects, but produce intermediate
    files that may be useful for analysis.
    """

    VIA_LLVM_BITCODE = "via-llvm-bitcode"
    VIA_LLVM_IR = "via-llvm-ir"


class BitcodeString:
    bitcode: bytes

    def __init__(self, bitcode: bytes):
        self.bitcode = bitcode


def _collect_libdeps(
    planner: BuildPlanner,
    interface: QuantumInterface | None,
    utilities: Sequence[Utility] | None,
) -> list[LibDep]:
    """
    Collects the library dependencies for the selene build process,
    and registers the chosen quantum interface with the planner.
    """
    from selene_helios_qis_plugin import HeliosInterface

    interface = interface or HeliosInterface()
    deps = [LibDep.from_plugin(interface)]
    interface.register_build_steps(planner)
    for u in utilities or []:
        deps.append(LibDep.from_plugin(u))
    return deps


def build(
    src: Any,
    name: str | None = None,
    *,
    build_dir: Path | None = None,
    interface: QuantumInterface | None = None,
    utilities: Sequence[Utility] | None = None,
    verbose: bool = False,
    planner: BuildPlanner | None = None,
    progress_bar: bool = False,
    strict: bool = False,
    save_planner: bool = False,
    **kwargs,
) -> SeleneInstance:
    """
    Build an selene runner from a supported input resource type, e.g.
    a hugr package, LLVM file, etc. The type of this input is determined
    by matching against registered artifact kinds in the build planner.

    The build planner is used to determine a sequence of steps to transform
    the input resource into a selene executable. These are then performed,
    and the selene executable is returned wrapped in a SeleneInstance instance.

    Args:
        src: The input resource representing a hybrid program.
        name: The stem of the output executable filename, and referred
              to in temporary paths created in the process. If None
              (default), a random name is used.
        build_dir: The directory in which to establish the directory structure
                   for building and running the selene instance. Defaults to
                   a temporary directory.
        build_method: Where applicable, the method used to build the selene
                      executable. Defaults to BuildMethod.VIA_LLVM_BITCODE.
        interface: The quantum interface to target. This defaults to the
                   Helios QIS.
        utilities: A list of utility plugins to use. These are linked in with
                   the final program to expose additional symbols to the user
                   program.
        verbose: If True, the build process will be more verbose.
        planner: The build planner to use. If None, the default global planner
                 is used. For more information, see the selene_core.build_utils
                 documentation.
        progress_bar: If True, a progress bar will be displayed during the
                      build process. This requires tqdm to be installed.
        strict: If True, intermediate artifacts will be validated against their kinds
                on each step. This is more expensive, and is only recommended when
                debugging or developing new artifact kinds and build steps.
    Returns:
        An SeleneInstance object representing the built selene runner
    """
    if planner is None:
        # The DEFAULT_BUILD_PLANNER is a global planner exposed by
        # selene_core.build_utils. This has been set up to include
        # useful defaults, such as building HUGR and linking to the
        # Helios QIS. It can be extended by registering additional
        # build steps (internally or externally).
        #
        # In some cases, users may want to use a planner that does
        # not have the defaults included, and in those cases they
        # can create a new planner, register the steps they wish to
        # use, and pass it to this function.
        planner = DEFAULT_BUILD_PLANNER

    # Generate a random name if none is provided
    name = name or ("".join(random.choices(string.ascii_lowercase, k=8)))
    # If no build_dir is provided, use a temporary directory for building
    instance_root = Path(build_dir or tempfile.mkdtemp(prefix=f"selene_{name}_"))
    # Create the directory structure for the selene instance
    artifact_dir = instance_root / "artifacts"
    artifact_dir.mkdir(parents=True, exist_ok=True)
    run_dir = instance_root / "runs"
    run_dir.mkdir(parents=True, exist_ok=True)
    _log.info("Instance dir: %s", instance_root)

    # Create a build planner and register additional build information
    # from the interface and utilities passed in. This is necessary for
    # interfaces and utilities to be able to customise the final build,
    # e.g. adding link path arguments.
    deps = _collect_libdeps(planner, interface, utilities)

    # User-provided kwargs are bundled into a cfg dict used by the build
    # planner for customisation.
    cfg = kwargs
    if "build_method" not in cfg:
        # If the build method is not provided, default to VIA_LLVM_BITCODE
        cfg["build_method"] = BuildMethod.VIA_LLVM_BITCODE.value
    elif isinstance(cfg["build_method"], BuildMethod):
        cfg["build_method"] = cfg["build_method"].value

    # Add metadata that will be used when identifying build steps and
    # executing them.
    ctx = BuildCtx(
        artifact_dir=artifact_dir,
        deps=deps,
        cfg=cfg,
        verbose=verbose,
    )
    # Given the input resource, use the build planner to identify the kind of
    # resource it is by trying to match against registered artifact kinds in
    # priority order. Note that by registering extensions to the build planner,
    # new kinds of artifact can be added with a higher priority than the default
    # kinds, and this can lead to a *different* kind being assigned if it matches
    # against 'src'.
    #
    # This is by design. It allows for customisation and specialisation of the
    # build process. It is up to implementers to ensure that this does not lead
    # to a confusing experience for users. For example, if a high priority kind
    # is registered that simply matches:
    # > is this object a pathlib.Path?"
    # then this could override other kinds that are more specific, such as:
    # > is this object a pathlib.Path to an LLVM file with undefined symbols with
    #   a `new_shiny_platform__` prefix?
    input_kind = planner.identify_kind(src)
    if input_kind is None:
        raise ValueError(f"Unknown resource type: {type(src)}")

    if save_planner:
        _log.info("Saving planner to %s", instance_root / "planner.dot")
        planner.write_dot(instance_root / "planner.dot")

    # Wrap up the input as an artifact, and use the build planner to
    # determine the steps to take to build the selene executable.
    artifacts = [Artifact(src, input_kind)]
    steps = planner.get_optimal_steps_between(input_kind, SeleneExecutableKind, ctx)
    steps_to_iterate_over: Iterable[Step] = list(steps)
    if save_planner:
        _log.info("Saving planner with optimal path to %s", instance_root / "build.dot")
        planner.write_dot(instance_root / "build.dot", highlighted_steps=steps)
    # If a progress bar has been requested, wrap up the steps in tqdm
    if progress_bar:
        if "PYTEST_CURRENT_TEST" in os.environ:
            # Disable progress bar in pytest
            _log.warning("Progress bar disabled in pytest mode")
        else:
            try:
                # display progress bar iff tqdm is installed
                from tqdm import tqdm

                steps_to_iterate_over = tqdm(steps, desc="Building", unit="step")
            except ImportError:
                _log.warning("Please install tqdm to show progress bar")
                pass

    # Walk through the path from the input resource to the selene executable,
    # applying each step in turn. If a strict build has been requested, artifact
    # kind validation is performed on the output of each step.
    for step in steps_to_iterate_over:
        artifacts.append(step.apply(ctx, artifacts[-1]))
        if strict:
            assert artifacts[-1].validate_kind(), (
                f"Artifact failed validation: {artifacts[-1]}"
            )

    # The build has completed, and now it is time to wrap all of the required
    # running information into a SeleneInstance. This includes the selene executable,
    # pertinent metadata (such as paths and library search directories required for
    # dynamic linking), and the created run directory that will store information about
    # each run.
    selene_executable = artifacts[-1]
    executable_artifact = selene_executable.resource
    library_search_dirs = selene_executable.metadata.get("library_search_dirs", [])
    assert isinstance(library_search_dirs, list), (
        f"Expected library_path to be a list of paths, got {type(library_search_dirs)}"
    )
    for p in library_search_dirs:
        assert isinstance(p, Path), (
            f"Expected library_path to be a list of Path, but it includes {type(p)}"
        )
        assert p.is_dir(), (
            f"Expected library_path to be a directory, but it is not: {p}"
        )

    instance = SeleneInstance(
        instance_root, artifact_dir, run_dir, executable_artifact, library_search_dirs
    )
    instance.write_manifest(
        {
            "created_by": "selene-build 0.1",
            "deps": [d.__dict__ for d in deps],
            "steps": steps,
            "artifacts": artifacts,
            "library_search_dirs": library_search_dirs,
        }
    )
    return instance
