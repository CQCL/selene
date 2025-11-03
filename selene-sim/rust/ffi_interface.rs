use super::selene_instance::SeleneInstance;
use crate::selene_instance::configuration::Configuration;
use anyhow::Result;

#[repr(C)]
pub struct VoidResult {
    pub error_code: u32,
}
impl VoidResult {
    pub fn ok() -> Self {
        VoidResult { error_code: 0 }
    }
    pub fn err(error_code: u32) -> Self {
        VoidResult { error_code }
    }
}
#[repr(C)]
pub struct U64Result {
    pub error_code: u32,
    pub value: u64,
}
impl U64Result {
    pub fn ok(value: u64) -> Self {
        U64Result {
            error_code: 0,
            value,
        }
    }
    pub fn err(error_code: u32) -> Self {
        U64Result {
            error_code,
            value: 0,
        }
    }
}
#[repr(C)]
pub struct U32Result {
    pub error_code: u32,
    pub value: u32,
}
impl U32Result {
    pub fn ok(value: u32) -> Self {
        U32Result {
            error_code: 0,
            value,
        }
    }
    pub fn err(error_code: u32) -> Self {
        U32Result {
            error_code,
            value: 0,
        }
    }
}
#[repr(C)]
pub struct F64Result {
    pub error_code: u32,
    pub value: f64,
}
impl F64Result {
    pub fn ok(value: f64) -> Self {
        F64Result {
            error_code: 0,
            value,
        }
    }
    pub fn err(error_code: u32) -> Self {
        F64Result {
            error_code,
            value: 0.0,
        }
    }
}
#[repr(C)]
pub struct BoolResult {
    pub error_code: u32,
    pub value: bool,
}
impl BoolResult {
    pub fn ok(value: bool) -> Self {
        BoolResult {
            error_code: 0,
            value,
        }
    }
    pub fn err(error_code: u32) -> Self {
        BoolResult {
            error_code,
            value: false,
        }
    }
}
#[repr(C)]
pub struct FutureResult {
    pub error_code: u32,
    pub reference: u64,
}
impl FutureResult {
    pub fn ok(reference: u64) -> Self {
        FutureResult {
            error_code: 0,
            reference,
        }
    }
    pub fn err(error_code: u32) -> Self {
        FutureResult {
            error_code,
            reference: 0,
        }
    }
}

#[repr(C)]
pub struct WrappedString {
    pub data: *const std::ffi::c_char,
    pub length: u64,
    pub owned: bool,
}
impl std::str::FromStr for WrappedString {
    type Err = std::ffi::NulError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let c_str = std::ffi::CString::new(s)?;
        let length = c_str.as_bytes().len() as u64;
        let data = c_str.into_raw();
        Ok(WrappedString {
            data,
            length,
            owned: true,
        })
    }
}
impl WrappedString {
    pub fn to_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.data as *const u8, self.length as usize) }
    }
    pub fn to_str(&self) -> &str {
        let slice = self.to_slice();
        std::str::from_utf8(slice).unwrap()
    }
}
impl std::fmt::Display for WrappedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_load_config(
    instance: *mut *mut SeleneInstance,
    config_file: *const std::ffi::c_char,
) -> VoidResult {
    let config_file = match unsafe { std::ffi::CStr::from_ptr(config_file) }.to_str() {
        Ok(config_file) => config_file,
        Err(e) => {
            eprintln!("Error parsing config file path: {e}");
            return VoidResult::err(1);
        }
    };
    let config_file_handle = match std::fs::File::open(config_file) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error opening config file: {e}");
            return VoidResult::err(1);
        }
    };
    let config: Configuration = match serde_yml::from_reader(config_file_handle) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error parsing config file: {e}");
            return VoidResult::err(1);
        }
    };

    let new_instance = match SeleneInstance::new(config) {
        Ok(instance) => Box::into_raw(Box::new(instance)),
        Err(e) => {
            eprintln!("Error creating selene instance: {e}");
            return VoidResult::err(1);
        }
    };
    unsafe {
        *instance = new_instance;
    }
    VoidResult::ok()
}

