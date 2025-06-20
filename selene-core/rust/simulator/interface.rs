use anyhow::Result;
use std::sync::Arc;

use crate::utils::MetricValue;

pub trait SimulatorInterface {
    // Signals that the instance of the simulator should cleanup. Plugins
    // should `Err` from any functions called on an instance after `exit`.
    fn exit(&mut self) -> Result<()>;

    // Called to signal that the simulator should prepare to begin a shot
    // with the given random seed.
    fn shot_start(&mut self, shot_id: u64, seed: u64) -> Result<()>;

    // Called to signal that the simulator should prepare to end the current
    // shot.
    fn shot_end(&mut self) -> Result<()>;

    // Perform a Z rotation on the given qubit with the given angle.
    fn rz(&mut self, qubit: u64, theta: f64) -> Result<()>;

    // Perform an Rxy gate on the given qubit with the given angles.
    // This gate is also known as phased_x and R1XY
    fn rxy(&mut self, qubit: u64, theta: f64, phi: f64) -> Result<()>;

    // Perform an Rzz gate between the given qubits with the given angle.
    // This gate is also known as zz_phase, phase_shift, and R2ZZ
    fn rzz(&mut self, qubit1: u64, qubit2: u64, theta: f64) -> Result<()>;

    // Perform a measurement on the given qubit.
    // The result of the measurement is returned as a boolean.
    fn measure(&mut self, qubit: u64) -> Result<bool>;

    // Perform a post-selection on the given qubit.
    // If the post-selection isn't deemed possible, return an error.
    // This is optional functionality, and the default is to raise an
    // error.
    fn postselect(&mut self, _qubit: u64, _target_value: bool) -> Result<()> {
        Err(anyhow::anyhow!(
            "Post-selection is not supported on the chosen simulator."
        ))
    }

    // Reset the given qubit to the |0> state.
    fn reset(&mut self, qubit: u64) -> Result<()>;

    // Provide a metric to the output stream.
    // Will be called with incrementing `nth_metric` until `None` is returned.
    fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>>;

    // Dump the internal state of the simulator to the given file, in a manner
    // parsable by the python component of the simulator. The qubits provided
    // by the user may be used to specify an ordering, but the approach used
    // is implementor defined.
    fn dump_state(&mut self, _file: &std::path::Path, _qubits: &[u64]) -> Result<()> {
        Err(anyhow::anyhow!(
            "Dumping state is not supported on the chosen simulator."
        ))
    }
}

pub trait SimulatorInterfaceFactory {
    type Interface: SimulatorInterface;
    fn init(
        self: Arc<Self>,
        n_qubits: u64,
        args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>>;
}
