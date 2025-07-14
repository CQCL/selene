use anyhow::{anyhow, Result};
use selene_core::error_model::interface::ErrorModelInterfaceFactory;
use selene_core::error_model::{BatchResult, ErrorModelInterface};
use selene_core::export_error_model_plugin;
use selene_core::runtime::{BatchOperation, Operation};
use selene_core::simulator::{Simulator, SimulatorInterface};
use selene_core::utils::MetricValue;
use std::ffi::OsStr;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use clap::Parser;


#[derive(Parser, Debug)]
struct Params {
    #[arg(long, default_value= "0.01")]
    flip_probability: f64,
    #[arg(long, default_value = "0.01")]
    angle_mutation: f64,
}

#[derive(Default, Debug)]
struct Stats {
    flips_induced: u64,
    total_angle_error: f64,
}


pub struct ExampleErrorModel {
    simulator: Simulator,
    rng: Pcg64Mcg,
    // Store the user-customisable properties for this instance
    error_params: Params,
    // We can gather statistics that will be reported back to the user
    // if they are using the MetricStore.
    stats: Stats,
}

impl ExampleErrorModel {
    fn mutate_angle(&mut self, angle: f64) -> f64 {
        // Mutate the angle by a small random amount
        let offset = self.rng.random_range(-self.error_params.angle_mutation..self.error_params.angle_mutation);
        self.stats.total_angle_error += offset.abs();
        angle + offset
    }
    fn should_flip(&mut self) -> bool {
        // Randomly decide whether to flip a qubit in the computational basis
        self.rng.random_bool(self.error_params.flip_probability)
    }
    fn flip_qubit(&mut self, qubit_id: u64) -> Result<()> {
        // Flip the qubit in the computational basis
        self.simulator.rxy(qubit_id, std::f64::consts::PI, 0.0)?;
        self.stats.flips_induced += 1;
        Ok(())
    }
}

impl ErrorModelInterface for ExampleErrorModel {
    fn exit(&mut self) -> Result<()> {
        Ok(())
    }
    fn shot_start(&mut self, shot_id: u64, error_model_seed: u64, simulator_seed: u64) -> Result<()> {
        self.simulator.shot_start(shot_id, simulator_seed)?;
        self.rng = Pcg64Mcg::seed_from_u64(error_model_seed);
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
                    // An RXY gate has been requested.
                    //
                    // randomly mutate theta and phi
                    let theta = self.mutate_angle(theta);
                    let phi = self.mutate_angle(phi);
                    // Apply the RXY gate
                    self.simulator.rxy(qubit_id, theta, phi)?;
                    // randomly flip the qubit in the computational basis
                    if self.should_flip() {
                        self.flip_qubit(qubit_id)?;
                    }
                }
                Operation::RZZGate {
                    qubit_id_1,
                    qubit_id_2,
                    theta,
                } => {
                    // An RZZ gate has been requested.
                    //
                    // randomly mutate theta
                    let theta = self.mutate_angle(theta);
                    // apply the RZZ gate
                    self.simulator.rzz(qubit_id_1, qubit_id_2, theta)?;
                    // randomly flip both qubits in the computational basis
                    if self.should_flip() {
                        self.flip_qubit(qubit_id_1)?;
                        self.flip_qubit(qubit_id_2)?;
                    }
                }
                Operation::RZGate { qubit_id, theta } => {
                    // An RZ gate has been requested.
                    //
                    // randomly mutate theta
                    let theta = self.mutate_angle(theta);
                    // apply the RZ gate
                    self.simulator.rz(qubit_id, theta)?;
                    // randomly flip the qubit in the computational basis
                    if self.should_flip() {
                        self.flip_qubit(qubit_id)?;
                    }
                }
                Operation::Measure {
                    qubit_id,
                    result_id,
                } => {
                    // A measurement has been requested.
                    // We need to perform the measurement and store the result.
                    let measurement = self.simulator.measure(qubit_id)?;
                    // But wait! Let's randomly flip the measurement result with a small
                    // probability.
                    let measurement = if self.should_flip() {
                        self.stats.flips_induced += 1;
                        !measurement
                    } else {
                        measurement
                    };
                    results.set_bool_result(result_id, measurement);
                }
                Operation::MeasureLeaked {
                    qubit_id,
                    result_id,
                } => {
                    // We aren't modelling leakage so let's resolve this to
                    // a normal measurement following the same logic as with Operation::Measure
                    let measurement = self.simulator.measure(qubit_id)?;
                    let measurement = if self.should_flip() {
                        self.stats.flips_induced += 1;
                        !measurement
                    } else {
                        measurement
                    };
                    results.set_u64_result(result_id, if measurement { 1 } else { 0 });
                }
                Operation::Reset { qubit_id } => {
                    // A reset has been requested.
                    self.simulator.reset(qubit_id)?;
                    // So ideally it is in |0> now. Let's flip it with a small probability.
                    if self.should_flip() {
                        self.flip_qubit(qubit_id)?;
                    }
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
            0 => {
                // Return the number of flips induced by the error model
                Ok(Some(("flips_induced".to_string(), MetricValue::U64(self.stats.flips_induced))))
            }
            1 => {
                // Return the total angle error induced by the error model
                Ok(Some(("total_angle_error".to_string(), MetricValue::F64(self.stats.total_angle_error))))
            }
            2 => {
                // No other metrics are defined. Note this is NOT an error. We are
                // simply reporting to Selene that we are at the end of the metric list.
                Ok(None)
            }
            _ => {
                // Selene should not be requesting another metric. This shouldn't happen,
                // and you shouldn't need to handle this, but it's an example of how to provide
                // errors through this function.
                Err(anyhow!("Selene requested an out of bounds metric: {}", nth_metric))
            }
        }
    }

    fn get_simulator_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        // We passively forward the request for simulator metrics to the underlying simulator.
        self.simulator.get_metric(nth_metric)
    }

    fn dump_simulator_state(&mut self, file: &std::path::Path, qubits: &[u64]) -> Result<()> {
        // We passively forward the request to dump the simulator state to the underlying
        // simulator. As this error model isn't manipulating the simulator state beyond
        // immediate flips and gate mutations, this is fine.
        //
        // If instead we were managing some extension of simulator state, e.g. leaked qubits
        // or accumulated phase through the error model, we should not support state dumping
        // unless we have considered the consequences of reporting potentially incorrect state
        // to the end user.
        self.simulator.dump_state(file, qubits)
    }
}

#[derive(Default)]
pub struct ExampleErrorModelFactory;

impl ErrorModelInterfaceFactory for ExampleErrorModelFactory {
    type Interface = ExampleErrorModel;

    fn init(
        self: std::sync::Arc<Self>,
        n_qubits: u64,
        error_model_args: &[impl AsRef<str>],
        simulator_path: &impl AsRef<OsStr>,
        simulator_args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        match Params::try_parse_from(error_model_args.iter().map(|s| s.as_ref())) {
            Err(e) => Err(anyhow!(
                "Error parsing arguments to the example error model plugin: {}",
                e
            )),
            Ok(params) => {
                let simulator =
                    Simulator::load_from_file(simulator_path, n_qubits, simulator_args)?;
                Ok(Box::new(ExampleErrorModel {
                    rng: Pcg64Mcg::seed_from_u64(0),
                    simulator,
                    error_params: params,
                    stats: Stats::default(),
                }))
            }
        }
    }
}

export_error_model_plugin!(crate::ExampleErrorModelFactory);
