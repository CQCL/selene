import pytest
import yaml

from guppylang import guppy
from guppylang.std.quantum import qubit, x, h, measure, measure_array
from guppylang.std.builtins import exit

from selene_sim.build import build
from selene_sim import Quest, Stim
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


def test_flip_some_with_metrics_unparsed(snapshot):
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
    snapshot.assert_match(yaml.dump(got), "unparsed_metrics")


@pytest.mark.skip("See https://github.com/CQCL/selene/issues/37")
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
    and should not allow any further shots to be performed. Unlike with
    parse_results=True, the panic should be added into the results,
    and the panic should not result in an exception.
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
    shots = list(
        list(shot)
        for shot in runner.run_shots(
            Stim(),
            n_qubits=1,
            n_shots=100,
            random_seed=1,
            parse_results=False,
        )
    )
    assert shots[0] == [("USER:BOOL:c", 0)]
    assert shots[1] == [("USER:BOOL:c", 0)]
    assert shots[2] == [("EXIT:INT:Postselection failed", 1001)]
    for subsequent in shots[3:]:
        assert subsequent == []
