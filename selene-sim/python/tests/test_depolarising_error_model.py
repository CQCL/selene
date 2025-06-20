from guppylang.decorator import guppy
from guppylang.std.quantum import qubit, measure, h, cx, x, y
from guppylang.std.builtins import result
from hugr.qsystem.result import QsysResult
import random
import yaml

from selene_sim import Quest, ClassicalReplay
from selene_sim.build import build
from selene_sim.backends import DepolarizingErrorModel


# given some shot results, we want a nice dict from
# (c1 value, c2 value) => count
# requires 2 qubit runs, results output as c1, c2
def count_occurances(shots: QsysResult) -> dict:
    counts = {(False, False): 0, (False, True): 0, (True, False): 0, (True, True): 0}
    for results in shots.results:
        outcomes = dict(results.as_dict())
        c1, c2 = outcomes["c1"], outcomes["c2"]
        counts[(c1, c2)] += 1
    return counts


def test_measurement_error(snapshot):
    @guppy
    def main() -> None:
        q1: qubit = qubit()
        q2: qubit = qubit()
        h(q1)
        h(q2)
        result("c1", measure(q1))
        result("c2", measure(q2))

    runner = build(guppy.compile(main), "measurement_error")
    error_model = DepolarizingErrorModel(
        random_seed=12478918,  # for reproducibility
        p_init=0,  # constant zero for this test
        p_meas=0,  # begin with no errors (mutate during the test)
        p_1q=0,  # constant zero for this test
        p_2q=0,  # constant zero for this test
    )
    # each test will run through 1000 shots. Let's use a ClassicalReplay
    # simulator to provide exactly which measurements will be provided
    # to the error model from the simulator. In this case, we know the
    # ideal, pre-error simulator will provide:
    #  - [False, False] on 230 shots,
    #  - [False, True] on 260 shots
    #  - [True, False] on 300 shots
    #  - [True, True] on 210 shots
    measurements = (
        [[False, False]] * 230
        + [[False, True]] * 260
        + [[True, False]] * 300
        + [[True, True]] * 210
    )
    random.seed(789124)
    random.shuffle(measurements)
    simulator = ClassicalReplay(measurements=measurements)

    # now let's run through some p_meas values.
    def get_counts(error_model):
        return count_occurances(
            QsysResult(
                runner.run_shots(
                    simulator=simulator,
                    error_model=error_model,
                    n_qubits=2,
                    n_shots=len(measurements),
                )
            )
        )

    error_model.p_meas = 0
    counts = get_counts(error_model)
    # no snapshot needed: we've predetermined the counts
    assert counts[(False, False)] == 230
    assert counts[(False, True)] == 260
    assert counts[(True, False)] == 300
    assert counts[(True, True)] == 210

    # and now a 1% measurement error
    # note that although the only *apparent* change is that
    # a (False, True) becomes a (True, True), there are actually 18
    # changes, but they cross-polinate. We will demonstrate this
    # at the end of the test.
    error_model.p_meas = 0.01
    counts = get_counts(error_model)
    results = runner.run_shots(
        simulator=simulator,
        error_model=error_model,
        n_qubits=2,
        n_shots=len(measurements),
    )
    snapshot.assert_match(yaml.dump(counts), "counts_pmeas_1pc")

    # now a 25% measurement error
    # This blends the results up significantly. The (False, False) measurements are far less prevalent.
    error_model.p_meas = 0.25
    counts = get_counts(error_model)
    snapshot.assert_match(yaml.dump(counts), "counts_pmeas_25pc")

    # now let's introduce a 50% measurement error. At this point the outcomes are purely random.
    error_model.p_meas = 0.5
    counts = get_counts(error_model)
    snapshot.assert_match(yaml.dump(counts), "counts_pmeas_50pc")

    # now let's introduce MAXIMUM measurement error. This maps the ideal measurements (X, Y) to
    # erroneous measurements (!X, !Y)
    error_model.p_meas = 1
    counts = get_counts(error_model)
    snapshot.assert_match(yaml.dump(counts), "counts_pmeas_100pc")

    # Now, back to that 1% measurement error. Let's demonstrate the cross-pollination.
    #
    # There are two ways we can demonstrate this. The first is trivial, because we
    # know the exact measurements that the simulator is providing. We can step through
    # the results and count the number of errors.
    #
    # This is handy for the purposes of this test, but not in general. We don't
    # always know the simulator's measurements, so we need some way of gathering
    # metrics from the error model itself. We can do this with the event_hook in
    # run_shots: by passing a MetricStore, we can gather statistics on the user
    # program, the runtime optimiser, the post-optimiser output, the error model,
    # and the simulator itself - as long as the relevant plugins provide metrics.

    error_model.p_meas = 0.01
    from selene_sim.event_hooks import MetricStore

    metric_store = MetricStore()
    results = list(
        dict(x)
        for x in runner.run_shots(
            simulator=simulator,
            error_model=error_model,
            n_qubits=2,
            n_shots=len(measurements),
            event_hook=metric_store,
        )
    )

    # Approach 1: step through the ideal and actual results, count the errors.
    errors = 0
    for ideal, erroneous in zip(measurements, results):
        if ideal[0] != bool(erroneous["c1"]):
            errors += 1
        if ideal[1] != bool(erroneous["c2"]):
            errors += 1
    snapshot.assert_match(yaml.dump(errors), "measurement_errors")

    # Approach 2: use the MetricStore to gather the same information.
    assert len(metric_store.shots) == 1000
    total_measurement_errors = 0
    for metrics in metric_store.shots:
        total_measurement_errors += metrics["error_model"]["measurement_errors"]
    assert total_measurement_errors == errors


