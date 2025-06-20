def compile_to_bitcode(pkg_bytes: bytes, opt_level: int = 2) -> bytes:
    """Compile serialized HUGR to LLVM IR bitcode"""
    ...

def compile_to_llvm_ir(pkg_bytes: bytes, opt_level: int = 2) -> str:
    """Compile serialized HUGR to LLVM IR string"""
    ...
