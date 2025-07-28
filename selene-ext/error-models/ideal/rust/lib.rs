use anyhow::{Result, anyhow};
use selene_core::error_model::interface::ErrorModelInterfaceFactory;
use selene_core::error_model::{BatchResult, ErrorModelInterface};
use selene_core::export_error_model_plugin;
use selene_core::runtime::{BatchOperation, Operation};
use selene_core::simulator::{Simulator, SimulatorInterface};
use selene_core::utils::MetricValue;
use std::ffi::OsStr;

pub struct IdealErrorModel {
    simulator: Simulator,
}

impl ErrorModelInterface for IdealErrorModel {
    fn exit(&mut self) -> Result<()> {
        Ok(())
    }
    fn shot_start(&mut self, shot_id: u64, _seed: u64, simulator_seed: u64) -> Result<()> {
        self.simulator.shot_start(shot_id, simulator_seed)?;
        Ok(())
    }
    fn shot_end(&mut self) -> Result<()> {
        self.simulator.shot_end()?;
        Ok(())
    }

    fn handle_operations(&mut self, operations: BatchOperation) -> Result<BatchResult> {
        let mut results = BatchResult::default();
        for op in operations {
            match op {
                Operation::RXYGate {
                    qubit_id,
                    theta,
                    phi,
                } => {
                    self.simulator.rxy(qubit_id, theta, phi)?;
                }
                Operation::RZZGate {
                    qubit_id_1,
                    qubit_id_2,
                    theta,
                } => {
                    self.simulator.rzz(qubit_id_1, qubit_id_2, theta)?;
                }
                Operation::RZGate { qubit_id, theta } => {
                    self.simulator.rz(qubit_id, theta)?;
                }
                Operation::Measure {
                    qubit_id,
                    result_id,
                } => {
                    let measurement = self.simulator.measure(qubit_id)?;
                    results.set_bool_result(result_id, measurement);
                }
                Operation::MeasureLeaked {
                    qubit_id,
                    result_id,
                } => {
                    // In this ideal model, there's no leakage.
                    // Just do a normal measurement and stick to the [0,1] range
                    let measurement = self.simulator.measure(qubit_id)?;
                    results.set_u64_result(result_id, measurement.into());
                }

                Operation::Reset { qubit_id } => {
                    self.simulator.reset(qubit_id)?;
                }
                Operation::Custom { .. } => {
                    // Passively ignore custom operations
                }
            }
        }
        Ok(results)
    }

    fn get_metric(&mut self, _nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        Ok(None)
    }

    fn get_simulator_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        self.simulator.get_metric(nth_metric)
    }

    fn dump_simulator_state(&mut self, file: &std::path::Path, qubits: &[u64]) -> Result<()> {
        self.simulator.dump_state(file, qubits)
    }
}

#[derive(Default)]
pub struct IdealErrorModelFactory;

impl ErrorModelInterfaceFactory for IdealErrorModelFactory {
    type Interface = IdealErrorModel;

    fn init(
        self: std::sync::Arc<Self>,
        n_qubits: u64,
        error_model_args: &[impl AsRef<str>],
        simulator_path: &impl AsRef<OsStr>,
        simulator_args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        if error_model_args.len() > 1 {
            return Err(anyhow!(
                "Invalid number of arguments to ideal error model plugin"
            ));
        }
        let simulator_args: Vec<String> = simulator_args
            .iter()
            .map(|x| x.as_ref().to_string())
            .collect();

        let simulator = Simulator::load_from_file(simulator_path, n_qubits, &simulator_args)?;
        Ok(Box::new(IdealErrorModel { simulator }))
    }
}

export_error_model_plugin!(crate::IdealErrorModelFactory);
