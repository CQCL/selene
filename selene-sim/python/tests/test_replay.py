import datetime
import pytest

from guppylang.decorator import guppy
from guppylang.std.quantum import qubit, measure, h, cx, x
from guppylang.std.builtins import result

from selene_sim.build import build
from selene_sim import Quest, Stim, ClassicalReplay, QuantumReplay
from selene_sim.exceptions import SelenePanicError


def test_recursive_condition_successful_cases_single_process():
    """
    This test checks that the simulator can handle a recursive function, and
    that the ClassicalReplay is capable of validating expectations about control
    flow. The recursive code we use is a simple recursive function that measures
    a qubit, and then calls itself if the measurement is True. We output each measurement
    before checking the condition.

    The function should therefore always have exactly one False measurement (as the final
    output), and an arbitrary number of True measurements beforehand.
    """

    @guppy
    def recursive_condition() -> None:
        q = qubit()
        h(q)
        outcome = measure(q)
        result("c", outcome)
        if outcome:
            recursive_condition()

    @guppy
    def main() -> None:
        recursive_condition()

    runner = build(main.compile(), "recursive_condition")

    valid_measurements = [
        [False],
        [True] * 1 + [False],
        [True] * 10 + [False],
        [True] * 100 + [False],
        [True] * 1000 + [False],
    ]

    shots = runner.run_shots(
        ClassicalReplay(measurements=valid_measurements),
        1,
        n_shots=len(valid_measurements),
    )
    for expected, got in zip(valid_measurements, shots):
        got = map(lambda x: x[1] == 1, got)
        assert list(got) == expected


def test_recursive_condition_successful_cases_multi_process():
    @guppy
    def recursive_condition() -> None:
        q = qubit()
        h(q)
        outcome = measure(q)
        result("c", outcome)
        if outcome:
            recursive_condition()

    @guppy
    def main() -> None:
        recursive_condition()

    runner = build(main.compile(), "recursive_condition")

    valid_measurements = [
        [False],
        [True] * 1 + [False],
        [True] * 10 + [False],
        [True] * 100 + [False],
        [True] * 1000 + [False],
    ]

    # check multiprocessing works with ClassicalReplay
    shots = runner.run_shots(
        ClassicalReplay(measurements=valid_measurements),
        1,
        n_shots=len(valid_measurements),
        n_processes=3,
    )
    for expected, got in zip(valid_measurements, shots):
        got = map(lambda x: x[1] == 1, got)
        assert list(got) == expected


def test_recursive_condition_invalid_cases_single_process():
    @guppy
    def recursive_condition() -> None:
        q = qubit()
        h(q)
        outcome = measure(q)
        result("c", outcome)
        if outcome:
            recursive_condition()

    @guppy
    def main() -> None:
        recursive_condition()

    runner = build(main.compile(), "recursive_condition")

    invalid_measurements = [
        [],  # no outputs at all
        [True],  # no terminating False.
        [True] * 100,  # no terminating False.
        [False, True],  # extra measurement.
        [True] * 10 + [False, True],  # extra measurement.
    ]

    for invalid in invalid_measurements:
        try:
            list(
                runner.run(
                    ClassicalReplay(measurements=invalid),
                    1,
                    timeout=datetime.timedelta(seconds=1),
                )
            )
            assert False, "ClassicalReplay should have thrown an exception"
        except Exception:
            pass


def test_recursive_condition_invalid_cases_multi_process():
    @guppy
    def recursive_condition() -> None:
        q = qubit()
        h(q)
        outcome = measure(q)
        result("c", outcome)
        if outcome:
            recursive_condition()

    @guppy
    def main() -> None:
        recursive_condition()

    runner = build(main.compile(), "recursive_condition")

    invalid_measurements = [
        [],  # no outputs at all
        [True],  # no terminating False.
        [True] * 100,  # no terminating False.
        [False, True],  # extra measurement.
        [True] * 10 + [False, True],  # extra measurement.
    ]

    for invalid in invalid_measurements:
        try:
            list(
                runner.run(
                    ClassicalReplay(measurements=invalid),
                    1,
                    processes=3,
                    timeout=datetime.timedelta(seconds=1),
                )
            )
            assert False, "ClassicalReplay should have thrown an exception"
        except Exception:
            pass


