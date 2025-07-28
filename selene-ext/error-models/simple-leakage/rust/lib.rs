use anyhow::{Result, anyhow, bail};
use clap::Parser;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use selene_core::error_model::interface::ErrorModelInterfaceFactory;
use selene_core::error_model::{BatchResult, ErrorModelInterface};
use selene_core::export_error_model_plugin;
use selene_core::runtime::{BatchOperation, Operation};
use selene_core::simulator::{Simulator, SimulatorInterface};
use selene_core::utils::MetricValue;
use std::ffi::OsStr;

#[derive(Parser, Debug)]
struct Params {
    /// The probability of leakage on each operation
    #[arg(long)]
    p_leak: f64,
    /// The probability that a leaked qubit will measure as 1.
    #[arg(long)]
    leak_measurement_bias: f64,
}
#[derive(Default)]
struct Stats {
    spontaneous_leaks: u64,
    interaction_leaks: u64,
}

pub struct SimpleLeakageErrorModel {
    n_qubits: u64,
    rng: Pcg64Mcg,
    simulator: Simulator,
    leak_register: Vec<bool>,
    error_params: Params,
    stats: Stats,
}

impl SimpleLeakageErrorModel {
    fn is_leaked(&self, qubit: u64) -> Result<bool> {
        if qubit >= self.n_qubits {
            bail!(
                "Qubit ID {} is out of bounds for this error model with {} qubits",
                qubit,
                self.n_qubits
            );
        }
        Ok(self.leak_register[qubit as usize])
    }
    fn leak(&mut self, qubit: u64) -> Result<()> {
        if !self.is_leaked(qubit)? {
            self.leak_register[qubit as usize] = true;
        }
        Ok(())
    }
    fn maybe_leak(&mut self, qubit: u64) -> Result<()> {
        if self.is_leaked(qubit)? {
            // If the qubit is already in a leaked state, we don't do anything
            return Ok(());
        }
        if self.rng.random::<f64>() < self.error_params.p_leak {
            self.leak(qubit)?;
            self.stats.spontaneous_leaks += 1;
        }
        Ok(())
    }
    fn spread_leakage(&mut self, qubit_1: u64, qubit_2: u64) -> Result<()> {
        if self.is_leaked(qubit_1)? ^ self.is_leaked(qubit_2)? {
            self.leak(qubit_1)?;
            self.leak(qubit_2)?;
            self.stats.interaction_leaks += 1;
        }
        Ok(())
    }
}

impl ErrorModelInterface for SimpleLeakageErrorModel {
    fn shot_start(&mut self, shot_id: u64, seed: u64, simulator_seed: u64) -> Result<()> {
        self.rng = Pcg64Mcg::seed_from_u64(seed);
        self.simulator.shot_start(shot_id, simulator_seed)?;
        self.leak_register = vec![false; self.n_qubits as usize];
        self.stats = Stats::default();
        Ok(())
    }
    fn shot_end(&mut self) -> Result<()> {
        self.simulator.shot_end()?;
        Ok(())
    }

    fn exit(&mut self) -> Result<()> {
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
                    self.maybe_leak(qubit_id)?;
                    self.simulator.rxy(qubit_id, theta, phi)?;
                }
                Operation::RZGate { qubit_id, theta } => {
                    self.maybe_leak(qubit_id)?;
                    self.simulator.rz(qubit_id, theta)?;
                }
                Operation::RZZGate {
                    qubit_id_1,
                    qubit_id_2,
                    theta,
                } => {
                    self.maybe_leak(qubit_id_1)?;
                    self.maybe_leak(qubit_id_2)?;
                    self.spread_leakage(qubit_id_1, qubit_id_2)?;
                    self.simulator.rzz(qubit_id_1, qubit_id_2, theta)?;
                }
                Operation::Measure {
                    qubit_id,
                    result_id,
                } => {
                    let measurement = if self.leak_register[qubit_id as usize] {
                        self.rng
                            .random_bool(self.error_params.leak_measurement_bias)
                    } else {
                        self.simulator.measure(qubit_id)?
                    };
                    results.set_bool_result(result_id, measurement);
                }
                Operation::MeasureLeaked {
                    qubit_id,
                    result_id,
                } => {
                    let measurement = if self.leak_register[qubit_id as usize] {
                        2
                    } else {
                        self.simulator.measure(qubit_id)? as u64
                    };
                    results.set_u64_result(result_id, measurement);
                }
                Operation::Reset { qubit_id } => {
                    self.simulator.reset(qubit_id)?;
                    self.leak_register[qubit_id as usize] = false;
                }
                Operation::Custom { .. } => {
                    // Passively ignore custom operations
                }
            }
        }
        Ok(results)
    }

    fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        match nth_metric {
            0 => Ok(Some((
                "spontaneous_leaks".to_string(),
                MetricValue::U64(self.stats.spontaneous_leaks),
            ))),
            1 => Ok(Some((
                "interaction_leaks".to_string(),
                MetricValue::U64(self.stats.interaction_leaks),
            ))),
            _ => Ok(None),
        }
    }
    fn get_simulator_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        self.simulator.get_metric(nth_metric)
    }
}

#[derive(Default)]
pub struct SimpleLeakageErrorModelFactory;

impl ErrorModelInterfaceFactory for SimpleLeakageErrorModelFactory {
    type Interface = SimpleLeakageErrorModel;

    fn init(
        self: std::sync::Arc<Self>,
        n_qubits: u64,
        error_model_args: &[impl AsRef<str>],
        simulator_path: &impl AsRef<OsStr>,
        simulator_args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        match Params::try_parse_from(error_model_args.iter().map(|s| s.as_ref())) {
            Err(e) => Err(anyhow!(
                "Error parsing arguments to depolarizing error model plugin: {}",
                e
            )),
            Ok(params) => {
                let simulator =
                    Simulator::load_from_file(simulator_path, n_qubits, simulator_args)?;
                let leak_register = vec![false; n_qubits as usize];
                Ok(Box::new(SimpleLeakageErrorModel {
                    n_qubits,
                    rng: Pcg64Mcg::seed_from_u64(0),
                    simulator,
                    leak_register,
                    error_params: params,
                    stats: Stats::default(),
                }))
            }
        }
    }
}

export_error_model_plugin!(crate::SimpleLeakageErrorModelFactory);
