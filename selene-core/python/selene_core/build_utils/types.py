from dataclasses import dataclass, field
from pathlib import Path, PosixPath, WindowsPath
from typing import Generic, TypeVar, Any, ClassVar, cast
from typing_extensions import Self
import yaml
import hashlib


@dataclass(frozen=True)
class LibDep:
    """
    Describes a library dependency for a plugin, which includes
    a path to the library file, any link flags, and any library
    search directories required for dynamic linking at runtime.
    """

    path: Path
    link_flags: list[str] = field(default_factory=list)
    library_search_dirs: list[Path] = field(default_factory=list)

    @classmethod
    def from_plugin(cls, plugin) -> Self:
        return cls(
            path=plugin.library_file,
            link_flags=plugin.link_flags,
            library_search_dirs=plugin.library_search_dirs,
        )


@dataclass
class BuildCtx:
    """
    The build context is a collection of information that is
    passed around during the build process. It contains the
    artifact directory, a list of dependencies (for example,
    libraries), and any configuration options that are chosen
    by the user.
    """

    artifact_dir: Path
    deps: list[LibDep]  # interface + utilities
    cfg: dict[str, Any] = field(default_factory=dict)  # user provided metadata
    verbose: bool = False  # verbose output


class BuildTypeMeta(type):
    """
    A metaclass for core types in the build system.

    Artifact kinds and steps have class semantics rather than
    instance semantics. After all, a 'kind' is a type of
    classification, carrying no state, but we wish to have
    collections of them and so on.
    """

    def __str__(cls) -> str:
        if hasattr(cls, "__class_repr__"):
            return cls.__class_repr__()
        return f"{cls.__module__}.{cls.__name__}"


ResourceKind = TypeVar("ResourceKind")


class ArtifactKind(Generic[ResourceKind], metaclass=BuildTypeMeta):
    """
    An artifact kind characterises a type of resource. Examples of
    characterisation might include:
    - a path to an object file targeting a specific quantum instruction
    set
    - a HUGR package
    - a bitcode bytestring
    - an URL to a HUGR package (not supported but theoretically possible)

    It can be used to detect the type of a resource (an Any variable).
    The build planner (see planner.py) uses the ArtifactKind of a resource
    to determine the path(s) to the final selene executable.
    """

    priority: ClassVar[int] = 1000

    @classmethod
    def matches(cls, resource: Any) -> bool:
        """
        Returns true if the resource matches this kind, false otherwise.
        """
        raise NotImplementedError(
            f"matches() not implemented for {cls.__module__}.{cls.__name__}"
        )

    @classmethod
    def canonicalize(cls, resource: ResourceKind) -> ResourceKind:
        """
        Convert the resource to a canonical form, if necessary.
        As an example, a pathlib.Path resource might be resolved
        to an absolute path, or a string resource might be converted
        to lowercase (for example).

        A default for handling paths is provided here, but this
        should be overwritten where appropriate.
        """
        if isinstance(resource, Path):
            return cast(ResourceKind, resource.resolve())
        return resource

    @classmethod
    def digest(cls, resource: ResourceKind) -> str:
        """
        Generate a digest for the resource for lookup and caching.
        """
        h = hashlib.sha256()
        h.update(str(cls).encode())
        if isinstance(resource, Path):
            h.update(resource.read_bytes())
        elif isinstance(resource, bytes):
            h.update(resource)
        elif isinstance(resource, str):
            h.update(resource.encode())
        else:
            raise NotImplementedError(
                f"No digest defined for resource type: {type(resource)}"
            )
        return h.hexdigest()


@dataclass
class Artifact(Generic[ResourceKind], metaclass=BuildTypeMeta):
    """
    An artifact is a 'realised' artifact kind, and thus contains
    a resource, its kind, and any metadata gathered during the build
    process.
    """

    resource: ResourceKind
    kind: type[ArtifactKind[ResourceKind]]
    metadata: dict[str, Any] = field(default_factory=dict)

    def validate_kind(self) -> bool:
        return self.kind.matches(self.resource)

    def canonicalize(self) -> None:
        self.resource = self.kind.canonicalize(self.resource)

    def digest(self) -> str:
        return self.kind.digest(self.resource)


