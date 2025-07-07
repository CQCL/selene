import pytest
import platform

from guppylang import guppy
from guppylang.std.quantum import discard, qubit

from selene_sim.build import build
from selene_sim import Quest


@pytest.mark.xfail(
    platform.system() == "Windows",
    reason=(
        "As Lief doesn't support COFF formats yet, we can't "
        "detect undefined symbols in Windows lib files, and "
        "thus can't run a strict build on Windows."
    ),
)
@pytest.mark.parametrize(
    "build_config",
    [
        {"name": "default", "kwargs": {}},
        {"name": "llvm_ir", "kwargs": {"build_method": "via-llvm-ir"}},
        {"name": "llvm_bc", "kwargs": {"build_method": "via-llvm-bitcode"}},
    ],
)
def test_strict_builds(build_config, snapshot):
    """
    Selene sim builds are multi-step processes that
    are described by a build graph, with artifacts
    as nodes and build steps as edges.

    Artifacts have 'kinds' that allow an incoming
    artifact (e.g. passed to `selene_sim.build`)
    to be matched against in order to establish the
    build sequence.

    Examples:
      - An .ll file that is expected to depend on
        specific function signatures.
      - A .o file that depends on undefined functions
        that we expect to be defined after linking.

    By default, we only run artifact kind matching
    on the user input - this is because checking can
    be intensive (e.g. invoking system utilities to
    identify undefined functions in an object file, or
    parsing an LLVM file).

    If we pass `strict=True` to `build`, then outputs of
    steps are also checked against their expected kind.
    Here we do that test to ensure that standard builds
    and all of their intermediate artifacts conform to
    expectations - and, importantly, to ensure that the
    checks are working as intended.
    """

    @guppy
    def main() -> None:
        q0 = qubit()
        discard(q0)

    runner = build(guppy.compile(main), strict=True, **build_config["kwargs"])
    got = list(runner.run(Quest(), n_qubits=1))
    assert len(got) == 0
