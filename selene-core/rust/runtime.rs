pub mod helper;
pub mod interface;
pub mod plugin;
pub mod version;

use std::{iter, sync};

pub use interface::{RuntimeInterface, RuntimeInterfaceFactory};
pub use version::RuntimeAPIVersion;

use anyhow::Result;
use delegate::delegate;

/// We assume that operations of the same type can be done in parallel.
/// The level of parallelism is decided by the runtime - i.e. it can
/// choose to provide just one element at a time, or up to some limit N,
/// or even an unbounded number.
///
/// [BatchOperation]s are provided to the error model, as it may
/// find this information pertinent. However, there is no requirement
/// that simulation of the operations itself is done in parallel; in fact,
/// the interface is currently limited to individual operations, but this
/// may change in future if it is found to be beneficial for performance.
#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Measure {
        qubit_id: u64,
        result_id: u64,
    },
    Reset {
        qubit_id: u64,
    },
    RXYGate {
        qubit_id: u64,
        theta: f64,
        phi: f64,
    },
    RZGate {
        qubit_id: u64,
        theta: f64,
    },
    RZZGate {
        qubit_id_1: u64,
        qubit_id_2: u64,
        theta: f64,
    },
    Custom {
        custom_tag: usize,
        data: Box<[u8]>,
    },
    MeasureLeaked {
        qubit_id: u64,
        result_id: u64,
    },
}

#[derive(Default, Clone, Debug)]
pub struct BatchOperation {
    ops: Vec<Operation>,
    start: crate::time::Instant,
    duration: crate::time::Duration,
}

impl BatchOperation {
    delegate! {
        to self.ops {
            pub fn len(&self) -> usize;
            pub fn is_empty(&self) -> bool;
            #[call(iter)]
            pub fn iter_ops(&self) -> impl iter::DoubleEndedIterator<Item=&Operation> + '_;
        }
    }

    pub fn start(&self) -> crate::time::Instant {
        self.start
    }

    pub fn end(&self) -> crate::time::Instant {
        self.start + self.duration
    }

    pub fn duration(&self) -> crate::time::Duration {
        self.duration
    }

    pub fn new(
        ops: Vec<Operation>,
        start: crate::time::Instant,
        duration: crate::time::Duration,
    ) -> Self {
        Self {
            ops,
            start,
            duration,
        }
    }
}

impl IntoIterator for BatchOperation {
    type Item = Operation;

    type IntoIter = <Vec<Operation> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.ops.into_iter()
    }
}

/// An instance of a runtime plugin, ready to be used for emulation.
///
/// `Runtime`'s impl [RuntimeInterface] delegates to a wrapped `dyn
/// RuntimeInterface`.
///
/// Usually the wrapped [RuntimeInterface] will be an instance
/// delegating to an external runtime plugin as defined by
/// [plugin::RuntimeInterfacePlugin)].
pub struct Runtime(Box<dyn RuntimeInterface>);

impl Runtime {
    /// Constructs a new Runtime from a [RuntimeInterfaceFactory].
    pub fn new(
        factory: sync::Arc<impl RuntimeInterfaceFactory + 'static>,
        n_qubits: u64,
        start: crate::time::Instant,
        args: &[impl AsRef<str>],
    ) -> Result<Self> {
        Ok(Self(factory.init(n_qubits, start, args)?))
    }
}

impl AsRef<dyn RuntimeInterface> for Runtime {
    fn as_ref(&self) -> &(dyn RuntimeInterface + 'static) {
        self.0.as_ref()
    }
}

impl AsMut<dyn RuntimeInterface> for Runtime {
    fn as_mut(&mut self) -> &mut (dyn RuntimeInterface + 'static) {
        &mut *self.0
    }
}

impl RuntimeInterface for Runtime {
    delegate! {
        to self.0.as_mut() {
            fn exit(&mut self) -> Result<()>;
            fn get_next_operations(&mut self) -> Result<Option<BatchOperation>>;
            fn shot_start(&mut self, shot_id: u64, seed: u64) -> Result<()>;
            fn shot_end(&mut self) -> Result<()>;
            fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, crate::utils::MetricValue)>>;
            fn qalloc(&mut self) -> Result<u64>;
            fn qfree(&mut self, qubit_id: u64) -> Result<()>;
            fn local_barrier(&mut self, qubits: &[u64], sleep_ns: u64) -> Result<()>;
            fn global_barrier(&mut self, sleep_ns: u64) -> Result<()>;
            fn rxy_gate(&mut self, qubit_id: u64, theta: f64, phi: f64) -> Result<()>;
            fn rzz_gate(&mut self, qubit_id_1: u64, qubit_id_2: u64, theta: f64) -> Result<()>;
            fn rz_gate(&mut self, qubit_id: u64, theta: f64) -> Result<()>;
            fn measure(&mut self, qubit_id: u64) -> Result<u64>;
            fn measure_leaked(&mut self, qubit_id: u64) -> Result<u64>;
            fn reset(&mut self, qubit_id: u64) -> Result<()>;
            fn force_result(&mut self, result_id: u64) -> Result<()>;
            fn get_bool_result(&mut self, result_id: u64) -> Result<Option<bool>>;
            fn set_bool_result(&mut self, result_id: u64, result: bool) -> Result<()>;
            fn get_u64_result(&mut self, result_id: u64) -> Result<Option<u64>>;
            fn set_u64_result(&mut self, result_id: u64, result: u64) -> Result<()>;
            fn increment_future_refcount(&mut self, future: u64) -> Result<()>;
            fn decrement_future_refcount(&mut self, future: u64) -> Result<()>;
            fn custom_call(&mut self, custom_tag: u64, data: &[u8]) -> Result<u64>;
        }
    }
}
