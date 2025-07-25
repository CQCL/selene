use anyhow::{Result, anyhow};
use std::sync::Arc;

use crate::utils::MetricValue;

use super::BatchOperation;

/// Instances of runtime plugins implement this interface.
///
/// Many instances of a plugin may exist simultaneously. Instance are
/// generically constructed by impls of [RuntimeInterfaceFactory].
///
/// All functions can return an error, which will usually result in aborting the
/// emulation.
///
/// `cdylib` crates can export the runtime plugin C interface via
/// [crate::export_runtime_plugin!]
pub trait RuntimeInterface {
    /// Signals that the instance of the runtime plugin should cleanup. Plugins
    /// should `Err`` from any functions called on an instance after `exit`.
    fn exit(&mut self) -> Result<()>;
    /// Called to retrieve the next batch of operations from the runtime.
    ///
    /// An empty batch is interpreted as the plugin having no more operations at
    /// this time
    fn get_next_operations(&mut self) -> Result<Option<BatchOperation>>;

    /// Called to signal that the runtime should prepare to begin a shot
    fn shot_start(&mut self, shot_id: u64, seed: u64) -> Result<()>;

    /// Called to signal that the current shot has ended. This is an
    /// ideal place to perform validation (if applicable) and cleanup.
    fn shot_end(&mut self) -> Result<()>;

    // If a runtime has special behaviour that a user may wish to invoke
    // from within a utility, this function serves as a generic interface
    // to that. For example, if the runtime supports some behaviour invokable
    // via frobnicate_qubits(qubit_a, qubit_b), and this doesn't make sense to
    // expose as a behaviour within standard guppy/hugr/etc, then a developer
    // may create extensions to guppy/hugr/etc that define frobinate_qubits
    // for a user to invoke as a symbol. A utility can then be created that
    // defines frobnicate_qubits as a symbol, invoking selene's exposed
    // runtime_call_direct function with a unique tag and some data, which
    // will then be passed to this function. After this function has been invoked,
    // get_next_operations will be called by selene in case the result of the
    // call is a sequence of operations.
    fn custom_call(&mut self, _tag: u64, _data: &[u8]) -> Result<u64> {
        Err(anyhow!(
            "A custom call has been issued to a runtime that does not support custom calls."
        ))
    }

    /// Provide a metric to the output stream.
    ///
    /// Will be called with incrementing `nth_metric` until `None` is returned.
    fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>>;

    /// Attempt to allocate a free qubit. If no qubits are available,
    /// then [u64::MAX] should be returned.
    fn qalloc(&mut self) -> Result<u64>;

    /// Free a qubit. An error should be returned if the qubit is not allocated.
    fn qfree(&mut self, qubit_id: u64) -> Result<()>;

    /// Schedule an RXY gate to allocated qubit `qubit_id` with the given angles.
    fn rxy_gate(&mut self, qubit_id: u64, theta: f64, phi: f64) -> Result<()>;

    /// Schedule an RZZ gate between allocated qubits `qubit_id_1` and `qubit_id_2` with the given angle.
    fn rzz_gate(&mut self, qubit_id_1: u64, qubit_id_2: u64, theta: f64) -> Result<()>;

    /// Schedule an RZ gate to allocated qubit `qubit_id` with the given angle.
    fn rz_gate(&mut self, qubit_id: u64, theta: f64) -> Result<()>;

    /// Schedule a measurement of allocated qubit `qubit_id`. The plugin should return a
    /// new result index. That result index must have a reference count of 1.
    fn measure(&mut self, qubit_id: u64) -> Result<u64>;

    /// Schedule a leakage-detection and measurement of allocated qubit `qubit_id`.
    /// The plugin should return a new result index. That result index must have a
    /// reference count of 1.
    fn measure_leaked(&mut self, qubit_id: u64) -> Result<u64>;

    /// Schedule a reset of allocated qubit `qubit_id`.
    fn reset(&mut self, qubit_id: u64) -> Result<()>;

    /// A hint to the plugin that the result with index `result_id` is needed.
    ///
    /// The plugin should ensure that, were the emulator to call
    /// `get_next_operations` until it returns an empty batch, then `get_result`
    /// would return `Some`.
    fn force_result(&mut self, result_id: u64) -> Result<()>;

    /// Get the result of the measurement with index `result_id`. The plugin should
    /// return `None` if the result is not yet available.
    fn get_bool_result(&mut self, result_id: u64) -> Result<Option<bool>>;

    /// Get the result of the measurement with index `result_id`. The plugin should
    /// return `None` if the result is not yet available.
    fn get_u64_result(&mut self, result_id: u64) -> Result<Option<u64>>;

    /// Set the result of the measurement with index `result_id` to `result`.
    /// This is called by the emulator with the result from the simulator after
    /// the corresponding measurement is returned from `get_next_operations`.
    fn set_bool_result(&mut self, result_id: u64, result: bool) -> Result<()>;

    /// Increment the reference count of the result with index `result_id`.
    /// Set the result of the measurement with index `result_id` to `result`.
    /// This is called by the emulator with the result from the simulator after
    /// the corresponding measurement is returned from `get_next_operations`.
    fn set_u64_result(&mut self, result_id: u64, result: u64) -> Result<()>;

    /// It is invalid to refer to a result index after its reference count has
    /// reached zero.
    fn increment_future_refcount(&mut self, future: u64) -> Result<()>;

    /// Decrement the reference count of the result with index `result_id`.
    /// It is invalid to refer to a result index after its reference count has
    /// reached zero.
    fn decrement_future_refcount(&mut self, future: u64) -> Result<()>;

    /// Flush all operations involving the given qubits
    /// with a sleep time associated with those qubits afterwards
    /// (set to 0 for vanilla barrier).
    fn local_barrier(&mut self, qubits: &[u64], sleep_ns: u64) -> Result<()>;

    /// Flush all operations involving all allocated qubits
    /// with a sleep time associated afterwards
    /// (set to 0 for vanilla barrier).
    fn global_barrier(&mut self, sleep_ns: u64) -> Result<()>;
}

pub trait RuntimeInterfaceFactory {
    type Interface: RuntimeInterface;
    fn init(
        self: Arc<Self>,
        n_qubits: u64,
        start: crate::time::Instant,
        args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>>;
}
