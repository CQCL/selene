import pytest

from guppylang import guppy
from guppylang.std.quantum import qubit, x, h, measure, measure_array
from hugr.qsystem.result import QsysResult
from guppylang.std.builtins import exit

from selene_sim.build import build
from selene_sim import Quest, Stim
from selene_sim.exceptions import SelenePanicError
from selene_sim.event_hooks import MetricStore


def test_flip_some_unparsed():
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

    runner = build(main.compile(), "flip_n4")
    got = list(runner.run(Quest(), verbose=True, n_qubits=4, parse_results=False))
    expected = [
        ("USER:BOOL:c0", 1),
        ("USER:BOOL:c1", 0),
        ("USER:BOOL:c2", 1),
        ("USER:BOOL:c3", 1),
    ]
    assert got == expected, f"expected {expected}, got {got}"


def test_flip_some_multishot_unparsed():
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

    runner = build(main.compile(), "flip_n4")
    shots = runner.run_shots(
        Quest(), verbose=True, n_qubits=4, n_shots=10, parse_results=False
    )
    expected = [
        ("USER:BOOL:c0", 1),
        ("USER:BOOL:c1", 0),
        ("USER:BOOL:c2", 1),
        ("USER:BOOL:c3", 1),
    ]
    for shot in shots:
        got = list(shot)
        assert got == expected, f"expected {expected}, got {got}"


def test_flip_some_with_metrics_unparsed():
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

    runner = build(main.compile(), "flip_n4")
    store = MetricStore()
    got = list(
        runner.run(
            Quest(), verbose=True, n_qubits=4, parse_results=False, event_hook=store
        )
    )
    expected = [
        ("USER:BOOL:c0", 1),
        ("USER:BOOL:c1", 0),
        ("USER:BOOL:c2", 1),
        ("USER:BOOL:c3", 1),
        ("METRICS:INT:user_program:qalloc_count", 4),
        ("METRICS:INT:user_program:qfree_count", 4),
        ("METRICS:INT:user_program:reset_count", 4),
        ("METRICS:INT:user_program:measure_request_count", 4),
        ("METRICS:INT:user_program:measure_read_count", 4),
        ("METRICS:INT:user_program:rxy_count", 3),
        ("METRICS:INT:user_program:rzz_count", 0),
        ("METRICS:INT:user_program:rz_count", 0),
        ("METRICS:INT:user_program:global_barrier_count", 0),
        ("METRICS:INT:user_program:local_barrier_count", 0),
        ("METRICS:INT:user_program:max_allocated", 4),
        ("METRICS:INT:user_program:currently_allocated", 0),
        ("METRICS:INT:post_runtime:custom_op_batch_count", 0),
        ("METRICS:INT:post_runtime:custom_op_individual_count", 0),
        ("METRICS:INT:post_runtime:measure_batch_count", 4),
        ("METRICS:INT:post_runtime:measure_individual_count", 4),
        ("METRICS:INT:post_runtime:reset_batch_count", 4),
        ("METRICS:INT:post_runtime:reset_individual_count", 4),
        ("METRICS:INT:post_runtime:rxy_batch_count", 3),
        ("METRICS:INT:post_runtime:rxy_individual_count", 3),
        ("METRICS:INT:post_runtime:rz_batch_count", 0),
        ("METRICS:INT:post_runtime:rz_individual_count", 0),
        ("METRICS:INT:post_runtime:rzz_batch_count", 0),
        ("METRICS:INT:post_runtime:rzz_individual_count", 0),
        ("METRICS:INT:post_runtime:total_duration_ns", 0),
        ("METRICS:INT:emulator:shot_number", 0),
        ("METRICS:FLOAT:simulator:cumulative_postselect_probability", 1.0),
    ]
    assert got == expected, f"expected {expected}, got {got}"


def test_array_results_unparsed():
    @guppy
    def main() -> None:
        qs = array(qubit() for _ in range(10))
        for i in range(len(qs)):
            x(qs[i])
        bs = measure_array(qs)
        result("bools", bs)
        result("floats", array(1.0 / 2**i for i in range(10)))
        result("ints", array(i for i in range(100)))

    runner = build(main.compile(), "array_results")
    shots = list(
        list(shot)
        for shot in runner.run_shots(
            Stim(), n_qubits=10, n_shots=20, parse_results=False, verbose=True
        )
    )
    expected = [
        ("USER:BOOLARR:bools", [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
        (
            "USER:FLOATARR:floats",
            [
                1.0,
                0.5,
                0.25,
                0.125,
                0.0625,
                0.03125,
                0.015625,
                0.0078125,
                0.00390625,
                0.001953125,
            ],
        ),
        ("USER:INTARR:ints", list(range(100))),
    ]
    for shot in shots:
        assert shot == expected, f"Expected one of the two possibilities, got {shot}"


def test_exit_unparsed():
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

    runner = build(main.compile(), "exit")

    # some should have measurements of 0, some should have no measurements.
    n_1 = 0
    n_0 = 0
    n_exit = 0
    for shot in runner.run_shots(
        Quest(),
        n_qubits=1,
        n_shots=100,
        random_seed=0,
        parse_results=False,
    ):
        shot = list(shot)
        if shot == [
            ("EXIT:INT:Postselection failed", 42),
        ]:
            n_exit += 1
        elif shot == [
            ("USER:BOOL:c", 1),
        ]:
            n_1 += 1
        elif shot == [
            ("USER:BOOL:c", 0),
        ]:
            n_0 += 1
        else:
            raise ValueError(f"Unexpected shot result: {shot}")
    # no shots should have provided a result of 1
    assert n_1 == 0
    # some shots should have provided a result of 0
    assert n_0 > 0
    # some shots should have exited
    assert n_exit > 0
    # we still have 100 shots total
    assert n_0 + n_exit == 100


def test_panic_unparsed():
    """
    This test verifies the behaviour of panic(), which should stop the shot
    and should not allow any further shots to be performed. On the python
    client side, this should result in an Exception. Unlike with parse_result=True,
    the panic should STILL be added into the results.
    """

    @guppy
    def main() -> None:
        q = qubit()
        h(q)
        outcome = measure(q)
        if outcome:
            panic("Postselection failed")
        result("c", outcome)

    runner = build(main.compile(), "panic")
    with pytest.raises(
        SelenePanicError, match="Postselection failed"
    ) as exception_info:
        shots = QsysResult(
            runner.run_shots(
                Quest(),
                n_qubits=1,
                n_shots=100,
                random_seed=0,
                parse_results=False,
            )
        )