fn with_instance_void<F>(instance: *mut SeleneInstance, f: F) -> VoidResult
where
    F: FnOnce(&mut SeleneInstance) -> Result<()>,
{
    if instance.is_null() {
        return VoidResult::err(100000);
    }
    let instance = unsafe { &mut *instance };
    if let Err(e) = f(instance) {
        let code = 100001;
        instance.fallible_print_panic(format!("{e:#}").as_str(), code);
        VoidResult::err(code)
    } else {
        VoidResult::ok()
    }
}
fn with_instance_bool<F>(instance: *mut SeleneInstance, f: F) -> BoolResult
where
    F: FnOnce(&mut SeleneInstance) -> Result<bool>,
{
    if instance.is_null() {
        return BoolResult::err(100000);
    }
    let instance = unsafe { &mut *instance };
    match f(instance) {
        Ok(value) => BoolResult::ok(value),
        Err(e) => {
            let code = 100001;
            instance.fallible_print_panic(format!("{e:#}").as_str(), code);
            BoolResult::err(code)
        }
    }
}

fn with_instance_u64<F>(instance: *mut SeleneInstance, f: F) -> U64Result
where
    F: FnOnce(&mut SeleneInstance) -> anyhow::Result<u64>,
{
    if instance.is_null() {
        return U64Result::err(100000);
    }
    let instance = unsafe { &mut *instance };
    match f(instance) {
        Ok(value) => U64Result::ok(value),
        Err(e) => {
            let code = 100001;
            instance.fallible_print_panic(format!("{e:#}").as_str(), code);
            U64Result::err(code)
        }
    }
}
fn with_instance_f64<F>(instance: *mut SeleneInstance, f: F) -> F64Result
where
    F: FnOnce(&mut SeleneInstance) -> anyhow::Result<f64>,
{
    if instance.is_null() {
        return F64Result::err(100000);
    }
    let instance = unsafe { &mut *instance };
    match f(instance) {
        Ok(value) => F64Result::ok(value),
        Err(e) => {
            let code = 100001;
            instance.fallible_print_panic(format!("{e:#}").as_str(), code);
            F64Result::err(code)
        }
    }
}

fn with_instance_u32<F>(instance: *mut SeleneInstance, f: F) -> U32Result
where
    F: FnOnce(&mut SeleneInstance) -> Result<u32>,
{
    if instance.is_null() {
        return U32Result::err(100000);
    }
    let instance = unsafe { &mut *instance };
    match f(instance) {
        Ok(value) => U32Result::ok(value),
        Err(e) => {
            let code = 100001;
            instance.fallible_print_panic(format!("{e:#}").as_str(), code);
            U32Result::err(code)
        }
    }
}

