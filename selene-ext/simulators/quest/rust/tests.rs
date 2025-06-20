use crate::QuestSimulatorFactory;
use selene_core::simulator::conformance_testing::run_basic_tests;
use std::sync::Arc;
#[test]
fn basic_conformance_test() {
    let interface = Arc::new(QuestSimulatorFactory);
    let args = vec![];
    run_basic_tests(interface, args);
}
