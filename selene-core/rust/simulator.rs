pub mod conformance_testing;
pub mod helper;
pub mod interface;
pub mod plugin;
pub mod version;

use std::ffi::OsStr;
use std::sync::Arc;

pub use interface::{SimulatorInterface, SimulatorInterfaceFactory};
pub use version::SimulatorAPIVersion;

use crate::utils::MetricValue;
use anyhow::Result;
use delegate::delegate;

/// An instance of a simulator plugin, ready to be used for emulation.
///
/// `Simulator`'s impl [SimulatorInterface] delegates to a wrapped `dyn
/// SimulatorInterface`.
///
/// Usually the wrapped [SimulatorInterface] will be an instance
/// delegating to an external simulator plugin as defined by
/// [plugin::SimulatorPluginInterface)].
pub struct Simulator(Box<dyn SimulatorInterface>);

impl Simulator {
    /// Constructs a new Simulator from a [SimulatorInterfaceFactory].
    pub fn new(
        factory: Arc<impl SimulatorInterfaceFactory + 'static>,
        n_qubits: u64,
        args: &[impl AsRef<str>],
    ) -> Result<Self> {
        Ok(Self(factory.init(n_qubits, args)?))
    }

    pub fn load_from_file(
        plugin_path: &impl AsRef<OsStr>,
        n_qubits: u64,
        args: &[impl AsRef<str>],
    ) -> Result<Self> {
        let plugin = plugin::SimulatorPluginInterface::new_from_file(plugin_path)?;
        Self::new(plugin, n_qubits, args)
    }
}

impl AsRef<dyn SimulatorInterface> for Simulator {
    fn as_ref(&self) -> &(dyn SimulatorInterface + 'static) {
        self.0.as_ref()
    }
}

impl AsMut<dyn SimulatorInterface> for Simulator {
    fn as_mut(&mut self) -> &mut (dyn SimulatorInterface + 'static) {
        &mut *self.0
    }
}

impl SimulatorInterface for Simulator {
    delegate! {
        to self.0 {
            fn exit(&mut self) -> Result<()>;
            fn shot_start(&mut self, shot_id: u64, seed: u64) -> Result<()>;
            fn shot_end(&mut self) -> Result<()>;
            fn rxy(&mut self, qubit: u64, theta: f64, phi: f64) -> Result<()>;
            fn rzz(&mut self, qubit1: u64, qubit2: u64, theta: f64) -> Result<()>;
            fn rz(&mut self, qubit: u64, theta: f64) -> Result<()>;
            fn measure(&mut self, qubit: u64) -> Result<bool>;
            fn postselect(&mut self, qubit: u64, target_value: bool) -> Result<()>;
            fn reset(&mut self, qubit: u64) -> Result<()>;
            fn dump_state(&mut self, file: &std::path::Path, qubits: &[u64]) -> Result<()>;
            fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>>;
        }
    }
}
