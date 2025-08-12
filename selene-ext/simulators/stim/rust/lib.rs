mod bindings;
mod wrapper;

#[cfg(test)]
mod tests;

use anyhow::{Result, anyhow};
use clap::Parser;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use selene_core::export_simulator_plugin;
use selene_core::simulator::SimulatorInterface;
use selene_core::simulator::interface::SimulatorInterfaceFactory;
use selene_core::utils::MetricValue;
use std::io::Write;
use wrapper::TableauSimulator64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum Quadrant {
    Zero = 0,
    FracPi2 = 1,
    Pi = 2,
    Frac3Pi2 = 3,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum Octant {
    Zero = 0,
    FracPi4 = 1,
    FracPi2 = 2,
    Frac3Pi4 = 3,
    Pi = 4,
    Frac5Pi4 = 5,
    Frac3Pi2 = 6,
    Frac7Pi4 = 7,
}

#[derive(Parser, Debug)]
struct Params {
    #[arg(long)]
    angle_threshold: f64,
}

pub struct StimSimulator {
    simulator: TableauSimulator64,
    n_qubits: u64,
    angle_threshold_quad: f64,
}
impl StimSimulator {
    fn get_approximate_quadrant(&self, theta: f64) -> Option<Quadrant> {
        let quadrant_float = theta / std::f64::consts::FRAC_PI_2;
        let quadrant_rounded = quadrant_float.round();
        if (quadrant_float - quadrant_rounded).abs() > self.angle_threshold_quad {
            return None; // Not within the threshold for any quadrant
        }
        let quadrant = (quadrant_rounded as i64).rem_euclid(4) as u8;
        Some(Quadrant::try_from(quadrant).unwrap())
    }
    fn get_approximate_octant(&self, theta: f64) -> Option<Octant> {
        let octant_float = theta / std::f64::consts::FRAC_PI_4;
        let octant_rounded = octant_float.round();
        if (octant_float - octant_rounded).abs() > self.angle_threshold_quad {
            return None; // Not within the threshold for any octant
        }
        let octant = (octant_rounded as i64).rem_euclid(8) as u8;
        Some(Octant::try_from(octant).unwrap())
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
        // We can represent the rxy gate as a sequence of clifford
        // operations for certain combinations of theta and phi:
        //
        // +===========================+
        // | Theta | Phi   | Operation | notes
        // +-------+-------+-----------+
        // |   0   | [any] |           | identity regardless of phi
        // +-------+-------+-----------+
        // | pi/2  |   0   | V         | pi/2 theta section
        // | pi/2  | pi/2  | H X       |
        // | pi/2  | pi    | Vdg       |
        // | pi/2  | 3pi/2 | H Z       |
        // +-------+-------+-----------+
        // | pi    |   0   | X         | pi theta, quadrant phi section
        // | pi    | pi/2  | X Z       |
        // | pi    | pi    | X         |
        // | pi    | 3pi/2 | X Z       |
        // |-------+-------+-----------+
        // | pi    | pi/4  | X S       | pi theta, non-quadrant phi section
        // | pi    | 3pi/4 | X Sdg     |
        // | pi    | 5pi/4 | X S       |
        // | pi    | 7pi/4 | X Sdg     |
        // +-------+-------+-----------+
        // | 3pi/2 |   0   | Vdg       | 3pi/2 theta section
        // | 3pi/2 | pi/2  | H Z       |
        // | 3pi/2 | pi    | V         |
        // | 3pi/2 | 3pi/2 | H X       |
        // +===========================+
        //
        if q0 >= self.n_qubits {
            return Err(anyhow!(
                "RXY(q0={q0}, theta={theta}, phi={phi}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ));
        }

        let q0_u32: u32 = q0.try_into().unwrap();
        let Some(approx_theta) = self.get_approximate_quadrant(theta) else {
            return Err(anyhow!(
                "RXY(q0={q0}, theta={theta}, phi={phi}) is not representable in stabiliser form. Theta must be an (approximate) multiple of pi/2 for Clifford operations."
            ));
        };
        if approx_theta == Quadrant::Zero {
            // If theta is zero, the operation is just the identity, regardless of phi.
            // Return early to avoid validation on phi.
            return Ok(());
        }

        let Some(approx_phi) = self.get_approximate_octant(phi) else {
            return Err(anyhow!(
                "RXY(q0={q0}, theta={theta}, phi={phi}) is not representable in stabiliser form. When theta is nonzero, phi must be an (approximate) multiple of pi/4 for Clifford operations."
            ));
        };

        match (approx_theta, approx_phi) {
            (Quadrant::Zero, _) => {
                // Already handled with an early return
                unreachable!()
            }

            ////////////////////////
            // pi/2 theta section //
            ////////////////////////
            (Quadrant::FracPi2, Octant::Zero) => {
                self.simulator.sqrt_x(q0_u32);
                Ok(())
            }
            (Quadrant::FracPi2, Octant::FracPi2) => {
                self.simulator.h(q0_u32);
                self.simulator.x(q0_u32);
                Ok(())
            }
            (Quadrant::FracPi2, Octant::Pi) => {
                self.simulator.sqrt_x_dag(q0_u32);
                Ok(())
            }
            (Quadrant::FracPi2, Octant::Frac3Pi2) => {
                self.simulator.h(q0_u32);
                self.simulator.z(q0_u32);
                Ok(())
            }
            (Quadrant::FracPi2, _) => Err(anyhow!(
                "RXY(q0={q0}, theta={theta}, phi={phi}) is not representable in stabiliser form. When theta is pi/2, phi must be an (approximate) multiple of pi/2."
            )),

            ////////////////////////////////////
            // pi theta, quadrant phi section //
            ////////////////////////////////////
            (Quadrant::Pi, Octant::Zero | Octant::Pi) => {
                self.simulator.x(q0_u32);
                Ok(())
            }
            (Quadrant::Pi, Octant::FracPi2 | Octant::Frac3Pi2) => {
                self.simulator.x(q0_u32);
                self.simulator.z(q0_u32);
                Ok(())
            }

            ////////////////////////////////////////
            // pi theta, non-quadrant phi section //
            ////////////////////////////////////////
            (Quadrant::Pi, Octant::FracPi4 | Octant::Frac5Pi4) => {
                // phi - pi/4 = 0 mod pi, phi = pi/4 mod pi
                self.simulator.x(q0_u32);
                self.simulator.sqrt_z(q0_u32);
                Ok(())
            }
            (Quadrant::Pi, Octant::Frac3Pi4 | Octant::Frac7Pi4) => {
                // phi - pi/4 = pi/2 mod pi, phi = 3pi/4 mod pi
                self.simulator.x(q0_u32);
                self.simulator.sqrt_z_dag(q0_u32);
                Ok(())
            }

            /////////////////////////
            // 3pi/2 theta section //
            /////////////////////////
            (Quadrant::Frac3Pi2, Octant::Zero) => {
                self.simulator.sqrt_x_dag(q0_u32);
                Ok(())
            }
            (Quadrant::Frac3Pi2, Octant::FracPi2) => {
                self.simulator.h(q0_u32);
                self.simulator.z(q0_u32);
                Ok(())
            }
            (Quadrant::Frac3Pi2, Octant::Pi) => {
                self.simulator.sqrt_x(q0_u32);
                Ok(())
            }
            (Quadrant::Frac3Pi2, Octant::Frac3Pi2) => {
                self.simulator.h(q0_u32);
                self.simulator.x(q0_u32);
                Ok(())
            }
            (Quadrant::Frac3Pi2, _) => Err(anyhow!(
                "RXY(q0={q0}, theta={theta}, phi={phi}) is not representable in stabiliser form. When theta is 3pi/2, phi must be an (approximate) multiple of pi/2."
            )),
        }
    }

    fn rz(&mut self, q0: u64, theta: f64) -> Result<()> {
        if q0 >= self.n_qubits {
            return Err(anyhow!(
                "RZ(q0={q0}, theta={theta}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ));
        }

        let Some(approx) = self.get_approximate_quadrant(theta) else {
            return Err(anyhow!(
                "RZ(q0={q0}, theta={theta}) is not representable in stabiliser form. Theta must be an (approximate) multiple of pi/2 for Clifford operations."
            ));
        };

        let q0_u32: u32 = q0.try_into().unwrap();

        match approx {
            Quadrant::Zero => (),
            Quadrant::FracPi2 => self.simulator.sqrt_z(q0_u32),
            Quadrant::Pi => self.simulator.z(q0_u32),
            Quadrant::Frac3Pi2 => self.simulator.sqrt_z_dag(q0_u32),
        }
        Ok(())
    }

    fn rzz(&mut self, q0: u64, q1: u64, theta: f64) -> Result<()> {
        if q0 >= self.n_qubits || q1 >= self.n_qubits {
            return Err(anyhow!(
                "RZZ(q0={q0}, q1={q1}, theta={theta}) is out of bounds. q0 and q1 must be less than the number of qubits ({}).",
                self.n_qubits
            ));
        }

        let q0_u32: u32 = q0.try_into().unwrap();
        let q1_u32: u32 = q1.try_into().unwrap();
        let Some(approx) = self.get_approximate_quadrant(theta) else {
            return Err(anyhow!(
                "RZZ(q0={q0}, q1={q1}, theta={theta}) is not representable in stabiliser form. Theta must be an (approximate) multiple of pi/2 for Clifford operations."
            ));
        };

        match approx {
            Quadrant::Zero => (),
            Quadrant::FracPi2 => self.simulator.sqrt_zz(q0_u32, q1_u32),
            Quadrant::Pi => {
                self.simulator.z(q0_u32);
                self.simulator.z(q1_u32)
            }
            Quadrant::Frac3Pi2 => self.simulator.sqrt_zz_dag(q0_u32, q1_u32),
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

    fn dump_state(&mut self, file: &std::path::Path, qubits: &[u64]) -> Result<()> {
        let handle = std::fs::File::create(file)?;
        let mut writer = std::io::BufWriter::new(handle);
        writer.write_all(b"selene-stim")?;
        writer.write_all(self.n_qubits.to_le_bytes().as_slice())?;
        writer.write_all((qubits.len() as u64).to_le_bytes().as_slice())?;
        for &q in qubits {
            writer.write_all(q.to_le_bytes().as_slice())?;
        }
        let stab = self.simulator.get_stabilisers();
        let stab_bytes = stab.as_bytes();
        writer.write_all(stab_bytes)?;
        Ok(())
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
                let angle_threshold_quad = params.angle_threshold / std::f64::consts::FRAC_PI_2;
                Ok(Box::new(StimSimulator {
                    simulator: TableauSimulator64::new(n_u32, 0),
                    n_qubits,
                    angle_threshold_quad,
                }))
            }
        }
    }
}

export_simulator_plugin!(crate::StimSimulatorFactory);
