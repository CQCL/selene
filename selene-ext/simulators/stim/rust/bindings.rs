use std::os::raw::{c_char, c_uint, c_ulonglong, c_void};
unsafe extern "C" {
    pub fn cstim_TableauSimulator64_create(
        numQubits: c_uint,
        randomSeed: c_ulonglong,
    ) -> *mut c_void;

    pub fn cstim_TableauSimulator64_destroy(rawptr: *mut c_void);

    pub fn cstim_TableauSimulator64_do_SQRT_X(rawptr: *mut c_void, q: c_uint);

    pub fn cstim_TableauSimulator64_do_SQRT_Z(rawptr: *mut c_void, q: c_uint);

    pub fn cstim_TableauSimulator64_do_SQRT_X_DAG(rawptr: *mut c_void, q: c_uint);

    pub fn cstim_TableauSimulator64_do_SQRT_Z_DAG(rawptr: *mut c_void, q: c_uint);

    pub fn cstim_TableauSimulator64_do_SQRT_ZZ(rawptr: *mut c_void, q0: c_uint, q1: c_uint);

    pub fn cstim_TableauSimulator64_do_SQRT_ZZ_DAG(rawptr: *mut c_void, q0: c_uint, q1: c_uint);

    pub fn cstim_TableauSimulator64_do_X(rawptr: *mut c_void, q: c_uint);

    pub fn cstim_TableauSimulator64_do_Z(rawptr: *mut c_void, q: c_uint);

    pub fn cstim_TableauSimulator64_do_H_XZ(rawptr: *mut c_void, q: c_uint);

    pub fn cstim_TableauSimulator64_do_MZ(rawptr: *mut c_void, q: c_uint) -> bool;

    pub fn cstim_TableauSimulator64_do_POSTSELECT_Z(
        rawptr: *mut c_void,
        q: c_uint,
        target_value: bool,
    ) -> bool;

    pub fn cstim_TableauSimulator64_get_stabilizers(rawptr: *mut c_void, write: *mut *mut c_char);
    pub fn cstim_TableauSimulator64_free_stabilizers(written: *mut c_char);
}