fn with_instance_future_bool<F>(instance: *mut SeleneInstance, f: F) -> FutureResult
where
    F: FnOnce(&mut SeleneInstance) -> Result<u64>,
{
    if instance.is_null() {
        return FutureResult::err(100000);
    }
    let instance = unsafe { &mut *instance };
    match f(instance) {
        Ok(value) => FutureResult::ok(value),
        Err(e) => {
            let code = 100001;
            instance.fallible_print_panic(format!("{e:#}").as_str(), code);
            FutureResult::err(code)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_exit(instance: *mut SeleneInstance) -> VoidResult {
    if instance.is_null() {
        eprintln!("Error: selene_exit called with null state");
        return VoidResult { error_code: 1 };
    }
    let mut instance = unsafe { Box::from_raw(instance) };
    if let Err(e) = instance.exit() {
        eprintln!("Error: {e}");
        return VoidResult { error_code: 1 };
    }
    // instance lifetime ends here
    VoidResult { error_code: 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_shot_count(instance: *mut SeleneInstance) -> U64Result {
    with_instance_u64(instance, |instance| Ok(instance.config.shots.count))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_get_current_shot(instance: *mut SeleneInstance) -> U64Result {
    with_instance_u64(instance, |instance| Ok(instance.shot_number))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_on_shot_start(
    instance: *mut SeleneInstance,
    shot_index: u64,
) -> VoidResult {
    with_instance_void(instance, |instance| instance.shot_start(shot_index))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_on_shot_end(instance: *mut SeleneInstance) -> VoidResult {
    with_instance_void(instance, |instance| instance.shot_end())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_print_bool(
    instance: *mut SeleneInstance,
    tag: WrappedString,
    value: bool,
) -> VoidResult {
    with_instance_void(instance, |instance| instance.print(tag.to_str(), value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_print_i64(
    instance: *mut SeleneInstance,
    tag: WrappedString,
    value: i64,
) -> VoidResult {
    with_instance_void(instance, |instance| instance.print(tag.to_str(), value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_print_u64(
    instance: *mut SeleneInstance,
    tag: WrappedString,
    value: u64,
) -> VoidResult {
    with_instance_void(instance, |instance| instance.print(tag.to_str(), value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_print_f64(
    instance: *mut SeleneInstance,
    tag: WrappedString,
    value: f64,
) -> VoidResult {
    with_instance_void(instance, |instance| instance.print(tag.to_str(), value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_print_bool_array(
    instance: *mut SeleneInstance,
    tag: WrappedString,
    ptr: *const bool,
    length: u64,
) -> VoidResult {
    let value = unsafe { std::slice::from_raw_parts(ptr, length as usize) };
    with_instance_void(instance, |instance| instance.print(tag.to_str(), value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_print_i64_array(
    instance: *mut SeleneInstance,
    tag: WrappedString,
    ptr: *const i64,
    length: u64,
) -> VoidResult {
    let value = unsafe { std::slice::from_raw_parts(ptr, length as usize) };
    with_instance_void(instance, |instance| instance.print(tag.to_str(), value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_print_u64_array(
    instance: *mut SeleneInstance,
    tag: WrappedString,
    ptr: *const u64,
    length: u64,
) -> VoidResult {
    let value = unsafe { std::slice::from_raw_parts(ptr, length as usize) };
    with_instance_void(instance, |instance| instance.print(tag.to_str(), value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_print_f64_array(
    instance: *mut SeleneInstance,
    tag: WrappedString,
    ptr: *const f64,
    length: u64,
) -> VoidResult {
    let value = unsafe { std::slice::from_raw_parts(ptr, length as usize) };
    with_instance_void(instance, |instance| instance.print(tag.to_str(), value))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_print_panic(
    instance: *mut SeleneInstance,
    message: WrappedString,
    error_code: u32,
) -> VoidResult {
    with_instance_void(instance, |instance| {
        instance.print_panic(message.to_str(), error_code)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_print_exit(
    instance: *mut SeleneInstance,
    message: WrappedString,
    error_code: u32,
) -> VoidResult {
    with_instance_void(instance, |instance| {
        instance.print_exit(message.to_str(), error_code)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_dump_state(
    instance: *mut SeleneInstance,
    message: WrappedString,
    qubits: *const u64,
    qubits_length: u64,
) -> VoidResult {
    let message = message.to_str();
    let qubits = unsafe { std::slice::from_raw_parts(qubits, qubits_length as usize) };
    with_instance_void(instance, |instance| instance.dump_state(message, qubits))
}

/// Seeds the PRNG with a user-provided seed.
///
/// If PRNG already has state, it will be overwritten
/// by this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_random_seed(
    instance: *mut SeleneInstance,
    seed: u64,
) -> VoidResult {
    with_instance_void(instance, |instance| {
        instance.random_seed(seed);
        Ok(())
    })
}

/// Advance the PRNG with a user-provided delta. As this
/// is cyclic, i64s with negative values have well defined
/// rewinding behaviour.
///
/// Requires the PRNG to be seeded with random_seed,
/// otherwise an error will be returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_random_advance(
    instance: *mut SeleneInstance,
    delta: u64,
) -> VoidResult {
    with_instance_void(instance, |instance| instance.random_advance(delta))
}

/// Produces a random 32-bit unsigned integer.
///
/// Requires the PRNG to be seeded with random_seed,
/// otherwise an error will be returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_random_u32(instance: *mut SeleneInstance) -> U32Result {
    with_instance_u32(instance, |instance| instance.random_u32())
}

/// Produces a random 32-bit float.
///
/// Requires the PRNG to be seeded with random_seed,
/// otherwise an error will be returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_random_f64(instance: *mut SeleneInstance) -> F64Result {
    with_instance_f64(instance, |instance| instance.random_f64())
}

/// Produces a bounded random 32-bit unsigned integer.
///
/// Requires the PRNG to be seeded with random_seed,
/// otherwise an error will be returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_random_u32_bounded(
    instance: *mut SeleneInstance,
    bound: u32,
) -> U32Result {
    with_instance_u32(instance, |instance| instance.random_u32_bounded(bound))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_qalloc(instance: *mut SeleneInstance) -> U64Result {
    with_instance_u64(instance, |instance| instance.emulator.user_issued_qalloc())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_qfree(instance: *mut SeleneInstance, q: u64) -> VoidResult {
    with_instance_void(instance, |instance| instance.emulator.user_issued_qfree(q))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_rxy(
    instance: *mut SeleneInstance,
    qubit_id: u64,
    theta: f64,
    phi: f64,
) -> VoidResult {
    with_instance_void(instance, |instance| instance.rxy(qubit_id, theta, phi))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_rz(
    instance: *mut SeleneInstance,
    qubit_id: u64,
    theta: f64,
) -> VoidResult {
    with_instance_void(instance, |instance| instance.rz(qubit_id, theta))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_rzz(
    instance: *mut SeleneInstance,
    qubit_id: u64,
    qubit_id2: u64,
    theta: f64,
) -> VoidResult {
    with_instance_void(instance, |instance| {
        instance.rzz(qubit_id, qubit_id2, theta)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_qubit_reset(instance: *mut SeleneInstance, q: u64) -> VoidResult {
    with_instance_void(instance, |instance| instance.emulator.user_issued_reset(q))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_qubit_measure(instance: *mut SeleneInstance, q: u64) -> BoolResult {
    with_instance_bool(instance, |instance| instance.qubit_measure(q))
}

/// Performs a lazy measurement
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_qubit_lazy_measure(
    instance: *mut SeleneInstance,
    q: u64,
) -> FutureResult {
    with_instance_future_bool(instance, |instance| instance.qubit_lazy_measure(q))
}

/// Performs a lazy measurement with leakage detection
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_qubit_lazy_measure_leaked(
    instance: *mut SeleneInstance,
    q: u64,
) -> FutureResult {
    with_instance_future_bool(instance, |instance| instance.qubit_lazy_measure_leaked(q))
}

/// Decrements a refcount
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_refcount_decrement(
    instance: *mut SeleneInstance,
    r: u64,
) -> VoidResult {
    with_instance_void(instance, |instance| instance.refcount_decrement(r))
}

/// Increments a refcount
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_refcount_increment(
    instance: *mut SeleneInstance,
    r: u64,
) -> VoidResult {
    with_instance_void(instance, |instance| instance.refcount_increment(r))
}

/// Reads a bool future
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_future_read_bool(
    instance: *mut SeleneInstance,
    r: u64,
) -> BoolResult {
    with_instance_bool(instance, |instance| instance.future_read_bool(r))
}

/// Reads a u64 future
#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_future_read_u64(
    instance: *mut SeleneInstance,
    r: u64,
) -> U64Result {
    with_instance_u64(instance, |instance| instance.future_read_u64(r))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_local_barrier(
    instance: *mut SeleneInstance,
    qubit_ids: *const u64,
    qubit_ids_length: u64,
    sleep_time: u64,
) -> VoidResult {
    let qubit_ids = unsafe { std::slice::from_raw_parts(qubit_ids, qubit_ids_length as usize) };
    with_instance_void(instance, |instance| {
        instance.local_barrier(qubit_ids, sleep_time)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_global_barrier(
    instance: *mut SeleneInstance,
    sleep_time: u64,
) -> VoidResult {
    with_instance_void(instance, |instance| instance.global_barrier(sleep_time))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_set_tc(instance: *mut SeleneInstance, tc: u64) -> VoidResult {
    with_instance_void(instance, |instance| {
        instance.time_cursor = tc;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_get_tc(instance: *mut SeleneInstance) -> U64Result {
    with_instance_u64(instance, |instance| Ok(instance.time_cursor))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn selene_custom_runtime_call(
    instance: *mut SeleneInstance,
    tag: u64,
    data: *const u8,
    data_length: u64,
) -> U64Result {
    let data = unsafe { std::slice::from_raw_parts(data, data_length as usize) };
    with_instance_u64(instance, |instance| instance.custom_runtime_call(tag, data))
}
