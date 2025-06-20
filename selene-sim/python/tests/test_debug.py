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
    assert state.get_dirac_notation()[0]["probability"] == 1


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
        assert state.get_state_vector_distribution()[0]["probability"] == 1


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
        assert state.get_state_vector_distribution()[0]["probability"] == 1
