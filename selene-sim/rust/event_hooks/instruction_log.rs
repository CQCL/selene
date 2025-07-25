use crate::event_hooks::{EventHook, Operation};
use selene_core::encoder::{OutputStream, OutputStreamError};
use selene_core::runtime::{self, BatchOperation};

pub struct Instruction {
    pub source: Source,
    pub operation: Operation,
}
#[derive(Clone)]
#[repr(u64)]
pub enum Source {
    UserProgram = 0,
    RuntimeOptimiser = 1,
    ErrorModel = 2,
}

impl Instruction {
    pub fn write(&self, encoder: &mut OutputStream) -> Result<(), OutputStreamError> {
        let source_id: u64 = self.source.clone() as u64;
        encoder.write(source_id)?;
        match &self.operation {
            Operation::BatchStart(start_time, duration) => {
                encoder.write(0u64)?;
                encoder.write(*start_time)?;
                encoder.write(*duration)?;
            }
            Operation::QAlloc(address) => {
                encoder.write(1u64)?;
                encoder.write(*address)?;
            }
            Operation::QFree(address) => {
                encoder.write(2u64)?;
                encoder.write(*address)?;
            }
            Operation::Reset(qubit1) => {
                encoder.write(3u64)?;
                encoder.write(*qubit1)?;
            }
            Operation::MeasureRequest(qubit1) => {
                encoder.write(4u64)?;
                encoder.write(*qubit1)?;
            }
            Operation::FutureRead(qubit1) => {
                encoder.write(5u64)?;
                encoder.write(*qubit1)?;
            }
            Operation::RXY(qubit1, angle1, angle2) => {
                encoder.write(6u64)?;
                encoder.write(*qubit1)?;
                encoder.write(*angle1)?;
                encoder.write(*angle2)?;
            }
            Operation::RZ(qubit1, angle) => {
                encoder.write(7u64)?;
                encoder.write(*qubit1)?;
                encoder.write(*angle)?;
            }
            Operation::RZZ(qubit1, qubit2, angle) => {
                encoder.write(8u64)?;
                encoder.write(*qubit1)?;
                encoder.write(*qubit2)?;
                encoder.write(*angle)?;
            }
            Operation::Custom(tag, data) => {
                encoder.write(9u64)?;
                encoder.write(*tag)?;
                encoder.write(&**data)?;
            }
            Operation::LocalBarrier(qubits, sleep_time) => {
                encoder.write(10u64)?;
                encoder.write(qubits.len() as u64)?;
                for qubit in qubits.iter() {
                    encoder.write(*qubit)?;
                }
                encoder.write(*sleep_time)?;
            }
            Operation::GlobalBarrier(sleep_time) => {
                encoder.write(11u64)?;
                encoder.write(*sleep_time)?;
            }
            Operation::MeasureLeakedRequest(qubit1) => {
                encoder.write(12u64)?;
                encoder.write(*qubit1)?;
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct InstructionLog {
    entries: Vec<Instruction>,
}

impl EventHook for InstructionLog {
    fn on_user_call(&mut self, operation: &Operation) {
        self.entries.push(Instruction {
            source: Source::UserProgram,
            operation: operation.clone(),
        });
    }
    fn on_runtime_batch(&mut self, batch: &BatchOperation) {
        let start = u64::from(batch.start());
        let duration = u64::from(batch.duration());
        self.entries.push(Instruction {
            source: Source::RuntimeOptimiser,
            operation: Operation::BatchStart(start, duration),
        });
        for op in batch.iter_ops() {
            let operation = match op {
                runtime::Operation::Reset { qubit_id } => Operation::Reset(*qubit_id),
                runtime::Operation::RXYGate {
                    qubit_id,
                    theta,
                    phi,
                } => Operation::RXY(*qubit_id, *theta, *phi),
                runtime::Operation::RZZGate {
                    qubit_id_1,
                    qubit_id_2,
                    theta,
                } => Operation::RZZ(*qubit_id_1, *qubit_id_2, *theta),
                runtime::Operation::RZGate { qubit_id, theta } => Operation::RZ(*qubit_id, *theta),
                runtime::Operation::Measure { qubit_id, .. } => Operation::FutureRead(*qubit_id),
                runtime::Operation::MeasureLeaked { qubit_id, .. } => {
                    Operation::FutureRead(*qubit_id)
                }
                runtime::Operation::Custom { custom_tag, data } => {
                    Operation::Custom(*custom_tag as u64, data.to_vec())
                }
            };
            self.entries.push(Instruction {
                source: Source::RuntimeOptimiser,
                operation,
            });
        }
    }
    fn write(&self, time_cursor: u64, encoder: &mut OutputStream) -> Result<(), OutputStreamError> {
        encoder.begin_message(time_cursor)?;
        encoder.write("INSTRUCTIONLOG")?;
        for instruction in self.entries.iter() {
            instruction.write(encoder)?;
        }
        encoder.end_message()?;
        Ok(())
    }
    fn on_shot_start(&mut self, _shot_id: u64) {
        self.entries.clear();
    }
    fn on_shot_end(&mut self) {}
}
