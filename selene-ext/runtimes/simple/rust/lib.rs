use std::collections::VecDeque;

use anyhow::{Result, bail};
use selene_core::{
    export_runtime_plugin,
    runtime::{BatchOperation, Operation, RuntimeInterface, interface::RuntimeInterfaceFactory},
    utils::MetricValue,
};

#[derive(Debug, Clone, PartialEq)]
enum QubitStatus {
    Free,
    Active,
}
#[derive(Debug, Clone)]
struct Measurement {
    measured: bool,
    value: bool,
}

struct SimpleRuntime {
    qubits: Vec<QubitStatus>,
    operation_queue: VecDeque<BatchOperation>,
    measurements: Vec<Measurement>,
    start: selene_core::time::Instant,
}

impl SimpleRuntime {
    pub fn new(n_qubits: u64, start: selene_core::time::Instant) -> Self {
        Self {
            qubits: vec![QubitStatus::Free; n_qubits as usize],
            operation_queue: VecDeque::with_capacity(10000),
            measurements: Vec::with_capacity(1000),
            start,
        }
    }

    pub fn push(&mut self, op: Operation) {
        self.operation_queue.push_back(BatchOperation::new(
            vec![op],
            self.start,
            Default::default(),
        ));
    }
}

impl RuntimeInterface for SimpleRuntime {
    fn exit(&mut self) -> Result<()> {
        self.operation_queue.clear();
        self.qubits.clear();
        self.measurements.clear();
        Ok(())
    }
    // Engine ops
    fn get_next_operations(&mut self) -> Result<Option<BatchOperation>> {
        Ok(self.operation_queue.pop_front())
    }

    fn shot_start(&mut self, _shot_id: u64, _seed: u64) -> Result<()> {
        Ok(())
    }
    fn shot_end(&mut self) -> Result<()> {
        self.qubits = vec![QubitStatus::Free; self.qubits.len()];
        self.operation_queue.clear();
        self.measurements.clear();
        Ok(())
    }
    fn global_barrier(&mut self, _sleep_ns: u64) -> Result<()> {
        // This runtime isn't lazy, so a barrier is not relevant
        // to its operation.
        Ok(())
    }
    fn local_barrier(&mut self, _qubits: &[u64], _sleep_ns: u64) -> Result<()> {
        // This runtime isn't lazy, so a barrier is not relevant
        // to its operation.
        Ok(())
    }
    // Allocation
    fn qalloc(&mut self) -> Result<u64> {
        for (i, qubit) in self.qubits.iter_mut().enumerate() {
            if *qubit == QubitStatus::Free {
                *qubit = QubitStatus::Active;
                return Ok(i as u64);
            }
        }
        Ok(u64::MAX)
    }
    fn qfree(&mut self, qubit_id: u64) -> Result<()> {
        if qubit_id >= self.qubits.len() as u64 {
            bail!("freeing out-of-bounds qubit {qubit_id}")
        } else {
            self.qubits[qubit_id as usize] = QubitStatus::Free;
            Ok(())
        }
    }
    fn rxy_gate(&mut self, qubit_id: u64, theta: f64, phi: f64) -> Result<()> {
        if qubit_id >= self.qubits.len() as u64 {
            bail!("applying rxy gate to out-of-bounds qubit {qubit_id}");
        }
        let QubitStatus::Active = self.qubits[qubit_id as usize] else {
            bail!("Qubit {qubit_id} is not active");
        };
        self.push(Operation::RXYGate {
            qubit_id,
            theta,
            phi,
        });
        Ok(())
    }
    fn rzz_gate(&mut self, qubit_id_1: u64, qubit_id_2: u64, theta: f64) -> Result<()> {
        if qubit_id_1 >= self.qubits.len() as u64 {
            bail!("applying rzz gate to out-of-bounds qubit1 {qubit_id_1}");
        }
        if qubit_id_2 >= self.qubits.len() as u64 {
            bail!("applying rzz gate to out-of-bounds qubit2 {qubit_id_2}");
        }
        self.push(Operation::RZZGate {
            qubit_id_1,
            qubit_id_2,
            theta,
        });
        Ok(())
    }
    fn rz_gate(&mut self, qubit_id: u64, theta: f64) -> Result<()> {
        if qubit_id >= self.qubits.len() as u64 {
            bail!("applying rz gate to out-of-bounds qubit {qubit_id}");
        }
        let QubitStatus::Active = self.qubits[qubit_id as usize] else {
            bail!("Qubit {qubit_id} is not active");
        };
        self.push(Operation::RZGate { qubit_id, theta });
        Ok(())
    }
    // Lifetime ops
    fn measure(&mut self, qubit_id: u64) -> Result<u64> {
        if qubit_id >= self.qubits.len() as u64 {
            bail!("measuring out-of-bounds qubit {qubit_id}")
        }
        let result_id = self.measurements.len() as u64;
        self.measurements.push(Measurement {
            measured: false,
            value: false,
        });
        self.push(Operation::Measure {
            qubit_id,
            result_id,
        });
        Ok(result_id)
    }
    fn reset(&mut self, qubit_id: u64) -> Result<()> {
        if qubit_id >= self.qubits.len() as u64 {
            bail!("resetting out-of-bounds qubit {qubit_id}")
        }
        self.push(Operation::Reset { qubit_id });
        Ok(())
    }
    fn force_result(&mut self, result_id: u64) -> Result<()> {
        if result_id >= self.measurements.len() as u64 {
            bail!("forcing out-of-bounds measurement {result_id}")
        }
        // This runtime isn't lazy, so if a result has been defined,
        // the measurement should already be done.
        Ok(())
    }
    fn get_result(&mut self, result_id: u64) -> Result<Option<bool>> {
        if result_id >= self.measurements.len() as u64 {
            bail!("getting out-of-bounds measurement {result_id}");
        }
        let result = &self.measurements[result_id as usize];
        Ok(if result.measured {
            Some(result.value)
        } else {
            None
        })
    }
    fn set_result(&mut self, result_id: u64, result: bool) -> Result<()> {
        if result_id >= self.measurements.len() as u64 {
            bail!("setting out-of-bounds measurement {result_id}");
        }
        self.measurements[result_id as usize].value = result;
        self.measurements[result_id as usize].measured = true;
        Ok(())
    }
    fn increment_future_refcount(&mut self, _future_ref: u64) -> Result<()> {
        Ok(())
    }
    fn decrement_future_refcount(&mut self, _future_ref: u64) -> Result<()> {
        Ok(())
    }
    fn get_metric(&mut self, _nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        Ok(None)
    }
}

#[derive(Default)]
struct SimpleRuntimeFactory;

impl RuntimeInterfaceFactory for SimpleRuntimeFactory {
    type Interface = SimpleRuntime;

    fn init(
        self: std::sync::Arc<Self>,
        n_qubits: u64,
        start: selene_core::time::Instant,
        _args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        Ok(Box::new(SimpleRuntime::new(n_qubits, start)))
    }
}

export_runtime_plugin!(crate::SimpleRuntimeFactory);
