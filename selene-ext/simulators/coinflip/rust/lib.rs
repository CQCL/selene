use anyhow::{Result, anyhow};
use clap::Parser;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use selene_core::export_simulator_plugin;
use selene_core::simulator::SimulatorInterface;
use selene_core::simulator::interface::SimulatorInterfaceFactory;
use selene_core::utils::MetricValue;

#[cfg(test)]
mod tests;

#[derive(Parser, Debug)]
struct Params {
    #[arg(long)]
    bias: f64,
}

pub struct CoinflipSimulator {
    n_qubits: u64,
    rng: Pcg64Mcg,
    bias: f64,
    total_flips: u64,
    true_flips: u64,
}
impl CoinflipSimulator {
    pub fn flip(&mut self) -> bool {
        self.rng.random::<f64>() < self.bias
    }
    pub fn get_observed_bias(&self) -> f64 {
        self.true_flips as f64 / self.total_flips as f64
    }
}

impl SimulatorInterface for CoinflipSimulator {
    fn exit(&mut self) -> Result<()> {
        Ok(())
    }

    fn shot_start(&mut self, _shot_id: u64, seed: u64) -> Result<()> {
        self.rng = Pcg64Mcg::seed_from_u64(seed);
        Ok(())
    }

    fn shot_end(&mut self) -> Result<()> {
        self.true_flips = 0;
        self.total_flips = 0;
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
        if q0 < self.n_qubits {
            self.total_flips += 1;
            if self.flip() {
                self.true_flips += 1;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Err(anyhow!(
                "Measure(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        }
    }

    fn postselect(&mut self, q0: u64, _target_value: bool) -> Result<()> {
        if q0 < self.n_qubits {
            Ok(())
        } else {
            Err(anyhow!(
                "Postselect(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        }
    }

    fn reset(&mut self, q0: u64) -> Result<()> {
        if q0 < self.n_qubits {
            Ok(())
        } else {
            Err(anyhow!(
                "Reset(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        }
    }

    fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        match nth_metric {
            0 => {
                if self.total_flips == 0 {
                    Ok(None)
                } else {
                    Ok(Some((
                        "observed_bias".to_string(),
                        MetricValue::F64(self.get_observed_bias()),
                    )))
                }
            }
            _ => Ok(None),
        }
    }
}

#[derive(Default)]
pub struct CoinflipSimulatorFactory;

impl SimulatorInterfaceFactory for CoinflipSimulatorFactory {
    type Interface = CoinflipSimulator;

    fn init(
        self: std::sync::Arc<Self>,
        n_qubits: u64,
        args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        let args: Vec<String> = args.iter().map(|s| s.as_ref().to_string()).collect();

        match Params::try_parse_from(args) {
            Err(e) => Err(anyhow!("Error parsing arguments to coinflip plugin: {}", e)),
            Ok(params) => Ok(Box::new(CoinflipSimulator {
                n_qubits,
                rng: Pcg64Mcg::seed_from_u64(0),
                bias: params.bias,
                total_flips: 0,
                true_flips: 0,
            })),
        }
    }
}

export_simulator_plugin!(crate::CoinflipSimulatorFactory);
