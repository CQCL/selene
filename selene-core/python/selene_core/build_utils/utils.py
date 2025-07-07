# utilities to help kinds and steps with common behaviour

import platform
import sys
from pathlib import Path


def get_target_triple(arch: str | None = None, system: str | None = None) -> str | None:
    """
    MacOS needs pointing to libSystem compatible with 11.0,
    as the default is the current platform which selene components
    might be incompatible with.

    Windows needs pointing to MSVC, as the default is MinGW,
    and selene components are shipped with MSVC bindings.

    Linux doesn't need further specification, as the default behaviour
    is correct. Using e.g. "x86_64-linux-gnu" would fail on nixos, for
    example.
    """

    if arch is None:
        arch = platform.machine()
    if system is None:
        system = platform.system()

    target_system = ""
    target_arch = ""

    match system.lower():
        case "linux":
            return None
        case "darwin" | "macos":
            target_system = "macos.11.0-none"
        case "windows":
            target_system = "windows-msvc"
        case _:
            raise RuntimeError(f"Unsupported OS: {system}")

    match arch.lower():
        case "arm64" | "aarch64":
            target_arch = "aarch64"
        case "amd64" | "x86_64":
            target_arch = "x86_64"
        case _:
            raise RuntimeError(f"Unsupported architecture: {arch}")
    return f"{target_arch}-{target_system}"


def invoke_zig(*args, handle_triple=True, verbose=False) -> str:
    """
    Invoke zig with the given arguments, after conversion to strings.
    """
    import subprocess

    args_str = [str(arg) for arg in args]
    if handle_triple:
        target_triple = get_target_triple()
        if target_triple is not None:
            args_str += ["-target", target_triple]
    argv = [sys.executable, "-m", "ziglang"] + args_str
    if verbose:
        print(f"zig command: {' '.join(argv)}")
    handle = subprocess.run(argv, stdout=None, stderr=subprocess.PIPE, text=True)
    if handle.returncode != 0:
        raise RuntimeError(
            f"zig command failed:\n  Command: {' '.join(argv)}\n  Error: {handle.stderr}"
        )
    return handle.stdout


def get_undefined_symbols_from_object(path: Path) -> list[str]:
    """
    Extract undefined symbols from an object file, with help from the `lief` library.

    This function is useful for inspection of object files, such as in the `matches`
    methods of ArtifactKind specialisations.
    """
    import lief

    binary = lief.parse(path)
    if isinstance(binary, lief.ELF.Binary):
        # ELF: undefined symbols have shndx == 0 (SHN_UNDEF)
        return [
            s.name
            for s in binary.symbols
            if s.shndx == 0 and s.value == 0  # Optional extra check
        ]

    elif isinstance(binary, lief.MachO.Binary):
        # Mach-O: undefined symbols have section_number == 0
        def demangle(name):
            return name[1:] if name.startswith("_") else name

        return [demangle(s.name) for s in binary.symbols if s.is_external]

    elif isinstance(binary, lief.PE.Binary):
        # PE doesn't have undefined symbols in the same sense, but we can check imports
        return [
            entry.name
            for entry in binary.imports
            for func in entry.entries
            if func.is_ordinal is False and func.name
        ]

    else:
        raise NotImplementedError(
            f"Unsupported binary format for undefined symbol extraction: {path}"
        )


def get_undefined_symbol_from_llvm_ir_line(line: str) -> str | None:
    if line.startswith("declare "):
        # Extract the function name from the line
        try:
            return line.split()[2][1:].split("(")[0]
        except Exception:
            pass
    return None


def get_undefined_symbols_from_llvm_ir_file(ir: Path) -> list[str]:
    """
    Extract undefined symbols from an LLVM IR file.

    This is a naive implementation that reads the file line by line and
    looks for lines that start with "declare". It may be fragile.
    """
    result = []
    with ir.open("r") as handle:
        for line in handle:
            symbol = get_undefined_symbol_from_llvm_ir_line(line)
            if symbol:
                result.append(symbol)
    return result


def get_undefined_symbols_from_llvm_ir_string(ir: str) -> list[str]:
    """
    Extract undefined symbols from an LLVM IR string.

    This is a naive implementation that reads the file line by line and
    looks for lines that start with "declare". It may be fragile.
    """
    result = []
    for line in ir.splitlines():
        symbol = get_undefined_symbol_from_llvm_ir_line(line)
        if symbol:
            result.append(symbol)
    return result