def test_init_error(snapshot):
    @guppy
    def main() -> None:
        q1: qubit = qubit()
        q2: qubit = qubit()
        # note: no gates
        result("c1", measure(q1))
        result("c2", measure(q2))

    runner = build(guppy.compile(main), "init_error")

    simulator = Quest(random_seed=12472461)
    error_model = DepolarizingErrorModel(
        random_seed=81257612,  # for reproducibility
        p_init=0,  # mutate during the test
        p_meas=0,  # constant zero for this test
        p_1q=0,  # constant zero for this test
        p_2q=0,  # constant zero for this test
    )
    # each test will run through 1000 shots. We'll use a Quest backend
    # for this test, as classical replay would not be affected by the
    # injected X gate in the presence of an init error.

    # now let's run through some p_init values.
    # first, no errors
    error_model.p_init = 0
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_ideal")

    # and now a 1% initialization error
    # note that (False, False) is still highly prevalent.
    error_model.p_init = 0.01
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_pinit_1pc")

    # now a 25% initialization error
    # This blends the results up significantly. The (False, False) measurements still dominate,
    # but they're not as prevalent.
    error_model.p_init = 0.25
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_pinit_25pc")

    # now let's introduce a 50% initialization error. At this point the outcomes are purely random.
    error_model.p_init = 0.5
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_pinit_50pc")

    # now let's introduce MAXIMUM initialization error.
    # This maps the ideal measurements (X, Y) to erroneous measurements (!X, !Y)
    error_model.p_init = 1
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_pinit_100pc")


def test_1q_error(snapshot):
    @guppy
    def main() -> None:
        q1: qubit = qubit()
        q2: qubit = qubit()
        # 1 1q gate on q1
        x(q1)
        # 2 1q gates on q2
        y(q2)
        y(q2)
        # some self-cancelling 2q gates just to demonstrate that
        # they aren't affected by q1 errors
        cx(q1, q2)
        cx(q1, q2)
        result("c1", measure(q1))
        result("c2", measure(q2))

    runner = build(guppy.compile(main), "init_error")

    simulator = Quest(random_seed=75264817)
    error_model = DepolarizingErrorModel(
        random_seed=81257612,  # for reproducibility
        p_init=0,  # constant zero for this test
        p_meas=0,  # constant zero for this test
        p_1q=0,  # mutate during the test
        p_2q=0,  # constant zero for this test
    )
    # each test will run through 1000 shots. We'll use a Quest backend
    # for this test, as classical replay would not be affected by the
    # injected pauli in the presence of a 1q error.

    # now let's run through some p_1q values.
    # first, no error
    error_model.p_1q = 0
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_ideal")

    # and now a 1% 1q error
    # note that the only change is one (False, True) becoming a (True, True)
    error_model.p_1q = 0.01
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_p1q_1pc")

    # now a 25% 1q error
    # This blends the results up significantly. The (False, False) measurements are far less prevalent.
    error_model.p_1q = 0.25
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_p1q_25pc")

    # now let's introduce a 50% 1q error. At this point the outcomes are purely random.
    error_model.p_1q = 0.5
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_p1q_50pc")

    # now let's introduce MAXIMUM 1q error. This maps the ideal measurements (X, Y) to
    # erroneous measurements (!X, !Y)
    error_model.p_1q = 1
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_p1q_100pc")


def test_2q_error(snapshot):
    @guppy
    def main() -> None:
        q1: qubit = qubit()
        q2: qubit = qubit()
        # 1 1q gate on q1 - should not be impacted by error
        x(q1)
        # 2 1q gates on q2 - should not be impacted by error
        y(q2)
        y(q2)
        # some self-cancelling 2q gates - should be impacted by error
        cx(q1, q2)
        cx(q1, q2)
        result("c1", measure(q1))
        result("c2", measure(q2))

    runner = build(guppy.compile(main), "init_error")

    simulator = Quest(random_seed=75264817)
    error_model = DepolarizingErrorModel(
        random_seed=12376125,  # for reproducibility
        p_init=0,  # constant zero for this test
        p_meas=0,  # constant zero for this test
        p_1q=0,  # constant zero for this test
        p_2q=0,  # mutate during the test
    )
    # each test will run through 1000 shots. We'll use a Quest backend
    # for this test, as classical replay would not be affected by the
    # injected paulis in the presence of a 2q error.

    # now let's run through some p_2q values.

    error_model.p_2q = 0
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_ideal")

    # and now a 1% 2q error
    # note that the only change is one (False, True) becoming a (True, True)
    error_model.p_2q = 0.01
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_p2q_1pc")

    # now a 25% 2q error
    # This blends the results up significantly. The (False, False) measurements are far less prevalent.
    error_model.p_2q = 0.25
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_p2q_25pc")

    # now let's introduce a 50% 2q error. At this point the outcomes are purely random.
    error_model.p_2q = 0.5
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_p2q_50pc")

    # now let's introduce MAXIMUM 2q error. This maps the ideal measurements (X, Y) to
    # erroneous measurements (!X, !Y)
    error_model.p_2q = 1
    shots = QsysResult(
        runner.run_shots(
            simulator=simulator, error_model=error_model, n_qubits=2, n_shots=1000
        )
    )
    counts = count_occurances(shots)
    snapshot.assert_match(yaml.dump(counts), "counts_p2q_100pc")
