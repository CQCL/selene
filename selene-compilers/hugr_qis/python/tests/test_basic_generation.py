import pytest
from guppylang.decorator import guppy
from guppylang.std.builtins import array, exit, panic, result
from guppylang.std.qsystem.random import RNG
from guppylang.std.qsystem.utils import get_current_shot
from guppylang.std.quantum import (
    cx,
    discard,
    discard_array,
    h,
    measure,
    measure_array,
    qubit,
    t,
    tdg,
    x,
    z,
)
from selene_hugr_qis_compiler import compile_to_llvm_ir

triples = [
    "x86_64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-windows-msvc",
]


@pytest.mark.parametrize("target_triple", triples)
def test_llvm_no_results(snapshot, target_triple):
    @guppy
    def main() -> None:
        q0: qubit = qubit()
        h(q0)
        m = measure(q0)

    hugr_envelope = guppy.compile(main).package.to_bytes()
    ir = compile_to_llvm_ir(hugr_envelope, target_triple=target_triple)
    snapshot.assert_match(ir, f"no_results_{target_triple}")


@pytest.mark.parametrize("target_triple", triples)
def test_llvm_flip_some(snapshot, target_triple):
    @guppy
    def main() -> None:
        q0: qubit = qubit()
        q1: qubit = qubit()
        q2: qubit = qubit()
        q3: qubit = qubit()
        x(q0)
        x(q2)
        x(q3)
        result("c0", measure(q0))
        result("c1", measure(q1))
        result("c2", measure(q2))
        result("c3", measure(q3))

    hugr_envelope = guppy.compile(main).package.to_bytes()
    ir = compile_to_llvm_ir(hugr_envelope, target_triple=target_triple)
    snapshot.assert_match(ir, f"flip_some_{target_triple}")


@pytest.mark.parametrize("target_triple", triples)
def test_llvm_discard_array(snapshot, target_triple):
    @guppy
    def main() -> None:
        qs = array(qubit() for _ in range(10))
        discard_array(qs)

    hugr_envelope = guppy.compile(main).package.to_bytes()
    ir = compile_to_llvm_ir(hugr_envelope, target_triple=target_triple)
    snapshot.assert_match(ir, f"discard_array_{target_triple}")


@pytest.mark.parametrize("target_triple", triples)
def test_llvm_measure_array(snapshot, target_triple):
    @guppy
    def main() -> None:
        qs = array(qubit() for _ in range(10))
        x(qs[0])
        x(qs[2])
        x(qs[3])
        x(qs[9])
        cs = measure_array(qs)

    hugr_envelope = guppy.compile(main).package.to_bytes()
    ir = compile_to_llvm_ir(hugr_envelope, target_triple=target_triple)
    snapshot.assert_match(ir, f"measure_array_{target_triple}")


@pytest.mark.parametrize("target_triple", triples)
def test_llvm_print_array(snapshot, target_triple):
    @guppy
    def main() -> None:
        qs = array(qubit() for _ in range(10))
        x(qs[0])
        x(qs[2])
        x(qs[3])
        x(qs[9])
        cs = measure_array(qs)
        result("cs", cs)
        result("is", array(i for i in range(100)))
        result("fs", array(i * 0.0625 for i in range(100)))

    hugr_envelope = guppy.compile(main).package.to_bytes()
    ir = compile_to_llvm_ir(hugr_envelope, target_triple=target_triple)
    snapshot.assert_match(ir, f"print_array_{target_triple}")


@pytest.mark.parametrize("target_triple", triples)
def test_llvm_exit(snapshot, target_triple):
    """
    This test verifies the behaviour of exit(), which should stop the shot
    and add the error message to the result stream, but should then resume
    further shots.
    """

    @guppy
    def main() -> None:
        q = qubit()
        h(q)
        outcome = measure(q)
        if outcome:
            exit("Postselection failed", 42)
        result("c", outcome)

    hugr_envelope = guppy.compile(main).package.to_bytes()
    ir = compile_to_llvm_ir(hugr_envelope, target_triple=target_triple)
    snapshot.assert_match(ir, f"exit_{target_triple}")


@pytest.mark.parametrize("target_triple", triples)
def test_llvm_panic(snapshot, target_triple):
    """
    This test verifies the behaviour of panic(), which should stop the shot
    and should not allow any further shots to be performed. On the python
    client side, this should result in an Exception rather than being added
    into the results.
    """

    @guppy
    def main() -> None:
        q = qubit()
        h(q)
        outcome = measure(q)
        if outcome:
            panic("Postselection failed")
        result("c", outcome)

    hugr_envelope = guppy.compile(main).package.to_bytes()
    ir = compile_to_llvm_ir(hugr_envelope, target_triple=target_triple)
    snapshot.assert_match(ir, f"panic_{target_triple}")


@pytest.mark.parametrize("target_triple", triples)
def test_llvm_rus(snapshot, target_triple):
    @guppy
    def rus(q: qubit) -> None:
        while True:
            # Prepare ancillary qubits
            a, b = qubit(), qubit()
            h(a)
            h(b)

            tdg(a)
            cx(b, a)
            t(a)
            if not measure(a):
                # First part failed; try again
                discard(b)
                continue

            t(q)
            z(q)
            cx(q, b)
            t(b)
            if measure(b):
                # Success, we are done
                break

            # Otherwise, apply correction
            x(q)

    @guppy
    def main() -> None:
        q = qubit()
        rus(q)
        result("result", measure(q))

    hugr_envelope = guppy.compile(main).package.to_bytes()
    ir = compile_to_llvm_ir(hugr_envelope, target_triple=target_triple)
    snapshot.assert_match(ir, f"rus_{target_triple}")


@pytest.mark.parametrize("target_triple", triples)
def test_llvm_get_current_shot(snapshot, target_triple):
    @guppy
    def main() -> None:
        result("shot", get_current_shot())

    hugr_envelope = guppy.compile(main).package.to_bytes()
    ir = compile_to_llvm_ir(hugr_envelope, target_triple=target_triple)
    snapshot.assert_match(ir, f"current_shot_{target_triple}")


@pytest.mark.parametrize("target_triple", triples)
def test_llvm_rng(snapshot, target_triple):
    @guppy
    def main() -> None:
        rng = RNG(42)
        rint = rng.random_int()
        rint1 = rng.random_int()
        rfloat = rng.random_float()
        rint_bnd = rng.random_int_bounded(100)
        rng.discard()
        result("rint", rint)
        result("rint1", rint1)
        result("rfloat", rfloat)
        result("rint_bnd", rint_bnd)
        rng = RNG(84)
        rint = rng.random_int()
        rfloat = rng.random_float()
        rint_bnd = rng.random_int_bounded(200)
        rng.discard()
        result("rint2", rint)
        result("rfloat2", rfloat)
        result("rint_bnd2", rint_bnd)

    hugr_envelope = guppy.compile(main).package.to_bytes()
    ir = compile_to_llvm_ir(hugr_envelope, target_triple=target_triple)
    snapshot.assert_match(ir, f"rng_{target_triple}")
