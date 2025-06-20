pub mod framework;
pub mod single_qubit;
pub mod two_qubit;

use crate::simulator::SimulatorInterfaceFactory;
use std::sync::Arc;

pub fn run_basic_tests(
    n_qubit_engine_generator: Arc<impl SimulatorInterfaceFactory + 'static>,
    args: Vec<String>,
) {
    single_qubit::single_qubit_operations(n_qubit_engine_generator.clone(), args.clone());
    two_qubit::two_qubit_operations(n_qubit_engine_generator.clone(), args.clone());
}
