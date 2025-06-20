use std::collections::VecDeque;

use anyhow::{bail, Result};
use selene_core::{
    export_runtime_plugin,
    runtime::{interface::RuntimeInterfaceFactory, BatchOperation, Operation, RuntimeInterface},
    utils::MetricValue,
};

#[derive(Debug, Clone, PartialEq)]
enum QubitStatus {
    Free,
    Active { phase: f64 },
}
#[derive(Debug, Clone)]
struct Measurement {
    measured: bool,
    value: bool,
}

struct ExampleRuntime {
    qubits: Vec<QubitStatus>,
    operation_queue: VecDeque<BatchOperation>,
    flush_size: usize,
    measurements: Vec<Measurement>,
    start: selene_core::time::Instant,
}

impl ExampleRuntime {
    pub fn new(n_qubits: u64, start: selene_core::time::Instant) -> Self {
        Self {
            qubits: vec![QubitStatus::Free; n_qubits as usize],
            operation_queue: VecDeque::with_capacity(10000),
            flush_size: 0,
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

impl RuntimeInterface for ExampleRuntime {
    fn exit(&mut self) -> Result<()> {
        self.operation_queue.clear();
        self.qubits.clear();
        self.flush_size = 0;
        self.measurements.clear();
        Ok(())
    }
    // Engine ops
    fn get_next_operations(&mut self) -> Result<Option<BatchOperation>> {
        debug_assert!(
            self.flush_size <= self.operation_queue.len(),
            "flush size is greater than operation queue length"
        );
        if self.flush_size == 0 {
            return Ok(None);
        }
        self.flush_size -= 1;
        Ok(self.operation_queue.pop_front())
    }

    fn shot_start(&mut self, _shot_id: u64, _seed: u64) -> Result<()> {
        Ok(())
    }
    fn shot_end(&mut self) -> Result<()> {
        self.qubits = vec![QubitStatus::Free; self.qubits.len()];
        self.operation_queue.clear();
        self.flush_size = 0;
        self.measurements.clear();
        Ok(())
    }
    fn global_barrier(&mut self, _sleep_ns: u64) -> Result<()> {
        self.flush_size = self.operation_queue.len();
        Ok(())
    }
    fn local_barrier(&mut self, qubits: &[u64], _sleep_ns: u64) -> Result<()> {
        let mut last_op_using_qubits = 0;
        for (i, operation) in self
            .operation_queue
            .iter()
            .skip(self.flush_size)
            .enumerate()
        {
            for op in operation.iter_ops() {
                match op {
                    Operation::RXYGate { qubit_id, .. } => {
                        if qubits.contains(qubit_id) {
                            last_op_using_qubits = i;
                        }
                    }
                    Operation::RZGate { qubit_id, .. } => {
                        if qubits.contains(qubit_id) {
                            last_op_using_qubits = i;
                        }
                    }
                    Operation::RZZGate {
                        qubit_id_1,
                        qubit_id_2,
                        ..
                    } => {
                        if qubits.contains(qubit_id_1) || qubits.contains(qubit_id_2) {
                            last_op_using_qubits = i;
                        }
                    }
                    Operation::Measure { qubit_id, .. } => {
                        if qubits.contains(qubit_id) {
                            last_op_using_qubits = i;
                        }
                    }
                    Operation::Reset { qubit_id, .. } => {
                        if qubits.contains(qubit_id) {
                            last_op_using_qubits = i;
                        }
                    }
                    Operation::Custom { .. } => {}
                }
            }
        }
        self.flush_size = std::cmp::max(self.flush_size, last_op_using_qubits + 1);
        Ok(())
    }
    // Allocation
    fn qalloc(&mut self) -> Result<u64> {
        for (i, qubit) in self.qubits.iter_mut().enumerate() {
            if *qubit == QubitStatus::Free {
                *qubit = QubitStatus::Active { phase: 0.0 };
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
        let QubitStatus::Active { phase } = self.qubits[qubit_id as usize] else {
            bail!("Qubit {qubit_id} is not active");
        };
        self.push(Operation::RXYGate {
            qubit_id,
            theta,
            phi: phi - phase, // The Z phase is enacted here.
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
        let QubitStatus::Active { phase } = self.qubits[qubit_id as usize] else {
            bail!("Qubit {qubit_id} is not active");
        };
        // We don't apply an RZ gate. Instead, we accumulate a phase, and mutate
        // RXY gates' phi parameters to account for the phase shift. RZZ and measurement
        // are unaffected.
        self.qubits[qubit_id as usize] = QubitStatus::Active {
            phase: phase + theta,
        };
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
        for (i, operation) in self.operation_queue.iter().enumerate().rev() {
            for op in operation.iter_ops() {
                if let Operation::Measure {
                    result_id: measure_result_id,
                    ..
                } = op
                {
                    if result_id == *measure_result_id {
                        self.flush_size = std::cmp::max(self.flush_size, i + 1);
                        return Ok(());
                    }
                }
            }
        }
        bail!("No measurement operation with result {result_id} found")
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
struct ExampleRuntimeFactory;

impl RuntimeInterfaceFactory for ExampleRuntimeFactory {
    type Interface = ExampleRuntime;

    fn init(
        self: std::sync::Arc<Self>,
        n_qubits: u64,
        start: selene_core::time::Instant,
        _args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        Ok(Box::new(ExampleRuntime::new(n_qubits, start)))
    }
}

export_runtime_plugin!(crate::ExampleRuntimeFactory);
