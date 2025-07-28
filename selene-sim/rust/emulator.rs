use crate::event_hooks::{EventHook, MultiEventHook, Operation};
use anyhow::{Result, anyhow};
use selene_core::error_model::{ErrorModel, ErrorModelInterface};
use selene_core::runtime::{Runtime, RuntimeInterface as _};

pub struct Emulator {
    pub runtime: Runtime,
    pub error_model: ErrorModel,
    pub event_hooks: MultiEventHook,
}

// User-issued function calls
impl Emulator {
    pub fn poke(&mut self) -> Result<()> {
        self.process_runtime()
    }
    pub fn dump_quantum_state(&mut self, file: &std::path::Path, qubits: &[u64]) -> Result<()> {
        self.runtime.global_barrier(0)?;
        self.process_runtime()?;
        self.error_model.dump_simulator_state(file, qubits)
    }
    pub fn user_issued_qalloc(&mut self) -> Result<u64> {
        let address = self.runtime.qalloc()?;
        //self.user_program_metrics.increment_qalloc();
        self.event_hooks.on_user_call(&Operation::QAlloc(address));
        self.process_runtime()?;
        Ok(address)
    }
    pub fn user_issued_qfree(&mut self, address: u64) -> Result<()> {
        self.runtime.qfree(address)?;
        //self.user_program_metrics.increment_qfree();
        self.event_hooks.on_user_call(&Operation::QFree(address));
        self.process_runtime()
    }
    pub fn user_issued_local_barrier(&mut self, qubits: &[u64], sleep_time: u64) -> Result<()> {
        self.runtime.local_barrier(qubits, sleep_time)?;
        self.event_hooks
            .on_user_call(&Operation::LocalBarrier(qubits.to_vec(), sleep_time));
        self.process_runtime()
    }
    pub fn user_issued_global_barrier(&mut self, sleep_time: u64) -> Result<()> {
        self.runtime.global_barrier(sleep_time)?;
        self.event_hooks
            .on_user_call(&Operation::GlobalBarrier(sleep_time));
        self.process_runtime()
    }
    pub fn user_issued_rxy(&mut self, q0: u64, theta: f64, phi: f64) -> Result<()> {
        self.runtime.rxy_gate(q0, theta, phi)?;
        //self.user_program_metrics.increment_rxy();
        self.event_hooks
            .on_user_call(&Operation::RXY(q0, theta, phi));
        self.process_runtime()
    }
    pub fn user_issued_rzz(&mut self, q0: u64, q1: u64, theta: f64) -> Result<()> {
        self.runtime.rzz_gate(q0, q1, theta)?;
        //self.user_program_metrics.increment_rzz();
        self.event_hooks
            .on_user_call(&Operation::RZZ(q0, q1, theta));
        self.process_runtime()
    }
    pub fn user_issued_rz(&mut self, q0: u64, theta: f64) -> Result<()> {
        self.runtime.rz_gate(q0, theta)?;
        //self.user_program_metrics.increment_rz();
        self.event_hooks.on_user_call(&Operation::RZ(q0, theta));
        self.process_runtime()
    }
    pub fn user_issued_reset(&mut self, q0: u64) -> Result<()> {
        self.runtime.reset(q0)?;
        //self.user_program_metrics.increment_reset();
        self.event_hooks.on_user_call(&Operation::Reset(q0));
        self.process_runtime()
    }
    pub fn user_issued_lazy_measure(&mut self, q0: u64) -> Result<u64> {
        let result_id = self.runtime.measure(q0)?;
        self.event_hooks
            .on_user_call(&Operation::MeasureRequest(q0));
        self.process_runtime()?;
        Ok(result_id)
    }
    pub fn user_issued_lazy_measure_leaked(&mut self, q0: u64) -> Result<u64> {
        let result_id = self.runtime.measure_leaked(q0)?;
        self.event_hooks
            .on_user_call(&Operation::MeasureLeakedRequest(q0));
        self.process_runtime()?;
        Ok(result_id)
    }
    pub fn user_issued_eager_measure(&mut self, q0: u64) -> Result<bool> {
        let result_id = self.user_issued_lazy_measure(q0)?;
        self.user_issued_read_future_bool(result_id)
    }
    pub fn user_issued_increment_measurement_refcount(&mut self, result_id: u64) -> Result<()> {
        self.runtime.increment_future_refcount(result_id).unwrap();
        self.process_runtime()
    }
    pub fn user_issued_decrement_measurement_refcount(&mut self, result_id: u64) -> Result<()> {
        self.runtime.decrement_future_refcount(result_id).unwrap();
        self.process_runtime()
    }
    pub fn user_issued_read_future_bool(&mut self, result_id: u64) -> Result<bool> {
        self.event_hooks
            .on_user_call(&Operation::FutureRead(result_id));
        match self.runtime.get_bool_result(result_id)? {
            Some(value) => Ok(value),
            None => {
                self.runtime.force_result(result_id)?;
                self.process_runtime()?;
                let Some(result) = self.runtime.get_bool_result(result_id)? else {
                    return Err(anyhow!(
                        "Future bool result not available after attempting to force it."
                    ));
                };
                Ok(result)
            }
        }
    }
    pub fn user_issued_read_future_u64(&mut self, result_id: u64) -> Result<u64> {
        self.event_hooks
            .on_user_call(&Operation::FutureRead(result_id));
        match self.runtime.get_u64_result(result_id)? {
            Some(value) => Ok(value),
            None => {
                self.runtime.force_result(result_id)?;
                self.process_runtime()?;
                let Some(result) = self.runtime.get_u64_result(result_id)? else {
                    return Err(anyhow!(
                        "Future u64 result not available after attempting to force it."
                    ));
                };
                Ok(result)
            }
        }
    }

    pub fn custom_runtime_call(&mut self, tag: u64, data: &[u8]) -> Result<u64> {
        let result = self.runtime.custom_call(tag, data)?;
        self.process_runtime()?;
        Ok(result)
    }
}

// Handling of runtime-issued instructions
impl Emulator {
    fn process_runtime(&mut self) -> Result<()> {
        while let Some(batch) = self.runtime.get_next_operations()? {
            self.event_hooks.on_runtime_batch(&batch);
            //self.post_runtime_metrics.update(&batch);
            let results = self.error_model.handle_operations(batch)?;
            for bool_result in results.bool_results {
                self.runtime
                    .set_bool_result(bool_result.result_id, bool_result.value)?;
            }
            for u64_result in results.u64_results {
                self.runtime
                    .set_u64_result(u64_result.result_id, u64_result.value)?;
            }
        }
        Ok(())
    }
}
