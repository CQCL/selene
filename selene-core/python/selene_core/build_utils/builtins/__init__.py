"""
This module exposes the selene and hugr artifacts and steps, for
registration with a build planner. Note that it does not expose
the compilation from hugr to selene-compatible object files, as
this is managed by the Interface plugin being used.
"""

from .selene import (
    SeleneExecutableKind,
    SeleneObjectFileKind,
    SeleneObjectToSeleneExecutable,
    register_selene_builtins,
)

from .hugr import (
    HUGRPackageKind,
    HUGRPackagePointerKind,
    HUGREnvelopeFileKind,
    HUGREnvelopeBytesKind,
    HUGRPackageToHUGREnvelopeBytesStep,
    HUGRPackagePointerToHugrPackageStep,
    HUGREnvelopeBytesToHUGRPackageStep,
    HUGREnvelopeBytesToHUGREnvelopeFileStep,
    HUGREnvelopeFileToHUGREnvelopeBytesStep,
    register_hugr_builtins,
)
from .helios import (
    HeliosLLVMIRStringKind,
    HeliosLLVMIRFileKind,
    HeliosLLVMBitcodeStringKind,
    HeliosLLVMBitcodeFileKind,
    HeliosObjectFileKind,
    LLVMBitcodeStringToLLVMBitcodeFileStep,
    LLVMIRStringToLLVMIRFileStep,
    HeliosLLVMIRFileToHeliosObjectFileStep,
    HeliosLLVMBitcodeFileToHeliosObjectFileStep,
    HeliosObjectFileToSeleneObjectFileStep_Linux,
    HeliosObjectFileToSeleneExecutableStep_Windows,
    HeliosObjectFileToSeleneExecutableStep_Darwin,
    register_helios_builtins,
)

from ..planner import BuildPlanner


def register_builtins(planner: BuildPlanner):
    """
    Register built-in types and steps to the build planner.
    """
    register_selene_builtins(planner)
    register_hugr_builtins(planner)
    register_helios_builtins(planner)


__all__ = [
    "SeleneExecutableKind",
    "SeleneObjectFileKind",
    "SeleneObjectToSeleneExecutable",
    "HUGREnvelopeFileKind",
    "HUGREnvelopeBytesKind",
    "HUGRPackageKind",
    "HUGRPackagePointerKind",
    "HUGRPackageToHUGREnvelopeBytesStep",
    "HUGRPackagePointerToHugrPackageStep",
    "HUGREnvelopeBytesToHUGRPackageStep",
    "HUGREnvelopeBytesToHUGREnvelopeFileStep",
    "HUGREnvelopeFileToHUGREnvelopeBytesStep",
    "HeliosLLVMIRStringKind",
    "HeliosLLVMIRFileKind",
    "HeliosLLVMBitcodeStringKind",
    "HeliosLLVMBitcodeFileKind",
    "HeliosObjectFileKind",
    "LLVMBitcodeStringToLLVMBitcodeFileStep",
    "LLVMIRStringToLLVMIRFileStep",
    "HeliosLLVMIRFileToHeliosObjectFileStep",
    "HeliosLLVMBitcodeFileToHeliosObjectFileStep",
    "HeliosObjectFileToSeleneObjectFileStep_Linux",
    "HeliosObjectFileToSeleneExecutableStep_Windows",
    "HeliosObjectFileToSeleneExecutableStep_Darwin",
    "register_builtins",
]
