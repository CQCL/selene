use crate::bindings;

pub struct TableauSimulator64 {
    ptr: *mut std::ffi::c_void,
}
impl TableauSimulator64 {
    pub fn new(num_qubits: u32, random_seed: u64) -> Self {
        Self {
            ptr: unsafe { bindings::cstim_TableauSimulator64_create(num_qubits, random_seed) },
        }
    }
    pub fn sqrt_x(&mut self, q: u32) {
        unsafe { bindings::cstim_TableauSimulator64_do_SQRT_X(self.ptr, q) }
    }
    pub fn sqrt_z(&mut self, q: u32) {
        unsafe { bindings::cstim_TableauSimulator64_do_SQRT_Z(self.ptr, q) }
    }
    pub fn sqrt_x_dag(&mut self, q: u32) {
        unsafe { bindings::cstim_TableauSimulator64_do_SQRT_X_DAG(self.ptr, q) }
    }
    pub fn sqrt_z_dag(&mut self, q: u32) {
        unsafe { bindings::cstim_TableauSimulator64_do_SQRT_Z_DAG(self.ptr, q) }
    }
    pub fn sqrt_zz(&mut self, q0: u32, q1: u32) {
        unsafe { bindings::cstim_TableauSimulator64_do_SQRT_ZZ(self.ptr, q0, q1) }
    }
    pub fn sqrt_zz_dag(&mut self, q0: u32, q1: u32) {
        unsafe { bindings::cstim_TableauSimulator64_do_SQRT_ZZ_DAG(self.ptr, q0, q1) }
    }
    pub fn x(&mut self, q: u32) {
        unsafe { bindings::cstim_TableauSimulator64_do_X(self.ptr, q) }
    }
    pub fn z(&mut self, q: u32) {
        unsafe { bindings::cstim_TableauSimulator64_do_Z(self.ptr, q) }
    }
    pub fn mz(&mut self, q: u32) -> bool {
        unsafe { bindings::cstim_TableauSimulator64_do_MZ(self.ptr, q) }
    }
    pub fn postselect_z(&mut self, q: u32, target_value: bool) -> bool {
        unsafe { bindings::cstim_TableauSimulator64_do_POSTSELECT_Z(self.ptr, q, target_value) }
    }
}

impl Drop for TableauSimulator64 {
    fn drop(&mut self) {
        unsafe { bindings::cstim_TableauSimulator64_destroy(self.ptr) }
    }
}
