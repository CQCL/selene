use anyhow::{Result, anyhow};
use clap::Parser;
use selene_core::export_simulator_plugin;
use selene_core::simulator::SimulatorInterface;
use selene_core::simulator::interface::SimulatorInterfaceFactory;
use selene_core::utils::MetricValue;

mod arg_encoding;
use arg_encoding::parse_measurement_strings;
#[cfg(test)]
mod tests;

#[derive(Parser, Debug)]
struct Params {
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
}

pub struct ClassicalReplaySimulator {
    n_qubits: u64,
    current_shot: u64,
    next_measurement_index: usize,
    current_shot_measurements: Vec<bool>,
    all_shot_measurements: Vec<Vec<bool>>,
}

impl SimulatorInterface for ClassicalReplaySimulator {
    fn exit(&mut self) -> Result<()> {
        Ok(())
    }
    fn shot_start(&mut self, shot_id: u64, _seed: u64) -> Result<()> {
        // Ensure the shot ID corresponds to a set of provided measurements.
        if shot_id > self.all_shot_measurements.len() as u64 {
            return Err(anyhow!(
                "Shot ID {shot_id} is out of bounds. The maximum shot ID is {}.",
                self.all_shot_measurements.len()
            ));
        }

        self.current_shot = shot_id;
        self.current_shot_measurements = self.all_shot_measurements[shot_id as usize].clone();
        self.next_measurement_index = 0;
        Ok(())
    }
    fn shot_end(&mut self) -> Result<()> {
        // Ensure all measurements have been performed for the current shot.
        if self.next_measurement_index < self.current_shot_measurements.len() {
            return Err(anyhow!(
                "Not all measurements have been performed for shot {}.",
                self.current_shot
            ));
        }
        self.current_shot_measurements.clear();
        self.next_measurement_index = 0;
        Ok(())
    }

    fn rxy(&mut self, q0: u64, _theta: f64, _phi: f64) -> Result<()> {
        if q0 < self.n_qubits {
            Ok(())
        } else {
            Err(anyhow!(
                "RXY(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        }
    }

    fn rzz(&mut self, q0: u64, q1: u64, _theta: f64) -> Result<()> {
        if q0 < self.n_qubits && q1 < self.n_qubits {
            Ok(())
        } else {
            Err(anyhow!(
                "RZZ(q0={q0}, q1={q1}) is out of bounds. q0 and q1 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        }
    }

    fn rz(&mut self, q0: u64, _theta: f64) -> Result<()> {
        if q0 < self.n_qubits {
            Ok(())
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
        } else if self.next_measurement_index < self.current_shot_measurements.len() {
            let measurement = self.current_shot_measurements[self.next_measurement_index];
            self.next_measurement_index += 1;
            Ok(measurement)
        } else {
            Err(anyhow!(
                "All measurements have already been performed for shot {}.",
                self.current_shot
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
            Ok(())
        }
    }
    fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        match nth_metric {
            0 => Ok(Some((
                "measurements_performed".to_string(),
                MetricValue::U64(self.next_measurement_index as u64),
            ))),
            _ => Ok(None),
        }
    }
}

#[derive(Default)]
pub struct ClassicalReplaySimulatorFactory;

impl SimulatorInterfaceFactory for ClassicalReplaySimulatorFactory {
    type Interface = ClassicalReplaySimulator;

    fn init(
        self: std::sync::Arc<Self>,
        n_qubits: u64,
        args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        let args: Vec<String> = args.iter().map(|s| s.as_ref().to_string()).collect();
        match Params::try_parse_from(args) {
            Err(e) => Err(anyhow!(
                "Error parsing arguments to classical replay plugin: {e}"
            )),
            Ok(params) => match parse_measurement_strings(params.measurements, params.counts) {
                Err(e) => Err(anyhow!("Error parsing measurements: {e}")),
                Ok(measurements) => Ok(Box::new(ClassicalReplaySimulator {
                    n_qubits,
                    current_shot: 0,
                    next_measurement_index: 0,
                    current_shot_measurements: vec![],
                    all_shot_measurements: measurements,
                })),
            },
        }
    }
}
export_simulator_plugin!(crate::ClassicalReplaySimulatorFactory);
