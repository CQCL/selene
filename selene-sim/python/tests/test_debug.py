import pytest

from guppylang import guppy
from guppylang.std.builtins import array
from guppylang.std.debug import state_result
from guppylang.std.quantum import discard, qubit, discard_array, x
from hugr.qsystem.result import QsysResult

from selene_sim.build import build
from selene_sim import Quest


def test_initial_state():
    @guppy
    def main() -> None:
        q0 = qubit()
        state_result("initial_state", q0)
        discard(q0)

    runner = build(guppy.compile(main), "initial_state")
    got = runner.run(Quest(), n_qubits=1)
    state = Quest.extract_states_dict(got)["initial_state"]
    assert state.get_dirac_notation()[0].probability == 1


def test_array_state():
    @guppy
    def main() -> None:
        qs = array(qubit() for _ in range(2))
        for i in range(2):
            x(qs[i])
        state_result("array_state", qs)
        discard_array(qs)

    runner = build(guppy.compile(main), "array_state")
    plugin = Quest()
    shots = QsysResult(
        runner.run_shots(
            simulator=plugin,
            n_qubits=2,
            n_shots=2,
        )
    )
    for shot in shots.results:
        state = Quest.extract_states_dict(shot.entries)["array_state"]
        assert state.get_density_matrix()[3][3] == 1
        assert len(state.get_single_state()) == 4


def test_array_subscript_state():
    @guppy
    def main() -> None:
        qs = array(qubit() for _ in range(2))
        for i in range(2):
            x(qs[i])
        state_result("array_state", qs[0])
        discard_array(qs)

    runner = build(guppy.compile(main), "array_state")
    plugin = Quest()
    shots = QsysResult(
        runner.run_shots(
            simulator=plugin,
            n_qubits=2,
            n_shots=1,
        )
    )
    for shot in shots.results:
        state = Quest.extract_states_dict(shot.entries)["array_state"]
        assert state.get_density_matrix()[1][1] == 1
        assert state.get_state_vector_distribution()[0].probability == 1


@pytest.mark.parametrize("cleanup_param", [True, False, None])
def test_state_cleanup(cleanup_param):
    @guppy
    def main() -> None:
        qs = array(qubit() for _ in range(2))
        for i in range(2):
            x(qs[i])
        state_result("array_state", qs[0])
        discard_array(qs)

    runner = build(guppy.compile(main), "array_state")
    plugin = Quest()
    shots = QsysResult(
        runner.run_shots(
            simulator=plugin,
            n_qubits=2,
            n_shots=1,
        )
    )
    run_directory = runner.runs
    individual_runs = list(run_directory.iterdir())
    assert len(individual_runs) == 1, "Expected only one run directory"
    individual_run = individual_runs[0]
    run_artifact_dir = individual_run / "artifacts"
    assert run_artifact_dir.exists(), "Expected artifacts directory to exist"
    state_files_pre_parse = list(run_artifact_dir.glob("*.state"))
    assert len(state_files_pre_parse) == 1

    for shot in shots.results:
        state_dict = (
            Quest.extract_states_dict(shot.entries)
            if cleanup_param is None
            else Quest.extract_states_dict(shot.entries, cleanup=cleanup_param)
        )
        state = state_dict["array_state"]
        assert state.get_density_matrix()[1][1] == 1
        assert state.get_state_vector_distribution()[0].probability == 1
        state_files_post_parse = list(run_artifact_dir.glob("*.state"))
        if cleanup_param in [True, None]:
            assert len(state_files_post_parse) == 0, (
                "Expected no state files after cleanup"
            )
        else:
            assert state_files_post_parse == state_files_pre_parse, (
                "Expected artifacts to remain unchanged if cleanup is False"
            )
