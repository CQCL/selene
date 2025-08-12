unsafe extern "C" {
    pub fn cstim_TableauSimulator64_create(
        numQubits: ::std::os::raw::c_uint,
        randomSeed: ::std::os::raw::c_ulonglong,
    ) -> *mut ::std::os::raw::c_void;

    pub fn cstim_TableauSimulator64_destroy(rawptr: *mut ::std::os::raw::c_void);

    pub fn cstim_TableauSimulator64_do_SQRT_X(
        rawptr: *mut ::std::os::raw::c_void,
        q: ::std::os::raw::c_uint,
    );

    pub fn cstim_TableauSimulator64_do_SQRT_Z(
        rawptr: *mut ::std::os::raw::c_void,
        q: ::std::os::raw::c_uint,
    );

    pub fn cstim_TableauSimulator64_do_SQRT_X_DAG(
        rawptr: *mut ::std::os::raw::c_void,
        q: ::std::os::raw::c_uint,
    );

    pub fn cstim_TableauSimulator64_do_SQRT_Z_DAG(
        rawptr: *mut ::std::os::raw::c_void,
        q: ::std::os::raw::c_uint,
    );

    pub fn cstim_TableauSimulator64_do_SQRT_ZZ(
        rawptr: *mut ::std::os::raw::c_void,
        q0: ::std::os::raw::c_uint,
        q1: ::std::os::raw::c_uint,
    );

    pub fn cstim_TableauSimulator64_do_SQRT_ZZ_DAG(
        rawptr: *mut ::std::os::raw::c_void,
        q0: ::std::os::raw::c_uint,
        q1: ::std::os::raw::c_uint,
    );

    pub fn cstim_TableauSimulator64_do_X(
        rawptr: *mut ::std::os::raw::c_void,
        q: ::std::os::raw::c_uint,
    );

    pub fn cstim_TableauSimulator64_do_Z(
        rawptr: *mut ::std::os::raw::c_void,
        q: ::std::os::raw::c_uint,
    );

    pub fn cstim_TableauSimulator64_do_H_XZ(
        rawptr: *mut ::std::os::raw::c_void,
        q: ::std::os::raw::c_uint,
    );

    pub fn cstim_TableauSimulator64_do_MZ(
        rawptr: *mut ::std::os::raw::c_void,
        q: ::std::os::raw::c_uint,
    ) -> bool;

    pub fn cstim_TableauSimulator64_do_POSTSELECT_Z(
        rawptr: *mut ::std::os::raw::c_void,
        q: ::std::os::raw::c_uint,
        target_value: bool,
    ) -> bool;
}
