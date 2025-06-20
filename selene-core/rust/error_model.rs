use crate::utils::MetricValue;
use anyhow::Result;
use std::ffi::OsStr;
use std::sync;

pub mod helper;
pub mod interface;
pub mod plugin;
pub mod version;
use crate::runtime::BatchOperation;
use delegate::delegate;
pub use interface::{ErrorModelInterface, ErrorModelInterfaceFactory};
pub use version::ErrorModelAPIVersion;

#[derive(Default)]
pub struct MeasurementResult {
    pub result_id: u64,
    pub measurement: bool,
}
#[derive(Default)]
pub struct BatchResult {
    pub measurements: Vec<MeasurementResult>,
}
impl BatchResult {
    pub fn set_measurement_result(&mut self, result_id: u64, measurement: bool) {
        self.measurements.push(MeasurementResult {
            result_id,
            measurement,
        });
    }
}

pub struct ErrorModel(Box<dyn ErrorModelInterface>);

impl ErrorModel {
    pub fn new(
        factory: sync::Arc<impl ErrorModelInterfaceFactory + 'static>,
        n_qubits: u64,
        error_model_args: &[impl AsRef<str>],
        simulator_path: &impl AsRef<OsStr>,
        simulator_args: &[impl AsRef<str>],
    ) -> Result<Self> {
        Ok(Self(factory.init(
            n_qubits,
            error_model_args,
            simulator_path,
            simulator_args,
        )?))
    }
}

impl AsRef<dyn ErrorModelInterface> for ErrorModel {
    fn as_ref(&self) -> &(dyn ErrorModelInterface + 'static) {
        self.0.as_ref()
    }
}

impl AsMut<dyn ErrorModelInterface> for ErrorModel {
    fn as_mut(&mut self) -> &mut (dyn ErrorModelInterface + 'static) {
        &mut *self.0
    }
}

impl ErrorModelInterface for ErrorModel {
    delegate! {
        to self.0.as_mut() {
            fn shot_start(&mut self, shot_id: u64, error_model_seed: u64, simulator_seed: u64) -> Result<()>;
            fn shot_end(&mut self) -> Result<()>;
            fn handle_operations(&mut self, operations: BatchOperation) -> Result<BatchResult>;
            fn exit(&mut self) -> Result<()>;
            fn dump_simulator_state(&mut self, file: &std::path::Path, qubits: &[u64]) -> Result<()>;
            fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>>;
            fn get_simulator_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>>;
        }
    }
}
