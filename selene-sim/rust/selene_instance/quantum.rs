use crate::selene_instance::SeleneInstance;
use anyhow::Result;

impl SeleneInstance {
    pub fn qalloc(&mut self) -> Result<u64> {
        self.emulator.user_issued_qalloc()
    }

    pub fn qfree(&mut self, q: u64) -> Result<()> {
        self.emulator.user_issued_qfree(q)
    }

    pub fn rz(&mut self, qubit_id: u64, theta: f64) -> Result<()> {
        self.emulator.user_issued_rz(qubit_id, theta)
    }

    pub fn rxy(&mut self, qubit_id: u64, theta: f64, phi: f64) -> Result<()> {
        self.emulator.user_issued_rxy(qubit_id, theta, phi)
    }

    pub fn rzz(&mut self, qubit_id: u64, qubit_id2: u64, theta: f64) -> Result<()> {
        self.emulator.user_issued_rzz(qubit_id, qubit_id2, theta)
    }

    pub fn qubit_reset(&mut self, q: u64) -> Result<()> {
        self.emulator.user_issued_reset(q)
    }

    pub fn qubit_measure(&mut self, q: u64) -> Result<bool> {
        self.emulator.user_issued_eager_measure(q)
    }

    pub fn qubit_lazy_measure(&mut self, q: u64) -> Result<u64> {
        self.emulator.user_issued_lazy_measure(q)
    }

    pub fn global_barrier(&mut self, sleep_time: u64) -> Result<()> {
        self.emulator.user_issued_global_barrier(sleep_time)
    }
    pub fn local_barrier(&mut self, qubits: &[u64], sleep_time: u64) -> Result<()> {
        self.emulator.user_issued_local_barrier(qubits, sleep_time)
    }

    pub fn refcount_decrement(&mut self, r: u64) -> Result<()> {
        self.emulator.user_issued_decrement_measurement_refcount(r)
    }

    pub fn refcount_increment(&mut self, r: u64) -> Result<()> {
        self.emulator.user_issued_increment_measurement_refcount(r)
    }

    pub fn future_read(&mut self, r: u64) -> Result<bool> {
        self.emulator.user_issued_read_measurement(r)
    }

    pub fn custom_runtime_call(&mut self, custom_tag: u64, data: &[u8]) -> Result<u64> {
        self.emulator.custom_runtime_call(custom_tag, data)
    }
}
