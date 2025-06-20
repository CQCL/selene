"""
This module contains the built-in artifact kinds representing HUGR artifacts
and the steps that convert between them.

Call register_hugr_builtins() on a planner to register them. This is called
automatically on the default planner, so manual invocation is only necessary
if you are using a fresh custom planner.
"""

from typing import Any
from pathlib import Path

import hugr.package as hp
import hugr.envelope as he

from ..planner import BuildPlanner
from ..types import ArtifactKind, Step, BuildCtx, Artifact


class HUGRPackageKind(ArtifactKind[hp.Package]):
    @staticmethod
    def matches(resource: Any) -> bool:
        return isinstance(resource, hp.Package)


class HUGRPackagePointerKind(ArtifactKind[hp.PackagePointer]):
    @staticmethod
    def matches(resource: Any) -> bool:
        return isinstance(resource, hp.PackagePointer)


class HUGREnvelopeBytesKind(ArtifactKind[bytes]):
    @staticmethod
    def matches(resource: Any) -> bool:
        if not isinstance(resource, bytes):
            return False
        return resource[:8] == he.MAGIC_NUMBERS


class HUGREnvelopeFileKind(ArtifactKind[Path]):
    @staticmethod
    def matches(resource: Any) -> bool:
        if not isinstance(resource, Path):
            return False
        if not resource.is_file():
            return False
        if resource.suffix != ".hugr":
            return False
        with resource.open("rb") as handle:
            try:
                magic = handle.read(8)
            except OSError:
                return False
        return magic == he.MAGIC_NUMBERS


class HUGRPackagePointerToHugrPackageStep(Step):
    """
    Convert a HUGR package pointer to a HUGR package
    """

    input_kind = HUGRPackagePointerKind
    output_kind = HUGRPackageKind

    @classmethod
    def apply(cls, build_ctx: BuildCtx, input_artifact: Artifact) -> Artifact:
        if build_ctx.verbose:
            print("Converting HUGR package pointer to HUGR package")
        return cls._make_artifact(input_artifact.resource.package)


class HUGRPackageToHUGREnvelopeBytesStep(Step):
    """
    Convert a HUGR package to a .hugr file
    """

    input_kind = HUGRPackageKind
    output_kind = HUGREnvelopeBytesKind

    @classmethod
    def apply(cls, build_ctx: BuildCtx, input_artifact: Artifact) -> Artifact:
        result = input_artifact.resource.to_bytes()
        return cls._make_artifact(result)


class HUGREnvelopeBytesToHUGRPackageStep(Step):
    """
    Convert a HUGR file to a HUGR package
    """

    input_kind = HUGREnvelopeBytesKind
    output_kind = HUGRPackageKind

    @classmethod
    def apply(cls, build_ctx: BuildCtx, input_artifact: Artifact) -> Artifact:
        if build_ctx.verbose:
            print(f"Converting HUGR file {input_artifact.resource} to HUGR package")
        pkg = hp.Package.from_bytes(input_artifact.resource)
        return cls._make_artifact(pkg)


class HUGREnvelopeBytesToHUGREnvelopeFileStep(Step):
    """
    Convert a HUGR file to a HUGR package
    """

    input_kind = HUGREnvelopeBytesKind
    output_kind = HUGREnvelopeFileKind

    @classmethod
    def apply(cls, build_ctx: BuildCtx, input_artifact: Artifact) -> Artifact:
        out_path = build_ctx.artifact_dir / "package.hugr"
        if build_ctx.verbose:
            print(f"Converting HUGR file to HUGR file: {out_path}")
        with open(out_path, "wb") as f:
            f.write(input_artifact.resource)
        return cls._make_artifact(out_path)


class HUGREnvelopeFileToHUGREnvelopeBytesStep(Step):
    """
    Convert a HUGR file to a HUGR package
    """

    input_kind = HUGREnvelopeFileKind
    output_kind = HUGREnvelopeBytesKind

    @classmethod
    def apply(cls, build_ctx: BuildCtx, input_artifact: Artifact) -> Artifact:
        if build_ctx.verbose:
            print(f"Converting HUGR file {input_artifact.resource} to HUGR package")
        pkg = input_artifact.resource.read_bytes()
        return cls._make_artifact(pkg)


def register_hugr_builtins(planner: BuildPlanner) -> None:
    planner.add_kind(HUGRPackageKind)
    planner.add_kind(HUGRPackagePointerKind)
    planner.add_kind(HUGREnvelopeBytesKind)
    planner.add_kind(HUGREnvelopeFileKind)
    planner.add_step(HUGRPackagePointerToHugrPackageStep)
    planner.add_step(HUGRPackageToHUGREnvelopeBytesStep)
    planner.add_step(HUGREnvelopeBytesToHUGRPackageStep)
    planner.add_step(HUGREnvelopeBytesToHUGREnvelopeFileStep)
    planner.add_step(HUGREnvelopeFileToHUGREnvelopeBytesStep)
