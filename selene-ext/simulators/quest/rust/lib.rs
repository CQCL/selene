use anyhow::{Result, anyhow, bail};
use selene_core::export_simulator_plugin;
use selene_core::simulator::SimulatorInterface;
use selene_core::simulator::interface::SimulatorInterfaceFactory;
use selene_core::utils::MetricValue;
use std::io::Write;

use quest_sys::Qureg;
use std::mem::size_of;
use std::os::raw::{c_int, c_ulong};

#[cfg(test)]
mod tests;

pub struct QuestSimulator {
    environment: quest_sys::QuESTEnv,
    qureg: Qureg,
    n_qubits: u64,
    cumulative_postselect_probability: f64,
}

impl QuestSimulator {
    fn seed(&mut self, seed: u64) {
        // seedQuest accepts an array of 'c_ulong's, so we need to split the
        // provided seed accordingly.
        //
        // c_ulong does not have a standard size, so we find out how many c_ulongs we
        // need and populate them with (low endian) bytes from the seed.
        const N_ULONGS: usize = size_of::<u64>().div_ceil(size_of::<c_ulong>());
        let mut seed_bytes = [0_u8; N_ULONGS * size_of::<c_ulong>()];
        seed_bytes[..size_of::<u64>()].copy_from_slice(&seed.to_le_bytes());
        unsafe {
            quest_sys::seedQuEST(
                &mut self.environment,
                seed_bytes.as_mut_ptr() as *mut c_ulong,
                N_ULONGS as i32,
            );
        }
    }
}

impl SimulatorInterface for QuestSimulator {
    fn exit(&mut self) -> Result<()> {
        Ok(())
    }

    fn shot_start(&mut self, _shot_id: u64, seed: u64) -> Result<()> {
        unsafe { quest_sys::initClassicalState(self.qureg, 0) };
        self.cumulative_postselect_probability = 1.0;
        self.seed(seed);
        Ok(())
    }

    fn shot_end(&mut self) -> Result<()> {
        Ok(())
    }

