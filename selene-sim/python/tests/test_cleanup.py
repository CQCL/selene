import pytest

from guppylang import guppy
from guppylang.std.quantum import discard, qubit

from selene_sim.build import build
from selene_sim import Quest


def test_delete_files():
    @guppy
    def main() -> None:
        q0 = qubit()
        discard(q0)

    runner = build(main.compile())
    got = list(runner.run(Quest(), n_qubits=1))
    runner.delete_files()
    with pytest.raises(FileNotFoundError):
        got = list(runner.run(Quest(), n_qubits=1))
    assert not runner.root.exists()
    # these are included in case we later decide to move files
    # out of the build root. In this case we may have neglected
    # to clean them up.
    #
    # If at the time this is intentional behaviour,
    # then we can change this test.
    assert not runner.executable.exists()
    assert not runner.runs.exists()
    assert not runner.artifacts.exists()


def test_delete_run_directories():
    @guppy
    def main() -> None:
        q0 = qubit()
        discard(q0)

    runner = build(main.compile())
    got = list(runner.run(Quest(), n_qubits=1))
    assert len(list(runner.runs.iterdir())) == 1

    runner.delete_run_directories()
    # we are only cleaning up runs, so everything needed
    # for creating new runs should still work.
    assert runner.root.exists()
    assert runner.artifacts.exists()
    assert runner.runs.exists()
    assert len(list(runner.runs.iterdir())) == 0

    # this should not fail
    got = list(runner.run(Quest(), n_qubits=1))

    # and we should see one run dir
    assert len(list(runner.runs.iterdir())) == 1
