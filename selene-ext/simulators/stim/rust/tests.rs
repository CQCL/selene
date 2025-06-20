use crate::StimSimulatorFactory;
use selene_core::simulator::conformance_testing::run_basic_tests;
use std::sync::Arc;
#[test]
fn basic_conformance_test() {
    let interface = Arc::new(StimSimulatorFactory);
    let args = vec!["".to_string(), "--angle-threshold=0.001".to_string()];
    run_basic_tests(interface, args);
}