    fn rz(&mut self, q0: u64, theta: f64) -> Result<()> {
        if q0 >= self.n_qubits {
            Err(anyhow!(
                "RZ(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        } else {
            unsafe { quest_sys::rotateZ(self.qureg, q0 as c_int, theta) };
            Ok(())
        }
    }

    fn rxy(&mut self, q0: u64, theta: f64, phi: f64) -> Result<()> {
        if q0 >= self.n_qubits {
            Err(anyhow!(
                "RXY(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        } else {
            unsafe {
                quest_sys::rotateZ(self.qureg, q0 as c_int, -phi);
                quest_sys::rotateX(self.qureg, q0 as c_int, theta);
                quest_sys::rotateZ(self.qureg, q0 as c_int, phi);
            }
            Ok(())
        }
    }

    fn rzz(&mut self, q0: u64, q1: u64, theta: f64) -> Result<()> {
        if q0 >= self.n_qubits || q1 >= self.n_qubits {
            Err(anyhow!(
                "RZZ(q0={q0}, q1={q1}) is out of bounds. q0 and q1 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        } else {
            let cos = theta.cos();
            let sin = theta.sin();
            let u = quest_sys::ComplexMatrix4 {
                real: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, cos, 0.0, 0.0],
                    [0.0, 0.0, cos, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
                imag: [
                    [0.0, 0.0, 0.0, 0.0],
                    [0.0, sin, 0.0, 0.0],
                    [0.0, 0.0, sin, 0.0],
                    [0.0, 0.0, 0.0, 0.0],
                ],
            };
            unsafe { quest_sys::applyMatrix4(self.qureg, q0 as c_int, q1 as c_int, u) };
            Ok(())
        }
    }

    fn measure(&mut self, q0: u64) -> Result<bool> {
        if q0 >= self.n_qubits {
            Err(anyhow!(
                "Measure(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        } else {
            Ok(unsafe { quest_sys::measure(self.qureg, q0 as i32) } > 0)
        }
    }

    fn postselect(&mut self, q0: u64, target_value: bool) -> Result<()> {
        if q0 >= self.n_qubits {
            Err(anyhow!(
                "Postselect(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        } else {
            let target_value = if target_value { 1 } else { 0 };
            unsafe {
                quest_sys::applyProjector(self.qureg, q0 as i32, target_value);
            }
            let postselect_probability = unsafe { quest_sys::calcTotalProb(self.qureg) };
            self.cumulative_postselect_probability *= postselect_probability;
            if postselect_probability < 1e-10 {
                return Err(anyhow!(
                    "Postselection of {target_value} on qubit {q0} is too unlikely to postselect. The probability of this outcome is {postselect_probability:.2e}.",
                ));
            }
            let scale = 1.0 / postselect_probability.sqrt();
            // Rescale the state vector to maintain normalization
            let mat = quest_sys::ComplexMatrix2 {
                real: [[scale, 0.0], [0.0, scale]],
                imag: [[0.0, 0.0], [0.0, 0.0]],
            };
            unsafe {
                quest_sys::applyMatrix2(self.qureg, q0 as i32, mat);
            }
            Ok(())
        }
    }

    fn reset(&mut self, q0: u64) -> Result<()> {
        if q0 >= self.n_qubits {
            Err(anyhow!(
                "Reset(q0={q0}) is out of bounds. q0 must be less than the number of qubits ({}).",
                self.n_qubits
            ))
        } else {
            let outcome = unsafe { quest_sys::measure(self.qureg, q0 as i32) };
            if outcome == 1 {
                unsafe { quest_sys::pauliX(self.qureg, q0 as i32) };
            }
            Ok(())
        }
    }
    fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        match nth_metric {
            0 => Ok(Some((
                "cumulative_postselect_probability".to_string(),
                MetricValue::F64(self.cumulative_postselect_probability),
            ))),
            _ => Ok(None),
        }
    }
    fn dump_state(&mut self, file: &std::path::Path, qubits: &[u64]) -> Result<()> {
        let handle = std::fs::File::create(file)?;
        let mut writer = std::io::BufWriter::new(handle);
        writer.write_all(b"selene-quest")?;
        writer.write_all(self.n_qubits.to_le_bytes().as_slice())?;
        writer.write_all((qubits.len() as u64).to_le_bytes().as_slice())?;
        for &q in qubits {
            writer.write_all(q.to_le_bytes().as_slice())?;
        }
        let reals: *const f64 = self.qureg.stateVec.real;
        let imags: *const f64 = self.qureg.stateVec.imag;
        for i in 0..(1 << self.n_qubits) {
            let real: f64 = unsafe { *reals.add(i as usize) };
            let imag: f64 = unsafe { *imags.add(i as usize) };
            writer.write_all(real.to_le_bytes().as_slice())?;
            writer.write_all(imag.to_le_bytes().as_slice())?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct QuestSimulatorFactory;

fn check_memory(n_qubits: u64) -> Result<()> {
    if n_qubits == 0 {
        bail!("Number of qubits must be greater than 0");
    } else if n_qubits > 60 {
        bail!(
            "It is impossible to describe more than 60 qubits in a statevector on a computer with a 64-bit address space."
        );
    }
    // check against the maximum size of a 64-bit address space
    let bytes_required = bytesize::ByteSize::b(16 * (1 << n_qubits));
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let reported_available = system.available_memory();
    if reported_available == 0 {
        eprintln!("-----------------------------------");
        eprintln!("Unable to determine available memory due to system limitations.");
        eprintln!("QuEST is going to try to allocate {bytes_required} of memory to");
        eprintln!("store the statevector, and this will be multiplied by the number");
        eprintln!("of processes if running in multiprocessing mode.");
        eprintln!();
        eprintln!("If this fails, verify that your system has sufficient memory.");
        eprintln!("-----------------------------------");
    } else {
        let bytes_available = bytesize::ByteSize::b(reported_available);
        if bytes_required > bytes_available {
            bail!(
                "Insufficient memory available ({bytes_available}) to allocate a state vector of {n_qubits} qubits ({bytes_required}).",
            );
        }
    }
    Ok(())
}

impl SimulatorInterfaceFactory for QuestSimulatorFactory {
    type Interface = QuestSimulator;

    fn init(
        self: std::sync::Arc<Self>,
        n_qubits: u64,
        args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        let args: Vec<String> = args.iter().map(|s| s.as_ref().to_string()).collect();
        if args.len() > 1 {
            bail!(
                "Expected no arguments for the quest plugin, got {} arguments: {:?}",
                args.len() - 1,
                args.iter().skip(1)
            );
        }
        check_memory(n_qubits)?;
        let environment = unsafe { quest_sys::createQuESTEnv() };
        let qureg = unsafe { quest_sys::createQureg(n_qubits.try_into().unwrap(), environment) };
        Ok(Box::new(QuestSimulator {
            environment,
            qureg,
            n_qubits,
            cumulative_postselect_probability: 1.0,
        }))
    }
}

export_simulator_plugin!(crate::QuestSimulatorFactory);
