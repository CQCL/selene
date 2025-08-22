import datetime
import time

import pytest
from guppylang.decorator import guppy
from guppylang.std.builtins import result
from guppylang.std.quantum import (
    h,
    measure,
    qubit,
)
from hugr.qsystem.result import QsysResult
from selene_sim import Quest
from selene_sim.build import build
from selene_sim.exceptions import SeleneTimeoutError, SeleneStartupError
from selene_sim.timeout import Timeout


@pytest.fixture(scope="module")
def non_terminating_with_results():
    @guppy
    def prog() -> None:
        while True:
            q0: qubit = qubit()
            h(q0)
            result("r", measure(q0))

    return build(prog.compile())


@pytest.fixture(scope="module")
def non_terminating_without_results():
    @guppy
    def recurse(i: int) -> int:
        # oops, infinite recursion!
        return recurse(i + 1)

    @guppy
    def prog() -> None:
        result("i", recurse(0))

    return build(prog.compile())


def test_timeout_zero(non_terminating_with_results):
    # if a timeout is set at 0, there is no time provided for any selene clients
    # to startup and connect to the results stream. As such we expect a SeleneTimeoutError
    with pytest.raises(
        SeleneStartupError, match="Timed out waiting for a client to connect for shot 0"
    ):
        QsysResult(
            non_terminating_with_results.run_shots(
                Quest(random_seed=1234),
                n_qubits=1,
                n_shots=100,
                timeout=datetime.timedelta(seconds=0),
            )
        )


def test_timeout_overall(non_terminating_with_results):
    # if a timeout is set at 1, there is plenty of time for the selene clients to connect,
    # but as the program is an infinite loop, we expect a SeleneTimeoutError after 1 second.
    start_time = time.perf_counter()
    with pytest.raises(SeleneTimeoutError, match="Expired timers: 'overall'"):
        QsysResult(
            non_terminating_with_results.run_shots(
                Quest(random_seed=1234),
                n_qubits=1,
                n_shots=100,
                timeout=datetime.timedelta(seconds=1),
            )
        )
    end_time = time.perf_counter()
    assert 1 <= end_time - start_time < 2, "Timeout should have taken around 1 second"


def test_timeout_per_shot(non_terminating_with_results):
    # In the above, we used a timeout with a timedelta. This sets the overall run_shots
    # duration. Now let's look at a per-shot timeout.
    start_time = time.perf_counter()
    with pytest.raises(SeleneTimeoutError, match="Expired timers: 'per_shot'"):
        QsysResult(
            non_terminating_with_results.run_shots(
                Quest(random_seed=1234),
                n_qubits=1,
                n_shots=100,
                timeout=Timeout(
                    overall=datetime.timedelta(seconds=10),
                    per_shot=datetime.timedelta(seconds=1),
                ),
            )
        )
    end_time = time.perf_counter()
    assert 1 <= end_time - start_time < 2, (
        "Per-shot timeout should have taken around 1 second"
    )


def test_timeout_per_result(non_terminating_without_results):
    # Now let's look at the case where the infinite loop doesn't emit results.
    # We have an additional timeout setting, 'per_result', which is useful when
    # we expect a relatively steady stream of outputs from the program.
    start_time = time.perf_counter()
    with pytest.raises(SeleneTimeoutError, match="Expired timers: 'per_result'"):
        shots = QsysResult(
            non_terminating_without_results.run_shots(
                Quest(random_seed=1234),
                n_qubits=1,
                n_shots=100,
                timeout=Timeout(
                    overall=datetime.timedelta(seconds=10),
                    per_shot=datetime.timedelta(seconds=5),
                    per_result=datetime.timedelta(seconds=1),
                ),
            )
        )
        print(shots)
    end_time = time.perf_counter()
    assert 1 <= end_time - start_time < 2, (
        "Per-result timeout should have taken around 1 second"
    )
