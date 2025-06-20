use crate::ClassicalReplaySimulatorFactory;
use crate::arg_encoding::{encode_counts, encode_measurements};
use selene_core::simulator::conformance_testing::framework::TestFramework;
use std::sync::Arc;

#[test]
fn replay_test() {
    let interface = Arc::new(ClassicalReplaySimulatorFactory);
    let measurement_results = vec![false, true, false, true, false];
    let measurement_string = encode_measurements(&measurement_results);
    let count_string = encode_counts(vec![measurement_results.len()]);
    let args = vec![
        format!("lib"),
        format!("--counts={count_string}"),
        format!("--measurements={measurement_string}"),
    ];

    TestFramework::new(5)
        .h(0)
        .rxy(0, std::f64::consts::FRAC_PI_8, std::f64::consts::E)
        .test(100, vec![0, 1], |populations| {
            // if we were to measure the first two
            // qubits, we should always get false and true,
            // as according to the measurement_results vector
            // we provided to the replay simulator
            // [false, true] => bit 0 from the right is false, bit 1 from the right is true
            populations.test_measurements[0b10] == 100
        })
        .h(4)
        .test(100, vec![0, 1, 2, 3, 4], |populations| {
            // This is a separate test, so we should see the measurement
            // results come up as false, true, false, true, false, unaffected
            // by the previous test.
            populations.test_measurements[0b01010] == 100
        })
        .cnot(0, 1)
        .test(100, vec![0, 1, 2, 3], |populations| {
            // This is a separate test, so we should see the measurement
            // results come up as false, true, false, true, unaffected
            // by the previous test.
            populations.test_measurements[0b1010] == 100
        })
        .run(interface, args);
}