InputKind = TypeVar("InputKind", bound=ArtifactKind)
OutputKind = TypeVar("OutputKind", bound=ArtifactKind)


@dataclass
class Step(metaclass=BuildTypeMeta):
    """
    A Step describes a single step in a build process, i.e. a transformation
    to an input ArtifactKind to an output ArtifactKind. It has an associated
    'cost', which may depend on some user-provided build context or the current
    platform.

    The Build Planner (see planner.py) uses the costing of each step to determine
    the optimal path from an input artifact to a final selene executable. Thus
    the cost of a step can be customised to prioritize or penalize a particular
    path in chosen circumstances.
    """

    input_kind: type[ArtifactKind]
    output_kind: type[ArtifactKind]

    @classmethod
    def get_cost(cls, build_ctx: BuildCtx) -> float:
        """
        When planning a build sequence, the path of minimal (cumulative) cost is chosen.
        This class method returns the 'cost' of this step, given the current build context
        and artifact. Use the context provided to decide whether a step should be prioritised
        by giving it a low cost, penalised by giving it a high cost, or ruled out entirely
        by giving it float('inf').

        For example, say you have two similar steps that offer a choice, such as compiling
        an IR to LLVM bitcode or to LLVM IR (text). You can use the build_ctx to accept user
        input to select between the two, providing a default (unequal) cost to prioritise
        a default route.

        Negative costs are not permitted.
        """
        return 100

    @classmethod
    def apply(
        cls, build_ctx: BuildCtx, input_artifact: Artifact
    ) -> Artifact[OutputKind]:
        """
        Convert the input artifact to the output artifact.
        """
        raise NotImplementedError(f"apply() not implemented for {cls}")

    @classmethod
    def _make_artifact(
        cls, resource: OutputKind, metadata: dict[str, Any] | None = None
    ) -> Artifact[OutputKind]:
        """
        A helper method to create an artifact of the output kind
        given an output resource.
        """
        return Artifact(resource, cls.output_kind, metadata or {})


"""
As we provide a YAML file to store details about a build, we need to be
able to serialise the different types of objects we use in the build system.
We define these here.
"""
yaml.SafeDumper.add_representer(
    Path,
    lambda dumper, data: dumper.represent_scalar("tag:yaml.org,2002:str", str(data)),
)
yaml.SafeDumper.add_representer(
    PosixPath,
    lambda dumper, data: dumper.represent_scalar("tag:yaml.org,2002:str", str(data)),
)
yaml.SafeDumper.add_representer(
    WindowsPath,
    lambda dumper, data: dumper.represent_scalar("tag:yaml.org,2002:str", str(data)),
)
yaml.SafeDumper.add_representer(
    LibDep,
    lambda dumper, data: dumper.represent_mapping(
        "tag:yaml.org,2002:map",
        {
            "path": str(data.path),
            "link_flags": data.link_flags,
            "library_search_dirs": [str(p) for p in data.library_search_dirs],
        },
    ),
)
yaml.SafeDumper.add_representer(
    Artifact,
    lambda dumper, data: dumper.represent_mapping(
        "tag:yaml.org,2002:map",
        {
            "resource": str(data.resource),
            "kind": data.kind,
            "metadata": data.metadata,
        },
    ),
)

yaml.SafeDumper.add_representer(
    BuildCtx,
    lambda dumper, data: dumper.represent_mapping(
        "tag:yaml.org,2002:map",
        {
            "artifact_dir": str(data.artifact_dir),
            "deps": data.deps,
            "cfg": data.cfg,
            "verbose": data.verbose,
        },
    ),
)


def repr_build_type(dumper: yaml.SafeDumper, cls: BuildTypeMeta) -> yaml.Node:
    values = {
        "module": cls.__module__,
        "class": cls.__name__,
    }
    for attr, val in cls.__dict__.items():
        if not attr.startswith("_") and not callable(val):
            try:
                dumper.represent_data(val)
                values[attr] = val
            except yaml.representer.RepresenterError:
                pass

    return dumper.represent_mapping("tag:yaml.org,2002:map", values)


yaml.SafeDumper.add_multi_representer(BuildTypeMeta, repr_build_type)
