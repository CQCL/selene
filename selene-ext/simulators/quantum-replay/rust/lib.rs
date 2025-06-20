use anyhow::{Result, anyhow};
use clap::Parser;
use selene_core::export_simulator_plugin;
use selene_core::simulator::interface::SimulatorInterfaceFactory;
use selene_core::simulator::{Simulator, SimulatorInterface};
use selene_core::utils::MetricValue;

mod arg_encoding;
use arg_encoding::parse_measurement_strings;

#[derive(Parser, Debug)]
struct Params {
    /// The path to a wrapped simulator plugin to drive with postselection
    #[arg(long)]
    wrapped_path: String,
    /// Arguments for the inner simulator plugin
    #[arg(long)]
    wrapped_arg: Vec<String>,
    /// A base64-encoded string representing a list of little-endian 32 bit
    /// unsigned integers, with element `n` representing the number of measurements
    /// performed on shot `n`.
    #[arg(long)]
    counts: String,
    /// A base64-encoded string representing the measurements (as bits) across the
    /// entire run. The number of measurements to associate with shot `n` is given
    /// by the `n`th element of `counts`.
    #[arg(long)]
    measurements: String,
    /// Whether to switch from postselection to measurement after user-provided
    /// measurement outcomes have been consumed (on a per-shot basis).
    ///
    /// Measurement within the user-provided outcomes is performed with postselection,
    /// and once all user-provided outcomes have been consumed, the user may wish for
    /// further measurements to be an error (i.e. the program should have finished but
    /// did not), and this can be achieved by setting `resume_with_measurement` to False.
    /// On the other hand, they may wish to switch to performing actual measurements on
    /// the simulator, and this can be achieved by setting `resume_with_measurement` to True.
    #[arg(long)]
    resume_with_measurement: bool,
}

pub struct QuantumReplaySimulator {
    wrapped: Simulator,
    n_qubits: u64,
    resume_with_measurement: bool,
    current_shot: u64,
    next_postselection_index: usize,
    current_shot_postselections: Vec<bool>,
    all_shot_postselections: Vec<Vec<bool>>,
    measurements_performed: usize,
}

impl SimulatorInterface for QuantumReplaySimulator {
    fn exit(&mut self) -> Result<()> {
        Ok(())
    }
    fn shot_start(&mut self, shot_id: u64, seed: u64) -> Result<()> {
        if shot_id > self.all_shot_postselections.len() as u64 {
            return Err(anyhow!(
                "Shot ID {shot_id} is out of bounds. The maximum shot ID is {}.",
                self.all_shot_postselections.len()
            ));
        }
        self.current_shot = shot_id;
        self.current_shot_postselections = self.all_shot_postselections[shot_id as usize].clone();
        self.wrapped.shot_start(shot_id, seed)?;
        self.next_postselection_index = 0;
        self.measurements_performed = 0;
        Ok(())
    }
    fn shot_end(&mut self) -> Result<()> {
        if self.next_postselection_index < self.current_shot_postselections.len() {
            Err(anyhow!(
                "ERROR: Not all measurements were consumed on shot {}: {} measurements remain.",
                self.current_shot,
                self.current_shot_postselections.len()
            ))
        } else {
            self.current_shot_postselections.clear();
            self.next_postselection_index = 0;
            self.measurements_performed = 0;
            self.wrapped.shot_end()
        }
    }

    fn rxy(&mut self, q0: u64, theta: f64, phi: f64) -> Result<()> {
        if q0 < self.n_qubits {
            self.wrapped.rxy(q0, theta, phi)
        } else {
            Err(anyhow!(
                "RXY(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        }
    }

    fn rzz(&mut self, q0: u64, q1: u64, theta: f64) -> Result<()> {
        if q0 < self.n_qubits && q1 < self.n_qubits {
            self.wrapped.rzz(q0, q1, theta)
        } else {
            Err(anyhow!(
                "RZZ(q0={q0}, q1={q1}) is out of bounds. q0 and q1 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        }
    }

    fn rz(&mut self, q0: u64, theta: f64) -> Result<()> {
        if q0 < self.n_qubits {
            self.wrapped.rz(q0, theta)
        } else {
            Err(anyhow!(
                "RZ(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        }
    }

    fn measure(&mut self, q0: u64) -> Result<bool> {
        if q0 >= self.n_qubits {
            Err(anyhow!(
                "Measure(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        } else if self.next_postselection_index < self.current_shot_postselections.len() {
            // postselect
            let measurement = self.current_shot_postselections[self.next_postselection_index];
            self.next_postselection_index += 1;
            self.wrapped.postselect(q0, measurement)?;
            Ok(measurement)
        } else if self.resume_with_measurement {
            // measure
            self.measurements_performed += 1;
            self.wrapped.measure(q0)
        } else {
            // exhausted predefined measurements, resume_with_measurement is false
            Err(anyhow!(
                "All measurements have been consumed, and resume_with_measurement is set to false. If you intend to switch to measurements after postselection measurements have been consumed, set resume_with_measurement to true."
            ))
        }
    }

    fn reset(&mut self, q0: u64) -> Result<()> {
        if q0 >= self.n_qubits {
            Err(anyhow!(
                "Reset(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        } else {
            self.wrapped.reset(q0)
        }
    }
    fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        match nth_metric {
            0 => Ok(Some((
                "postselections_performed".to_string(),
                MetricValue::U64(self.next_postselection_index as u64),
            ))),
            1 => Ok(Some((
                "measurements_performed".to_string(),
                MetricValue::U64(self.measurements_performed as u64),
            ))),
            n => self.wrapped.get_metric(n - 2),
        }
    }
}

#[derive(Default)]
pub struct QuantumReplaySimulatorFactory;

impl SimulatorInterfaceFactory for QuantumReplaySimulatorFactory {
    type Interface = QuantumReplaySimulator;

    fn init(
        self: std::sync::Arc<Self>,
        n_qubits: u64,
        args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        let args: Vec<String> = args.iter().map(|s| s.as_ref().to_string()).collect();
        match Params::try_parse_from(args) {
            Err(e) => Err(anyhow!(
                "Error parsing arguments to quantum replay plugin: {e}"
            )),
            Ok(params) => match parse_measurement_strings(params.measurements, params.counts) {
                Err(e) => Err(anyhow!("Error parsing measurements: {e}")),
                Ok(measurements) => {
                    let wrapped = Simulator::load_from_file(
                        &params.wrapped_path,
                        n_qubits,
                        params.wrapped_arg.as_ref(),
                    )?;
                    Ok(Box::new(QuantumReplaySimulator {
                        wrapped,
                        n_qubits,
                        resume_with_measurement: params.resume_with_measurement,
                        current_shot: 0,
                        next_postselection_index: 0,
                        current_shot_postselections: Vec::default(),
                        all_shot_postselections: measurements,
                        measurements_performed: 0,
                    }))
                }
            },
        }
    }
}

export_simulator_plugin!(crate::QuantumReplaySimulatorFactory);
