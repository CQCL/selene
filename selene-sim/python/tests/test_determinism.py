import yaml
from guppylang.decorator import guppy
from guppylang.std.quantum import qubit, h, measure_array
from guppylang.std.builtins import result, array
from guppylang.std.qsystem.random import RNG
from guppylang.std.qsystem.utils import get_current_shot

from selene_sim.build import build
from selene_sim import Stim, DepolarizingErrorModel


def test_repetition(snapshot):
    @guppy
    def main() -> None:
        qubits = array(qubit() for _ in range(5))
        for i in range(len(qubits)):
            h(qubits[i])
        bits = measure_array(qubits)
        result("a", bits[0])
        result("b", bits[1])
        result("c", bits[2])
        result("d", bits[3])
        result("e", bits[4])
        result("shot", get_current_shot())
        rng = RNG(get_current_shot())
        result("random_int", rng.random_int())
        result("random_float", rng.random_float())
        rng.discard()

    runner = build(guppy.compile(main), "no_results")

    error_model = DepolarizingErrorModel(
        random_seed=194678,
        p_init=0.01,
        p_meas=0.01,
        p_1q=0.01,
        p_2q=0.01,
    )

    simulator = Stim(random_seed=561278)

    full_results = list(
        dict(a)
        for a in runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=5, n_shots=1000
        )
    )

    # ensure results agree with the snapshot
    snapshot.assert_match(yaml.dump(full_results), "full_results")

    full_results_again = list(
        dict(a)
        for a in runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=5, n_shots=1000
        )
    )

    assert full_results == full_results_again

    offset_results = list(
        dict(a)
        for a in runner.run_shots(
            simulator=simulator,
            error_model=error_model,
            n_qubits=5,
            n_shots=600,  # note: only running 600 shots
            shot_offset=400,  # note: offsetting by 400
        )
    )

    assert full_results[400:] == offset_results

    results_ending_in_5 = list(
        dict(a)
        for a in runner.run_shots(
            simulator=simulator,
            error_model=error_model,
            n_qubits=5,
            n_shots=100,
            shot_offset=5,
            shot_increment=10,  # note: incrementing by 10
        )
    )

    assert full_results[5::10] == results_ending_in_5

    last_result = dict(
        runner.run(
            simulator=simulator,
            error_model=error_model,
            n_qubits=5,
            shot_offset=999,
        )
    )

    assert full_results[-1] == last_result

    # now try some multi-process runs
    results_ending_in_5_but_with_5_processes = list(
        dict(a)
        for a in runner.run_shots(
            simulator=simulator,
            error_model=error_model,
            n_qubits=5,
            n_shots=100,
            shot_offset=5,
            shot_increment=10,  # note: incrementing by 10
            n_processes=5,
        )
    )

    assert results_ending_in_5_but_with_5_processes == results_ending_in_5


if __name__ == "__main__":
    test_repetition()
