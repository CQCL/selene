mod bindings;
mod wrapper;

#[cfg(test)]
mod tests;

use anyhow::{Result, anyhow};
use clap::Parser;
use selene_core::export_simulator_plugin;
use selene_core::simulator::SimulatorInterface;
use selene_core::simulator::interface::SimulatorInterfaceFactory;
use selene_core::utils::MetricValue;
use wrapper::TableauSimulator64;

enum ApproxAngle {
    Zero,
    Frac3Pi2,
    FracPi2,
    Pi,
    NoSuitableApproximation,
}

#[derive(Parser, Debug)]
struct Params {
    #[arg(long)]
    angle_threshold: f64,
}

pub struct StimSimulator {
    simulator: TableauSimulator64,
    n_qubits: u64,
    angle_threshold: f64,
}
impl StimSimulator {
    fn get_approximate_angle(&self, theta: f64) -> ApproxAngle {
        let quadrant_float = theta * 2.0 / std::f64::consts::PI;
        let quadrant = quadrant_float.round() as i32;
        let within_threshold = (quadrant_float - quadrant as f64).abs() < self.angle_threshold;
        match (within_threshold, quadrant % 4) {
            (true, -3) => ApproxAngle::FracPi2,
            (true, -2) => ApproxAngle::Pi,
            (true, -1) => ApproxAngle::Frac3Pi2,
            (true, 0) => ApproxAngle::Zero,
            (true, 1) => ApproxAngle::FracPi2,
            (true, 2) => ApproxAngle::Pi,
            (true, 3) => ApproxAngle::Frac3Pi2,
            _ => ApproxAngle::NoSuitableApproximation,
        }
    }
}

impl SimulatorInterface for StimSimulator {
    fn exit(&mut self) -> Result<()> {
        Ok(())
    }

    fn shot_start(&mut self, _shot_id: u64, seed: u64) -> Result<()> {
        self.simulator = TableauSimulator64::new(self.n_qubits.try_into().unwrap(), seed);
        Ok(())
    }
    fn shot_end(&mut self) -> Result<()> {
        Ok(())
    }

