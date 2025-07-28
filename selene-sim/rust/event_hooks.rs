use selene_core::encoder::{OutputStream, OutputStreamError};
use selene_core::runtime::BatchOperation;

pub mod instruction_log;
pub mod metrics;

#[derive(Clone)]
pub enum Operation {
    QFree(u64),
    QAlloc(u64),
    RXY(u64, f64, f64),
    RZZ(u64, u64, f64),
    RZ(u64, f64),
    Reset(u64),
    MeasureRequest(u64),
    MeasureLeakedRequest(u64),
    FutureRead(u64),
    BatchStart(u64, u64),
    GlobalBarrier(u64),
    LocalBarrier(Vec<u64>, u64),
    Custom(u64, Vec<u8>),
}

pub trait EventHook {
    fn on_user_call(&mut self, _: &Operation) {}
    fn on_runtime_batch(&mut self, _: &BatchOperation) {}
    fn write(
        &self,
        _time_cursor: u64,
        _encoder: &mut OutputStream,
    ) -> Result<(), OutputStreamError> {
        Ok(())
    }
    fn on_shot_start(&mut self, _shot_id: u64) {}
    fn on_shot_end(&mut self) {}
}

#[derive(Default)]
pub struct MultiEventHook {
    hooks: Vec<Box<dyn EventHook>>,
}
impl MultiEventHook {
    pub fn add_hook(&mut self, hook: Box<dyn EventHook>) {
        self.hooks.push(hook);
    }
}

impl EventHook for MultiEventHook {
    fn on_user_call(&mut self, operation: &Operation) {
        for hook in self.hooks.iter_mut() {
            hook.on_user_call(operation);
        }
    }
    fn on_runtime_batch(&mut self, operation: &BatchOperation) {
        for hook in self.hooks.iter_mut() {
            hook.on_runtime_batch(operation);
        }
    }
    fn write(&self, time_cursor: u64, encoder: &mut OutputStream) -> Result<(), OutputStreamError> {
        for hook in self.hooks.iter() {
            hook.write(time_cursor, encoder)?;
        }
        Ok(())
    }
    fn on_shot_start(&mut self, shot_id: u64) {
        for hook in self.hooks.iter_mut() {
            hook.on_shot_start(shot_id);
        }
    }
    fn on_shot_end(&mut self) {
        for hook in self.hooks.iter_mut() {
            hook.on_shot_end();
        }
    }
}