@pytest.mark.parametrize("underlying_simulator_class", [Quest, Stim])
def test_quantum_replay(underlying_simulator_class):
    @guppy
    def main() -> None:
        q0: qubit = qubit()
        q1: qubit = qubit()
        q2: qubit = qubit()
        q3: qubit = qubit()
        h(q0)
        cx(q0, q1)
        cx(q1, q2)
        x(q2)
        cx(q2, q3)
        # at this stage, the state should be a superposition
        # of |0011> and |1100>
        result("c0", measure(q0))
        result("c1", measure(q1))
        result("c2", measure(q2))
        result("c3", measure(q3))
        # this the results should either be (0,0,1,1) or (1,1,0,0)
        # we can use quantum replay to verify this.

    runner = build(main.compile(), "quantum_replay_quest")
    underlying_simulator = underlying_simulator_class(random_seed=0)
    # run full replay - no measurements allowed, just postselection.
    # if the user program requires more measurements, this is an error.
    # use case: validating programs with the backing of a real simulator
    full_replay_measurements = [
        [True, True, False, False],
        [False, False, True, True],
    ]
    simulator = QuantumReplay(
        simulator=underlying_simulator,
        resume_with_measurement=False,  # only postselect
        measurements=full_replay_measurements,
    )
    shots = runner.run_shots(
        simulator,
        n_qubits=4,
        n_shots=len(full_replay_measurements),
        timeout=datetime.timedelta(seconds=1),
    )
    for expected, got in zip(full_replay_measurements, shots):
        got = dict(got)
        got = [bool(got[i]) for i in ["c0", "c1", "c2", "c3"]]
        assert got == expected

    # partial replay - provide some measurements, and allow the simulator
    # to take over afterwards. Usecase: getting your program into a certain
    # state before seeing how it behaves once in that state.

    # Because the state after running the above program is guaranteed to
    # a superposition of |0011> and |1100>, providing the first measurement
    # as 0 or 1 should dictate the remaining measurements under a reliable
    # simulation of the program.
    partial_replay_measurements = [
        {"init": [True], "expected": [True, False, False]},
        {"init": [False], "expected": [False, True, True]},
    ]
    simulator = QuantumReplay(
        simulator=underlying_simulator,
        resume_with_measurement=True,  # measure after all postselection is over
        measurements=list(r["init"] for r in partial_replay_measurements),
    )
    shots = runner.run_shots(
        simulator,
        n_qubits=4,
        n_shots=len(partial_replay_measurements),
        timeout=datetime.timedelta(seconds=1),
    )
    for spec, got in zip(partial_replay_measurements, shots):
        got = dict(got)
        got = [bool(got[i]) for i in ["c0", "c1", "c2", "c3"]]
        expected = spec["init"] + spec["expected"]
        assert got == expected

    # invalid replays - where postselection isn't possible or is simply
    # too unlikely. If the amplitude of the wanted state is very small,
    # numerical simulators are likely to introduce significant error in
    # the amplitudes of remaining states, or may even be put into an
    # invalid state entirely.

    # In this example, we provide a set of measurements that are impossible
    # given a reliable simulation of the provided program. We assert that
    # an exception is raised as a result.
    invalid_replay_measurements = [
        [True, False, False, True],  # this isn't a valid measurement outcome
    ]
    simulator = QuantumReplay(
        simulator=underlying_simulator,
        resume_with_measurement=False,
        measurements=invalid_replay_measurements,
    )

    with pytest.raises(SelenePanicError) as exception_info:
        s = list(
            list(x)
            for x in runner.run_shots(
                simulator,
                n_qubits=4,
                n_shots=len(invalid_replay_measurements),
                timeout=datetime.timedelta(seconds=1),
                verbose=True,
            )
        )
    assert any(x in str(exception_info.value) for x in ["impossible", "too unlikely"])