    fn rxy(&mut self, q0: u64, theta: f64, phi: f64) -> Result<()> {
        if q0 >= self.n_qubits {
            return Err(anyhow!(
                "RXYGate(q0={q0}, theta={theta}, phi={phi}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ));
        }

        let approx_theta = self.get_approximate_angle(theta);
        let approx_phi = self.get_approximate_angle(phi);
        let q0_u32: u32 = q0.try_into().unwrap();

        match approx_phi {
            ApproxAngle::Zero => (),
            ApproxAngle::FracPi2 => self.simulator.sqrt_z_dag(q0_u32),
            ApproxAngle::Pi => self.simulator.z(q0_u32),
            ApproxAngle::Frac3Pi2 => self.simulator.sqrt_z(q0_u32),
            ApproxAngle::NoSuitableApproximation => {
                return Err(anyhow!(
                    "RXYGate(q0={q0}, theta={theta}, phi={phi}) is not representable in stabiliser form. Angles must be (approximate) multiples of pi/2 in order to use Stim."
                ));
            }
        }
        match approx_theta {
            ApproxAngle::Zero => (),
            ApproxAngle::FracPi2 => self.simulator.sqrt_x(q0_u32),
            ApproxAngle::Pi => self.simulator.x(q0_u32),
            ApproxAngle::Frac3Pi2 => self.simulator.sqrt_x_dag(q0_u32),
            ApproxAngle::NoSuitableApproximation => {
                return Err(anyhow!(
                    "RXYGate(q0={q0}, theta={theta}, phi={phi}) is not representable in stabiliser form. Angles must be (approximate) multiples of pi/2 in order to use Stim."
                ));
            }
        }
        match approx_phi {
            ApproxAngle::Zero => (),
            ApproxAngle::FracPi2 => self.simulator.sqrt_z(q0_u32),
            ApproxAngle::Pi => self.simulator.z(q0_u32),
            ApproxAngle::Frac3Pi2 => self.simulator.sqrt_z_dag(q0_u32),
            ApproxAngle::NoSuitableApproximation => {
                return Err(anyhow!(
                    "RXYGate(q0={q0}, theta={theta}, phi={phi}) is not representable in stabiliser form. Angles must be (approximate) multiples of pi/2 in order to use Stim."
                ));
            }
        }
        Ok(())
    }

    fn rz(&mut self, q0: u64, theta: f64) -> Result<()> {
        if q0 >= self.n_qubits {
            return Err(anyhow!(
                "RZGate(q0={q0}, theta={theta}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ));
        }

        let approx = self.get_approximate_angle(theta);
        let q0_u32: u32 = q0.try_into().unwrap();

        match approx {
            ApproxAngle::Zero => (),
            ApproxAngle::FracPi2 => self.simulator.sqrt_z(q0_u32),
            ApproxAngle::Pi => self.simulator.z(q0_u32),
            ApproxAngle::Frac3Pi2 => self.simulator.sqrt_z_dag(q0_u32),
            ApproxAngle::NoSuitableApproximation => {
                return Err(anyhow!(
                    "RZGate(q0={q0}, theta={theta}) is not representable in stabiliser form. Angles must be (approximate) multiples of pi/2 in order to use Stim."
                ));
            }
        }
        Ok(())
    }

    fn rzz(&mut self, q0: u64, q1: u64, theta: f64) -> Result<()> {
        if q0 >= self.n_qubits || q1 >= self.n_qubits {
            return Err(anyhow!(
                "RZZGate(q0={q0}, q1={q1}, theta={theta}) is out of bounds. q0 and q1 must be less than the number of qubits ({}).",
                self.n_qubits
            ));
        }

        let q0_u32: u32 = q0.try_into().unwrap();
        let q1_u32: u32 = q1.try_into().unwrap();
        let approx = self.get_approximate_angle(theta);

        match approx {
            ApproxAngle::Zero => (),
            ApproxAngle::FracPi2 => self.simulator.sqrt_zz(q0_u32, q1_u32),
            ApproxAngle::Pi => {
                self.simulator.z(q0_u32);
                self.simulator.z(q1_u32)
            }
            ApproxAngle::Frac3Pi2 => self.simulator.sqrt_zz_dag(q0_u32, q1_u32),
            ApproxAngle::NoSuitableApproximation => {
                return Err(anyhow!(
                    "RZZGate(q0={q0}, q1={q1}, theta={theta}) is not representable in stabiliser form. Angles must be (approximate) multiples of pi/2 in order to use Stim."
                ));
            }
        }
        Ok(())
    }

    fn measure(&mut self, qubit: u64) -> Result<bool> {
        if qubit >= self.n_qubits {
            Err(anyhow!(
                "Measure(qubit={qubit}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        } else {
            let q_u32: u32 = qubit.try_into()?;
            Ok(self.simulator.mz(q_u32))
        }
    }

    fn postselect(&mut self, qubit: u64, target_value: bool) -> Result<()> {
        if qubit >= self.n_qubits {
            Err(anyhow!(
                "Postselect(qubit={qubit}, target_value={target_value}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        } else {
            let q_u32: u32 = qubit.try_into()?;
            match self.simulator.postselect_z(q_u32, target_value) {
                true => Ok(()),
                false => Err(anyhow!(
                    "Postselect(qubit={qubit}, target_value={target_value}) failed.",
                )),
            }
        }
    }

    fn reset(&mut self, qubit: u64) -> Result<()> {
        if qubit >= self.n_qubits {
            Err(anyhow!(
                "Reset(qubit={qubit}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        } else {
            let q_u32: u32 = qubit.try_into()?;
            if self.simulator.mz(q_u32) {
                self.simulator.x(q_u32);
            }
            Ok(())
        }
    }

    fn get_metric(&mut self, _nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        Ok(None)
    }
}

#[derive(Default)]
pub struct StimSimulatorFactory;

impl SimulatorInterfaceFactory for StimSimulatorFactory {
    type Interface = StimSimulator;

    fn init(
        self: std::sync::Arc<Self>,
        n_qubits: u64,
        args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        let args: Vec<String> = args.iter().map(|s| s.as_ref().to_string()).collect();
        match Params::try_parse_from(args) {
            Err(e) => Err(anyhow!("Error parsing arguments to stim plugin: {}", e)),
            Ok(params) => {
                let n_u32: u32 = n_qubits.try_into()?;
                Ok(Box::new(StimSimulator {
                    simulator: TableauSimulator64::new(n_u32, 0),
                    n_qubits,
                    angle_threshold: params.angle_threshold,
                }))
            }
        }
    }
}

export_simulator_plugin!(crate::StimSimulatorFactory);
