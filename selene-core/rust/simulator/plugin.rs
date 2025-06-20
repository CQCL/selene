use super::{SimulatorAPIVersion, SimulatorInterface, SimulatorInterfaceFactory};
use crate::utils::{MetricValue, check_errno, read_raw_metric, with_strings_to_cargs};
use anyhow::{Result, anyhow};
use libloading;
use ouroboros::self_referencing;
use std::ffi::OsStr;
use std::ffi::c_char;
use std::sync::Arc;

pub type SimulatorInstance = *mut std::ffi::c_void;

pub type Errno = i32;

/// Provides a simulation engine backend that controls a plugin, in the form of a shared object.
/// The plugin must export the following functions:
/// - `uint64_t selene_simulator_get_api_version()`
///    Returns the version of the Selene Simulator API that the plugin is built against.
///    Represents 4 8-bit numbers: [reserved], major, minor, patch, from most to least
///    significant byte. [reserved] must be zero. See the 'versioning' section of the plugin
///    development documentation for further information.
///
/// - `int selene_simulator_init(
///      *void handle_out,  // user settable state
///      uint64_t n_qubits, // number of qubits to be available
///      uint32_t argc,     // number of additional arguments
///      const char **argv  // additional arguments
///   )`
///   Called at startup to initialize the plugin. The plugin should return 0 on success, and
///   non-zero on failure.
///
/// - `int selene_simulator_shot_start(void* handle, uint64_t shot_id, uint64_t seed)`
///   Called at the start of a shot. The plugin should return 0 on success, and non-zero on
///   failure.
///
/// - `int selene_simulator_shot_end(void* handle)`
///   Called at the end of a shot. The plugin should return 0 on success, and non-zero on
///   failure.
///
/// - `int selene_simulator_operation_rxy(
///      *void,        // user-set state
///      uint64_t q0,  // qubit to apply the gate to
///      double theta, // angle
///      double phi    // angle
///    )`
///    Apply an RXY gate to qubit `q0` with the given angles. Return nonzero on failure.
///
/// - `int selene_simulator_operation_rzz(
///      *void,        // user-set state
///      uint64_t q0,  // first qubit
///      uint64_t q1,  // second qubit
///      double theta  // rotation angle
///    )`
///    Apply an RZZ gate between qubits `q0` and `q1` with the given angle. Return nonzero on
///    failure.
///
/// - `int selene_simulator_operation_rz(
///      *void,        // user-set state
///      uint64_t q0,  // qubit to apply the gate to
///      double theta, // angle
///    )`
///    Apply an RZ gate to qubit `q0` with the given angle. Return nonzero on failure.
///
///
/// - `int selene_simulator_operation_measure(
///       *void,      // user-set state
///       uint64_t q0 // qubit to measure
///    )`
///    Measure qubit `q0` and return 0 for false, 1 for true, and any other value for failure.
///
/// - `int selene_simulator_operation_postselect(
///       *void,     // user-set state
///       uint64_t q0, // qubit to postselect
///       bool target_value // target value to postselect
///    )`
///    Postselect qubit `q0` to the given value. If the postselection is viable and performed,
///    return 0. If it is not supported in the simulator, or if postselection isn't possible
///    from the state immediately prior to this operation, return nonzero.
///
/// - `int selene_simulator_operation_reset(
///       *void,     // user-set state
///       uint64_t q0 // qubit to measure
///    )`
///    Reset qubit `q0` to the |0> state. Return nonzero on failure.
///
/// - (optional) `int selene_simulator_get_metrics(
///       *void  // user-set state
///       uint8_t nth_metric, // index of metric to fetch (called with 0 to 255 until a non-zero
///       return)
///       char* tag, // pointer to 256-byte char array to write a tag name (up to 255 chars) into.
///       u8* datatype // write the datatype here: 0 => bool, 1 => i64, 2 => u64, 3 => f64
///       u64* data // write the data here
///    )`
///    Provide a set of metrics to be written to the output stream. Return zero if a metric
///    has been written, nonzero if there are no more metrics to write.
///
/// - (optional) `int selene_simulator_exit(*void /* user set state */)`
///    If present, is invoked at the end of the simulation to allow the plugin to clean up.
///    If the plugin returns a nonzero value, it is considered to be an error. This allows
///    a plugin to perform post-simulation validation. An example use case is the classical
///    replay plugin, that replays a sequence of recorded measurements: if at the end of a run,
///    there are still measurements to replay, the plugin can return an error code to indicate
///    that it is not a faithful replay.
///
/// Plugins are used allow implementations of simulation backends to be written and
/// distributed independently of selene. Users should be cautious about the plugins they use,
/// as it is possible that mistakes or malicious code could be present in the plugin, and, as
/// with all external libraries, due dilligence must be done to verify the source and the
/// trustworthiness of the provider.
#[self_referencing]
pub struct SimulatorPluginInterface {
    lib: libloading::Library,
    name: String,
    version: SimulatorAPIVersion,
    #[borrows(lib)]
    #[covariant]
    init_fn: libloading::Symbol<
        'this,
        unsafe extern "C" fn(
            handle: *mut SimulatorInstance,
            n_qubits: u64,
            argc: u32,
            argv: *const *const c_char,
        ) -> Errno,
    >,
    #[borrows(lib)]
    #[covariant]
    exit_fn:
        Option<libloading::Symbol<'this, unsafe extern "C" fn(handle: SimulatorInstance) -> Errno>>,
    #[borrows(lib)]
    #[covariant]
    shot_start_fn: libloading::Symbol<
        'this,
        unsafe extern "C" fn(handle: SimulatorInstance, shot_id: u64, seed: u64) -> Errno,
    >,
    #[borrows(lib)]
    #[covariant]
    shot_end_fn:
        libloading::Symbol<'this, unsafe extern "C" fn(handle: SimulatorInstance) -> Errno>,
    #[borrows(lib)]
    #[covariant]
    rxy_fn: libloading::Symbol<
        'this,
        unsafe extern "C" fn(handle: SimulatorInstance, qubit: u64, theta: f64, phi: f64) -> Errno,
    >,
    #[borrows(lib)]
    #[covariant]
    rz_fn: libloading::Symbol<
        'this,
        unsafe extern "C" fn(handle: SimulatorInstance, qubit0: u64, theta: f64) -> Errno,
    >,
    #[borrows(lib)]
    #[covariant]
    rzz_fn: libloading::Symbol<
        'this,
        unsafe extern "C" fn(
            handle: SimulatorInstance,
            qubit0: u64,
            qubit1: u64,
            theta: f64,
        ) -> Errno,
    >,
    #[borrows(lib)]
    #[covariant]
    measure_fn: libloading::Symbol<
        'this,
        unsafe extern "C" fn(handle: SimulatorInstance, qubit: u64) -> Errno,
    >,
    #[borrows(lib)]
    #[covariant]
    postselect_fn: Option<
        libloading::Symbol<
            'this,
            unsafe extern "C" fn(
                handle: SimulatorInstance,
                qubit: u64,
                target_value: bool,
            ) -> Errno,
        >,
    >,
    #[borrows(lib)]
    #[covariant]
    reset_fn: libloading::Symbol<
        'this,
        unsafe extern "C" fn(handle: SimulatorInstance, qubit: u64) -> Errno,
    >,
    #[borrows(lib)]
    #[covariant]
    get_metrics_fn: Option<
        libloading::Symbol<
            'this,
            unsafe extern "C" fn(
                handle: SimulatorInstance,
                nth_metric: u8,
                tag_out: *mut c_char,
                datatype_out: *mut u8,
                value_out: *mut u64,
            ) -> Errno,
        >,
    >,
    #[borrows(lib)]
    #[covariant]
    dump_state_fn: libloading::Symbol<
        'this,
        unsafe extern "C" fn(
            handle: SimulatorInstance,
            file: *const c_char,
            qubits: *const u64,
            n_qubits: u64,
        ) -> Errno,
    >,
}
impl SimulatorPluginInterface {
    pub fn new_from_file(plugin_file: impl AsRef<OsStr>) -> Result<Arc<Self>> {
        let lib = unsafe { libloading::Library::new(plugin_file.as_ref()) }.map_err(|e| {
            anyhow!(
                "Failed to load simulator plugin: {}. Error: {}",
                plugin_file.as_ref().to_string_lossy(),
                e
            )
        })?;
        let version: SimulatorAPIVersion = unsafe {
            if let Ok(func) =
                lib.get::<unsafe extern "C" fn() -> u64>(b"selene_simulator_get_api_version")
            {
                func().into()
            } else {
                return Err(anyhow!(
                    "Failed to load version from simulator at '{}'. The plugin is not compatible with this version of selene.",
                    plugin_file.as_ref().to_string_lossy(),
                ));
            }
        };
        version.validate()?;
        let name = unsafe {
            if let Ok(func) =
                lib.get::<unsafe extern "C" fn() -> *const c_char>(b"selene_simulator_get_name")
            {
                func().as_ref().map_or_else(
                    || "Unknown".to_string(),
                    |name| {
                        std::ffi::CStr::from_ptr(name)
                            .to_string_lossy()
                            .into_owned()
                    },
                )
            } else {
                "Unknown".to_string()
            }
        };
        let result = SimulatorPluginInterfaceTryBuilder {
            lib,
            version,
            name,
            init_fn_builder: |lib| unsafe { lib.get(b"selene_simulator_init") },
            exit_fn_builder: |lib| unsafe { Ok(lib.get(b"selene_simulator_exit").ok()) },
            shot_start_fn_builder: |lib| unsafe { lib.get(b"selene_simulator_shot_start") },
            shot_end_fn_builder: |lib| unsafe { lib.get(b"selene_simulator_shot_end") },
            rxy_fn_builder: |lib| unsafe { lib.get(b"selene_simulator_operation_rxy") },
            rz_fn_builder: |lib| unsafe { lib.get(b"selene_simulator_operation_rz") },
            rzz_fn_builder: |lib| unsafe { lib.get(b"selene_simulator_operation_rzz") },
            measure_fn_builder: |lib| unsafe { lib.get(b"selene_simulator_operation_measure") },
            postselect_fn_builder: |lib| unsafe {
                Ok(lib.get(b"selene_simulator_operation_postselect").ok())
            },
            reset_fn_builder: |lib| unsafe { lib.get(b"selene_simulator_operation_reset") },
            get_metrics_fn_builder: |lib| unsafe {
                Ok(lib.get(b"selene_simulator_get_metrics").ok())
            },
            dump_state_fn_builder: |lib| unsafe { lib.get(b"selene_simulator_dump_state") },
        }
        .try_build()?;
        Ok(Arc::new(result))
    }
}

pub struct SimulatorPlugin {
    interface: Arc<SimulatorPluginInterface>,
    instance: SimulatorInstance,
}

impl SimulatorInterfaceFactory for SimulatorPluginInterface {
    type Interface = SimulatorPlugin;

    fn init(
        self: Arc<Self>,
        n_qubits: u64,
        args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        let mut instance = std::ptr::null_mut();
        with_strings_to_cargs(args, |argc, argv| {
            check_errno(
                unsafe { self.borrow_init_fn()(&mut instance, n_qubits, argc, argv) },
                || anyhow!("SimulatorPlugin: init failed"),
            )
        })?;
        Ok(Box::new(SimulatorPlugin {
            interface: self.clone(),
            instance,
        }))
    }
}

impl SimulatorInterface for SimulatorPlugin {
    fn exit(&mut self) -> Result<()> {
        let Some(exit_fn) = self.interface.borrow_exit_fn() else {
            return Ok(());
        };
        check_errno(unsafe { exit_fn(self.instance) }, || {
            anyhow!("SimulatorPlugin: exit failed")
        })
    }
    fn shot_start(&mut self, shot_id: u64, seed: u64) -> Result<()> {
        check_errno(
            unsafe { self.interface.borrow_shot_start_fn()(self.instance, shot_id, seed) },
            || {
                anyhow!(
                    "SimulatorPlugin({}): shot_start failed",
                    self.interface.borrow_name()
                )
            },
        )
    }
    fn shot_end(&mut self) -> Result<()> {
        check_errno(
            unsafe { self.interface.borrow_shot_end_fn()(self.instance) },
            || {
                anyhow!(
                    "SimulatorPlugin({}): shot_end failed",
                    self.interface.borrow_name()
                )
            },
        )
    }
    fn rxy(&mut self, qubit: u64, theta: f64, phi: f64) -> Result<()> {
        check_errno(
            unsafe { self.interface.borrow_rxy_fn()(self.instance, qubit, theta, phi) },
            || {
                anyhow!(
                    "SimulatorPlugin({}): rxy failed",
                    self.interface.borrow_name()
                )
            },
        )
    }
    fn rz(&mut self, qubit: u64, theta: f64) -> Result<()> {
        check_errno(
            unsafe { self.interface.borrow_rz_fn()(self.instance, qubit, theta) },
            || {
                anyhow!(
                    "SimulatorPlugin({}): rz failed",
                    self.interface.borrow_name()
                )
            },
        )
    }
    fn rzz(&mut self, qubit1: u64, qubit2: u64, theta: f64) -> Result<()> {
        check_errno(
            unsafe { (self.interface.borrow_rzz_fn())(self.instance, qubit1, qubit2, theta) },
            || {
                anyhow!(
                    "SimulatorPlugin({}): rzz failed",
                    self.interface.borrow_name()
                )
            },
        )
    }
    fn measure(&mut self, qubit: u64) -> Result<bool> {
        let result = unsafe { (self.interface.borrow_measure_fn())(self.instance, qubit) };
        match result {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(anyhow!(
                "SimulatorPlugin({}): measure failed",
                self.interface.borrow_name()
            )),
        }
    }
    fn postselect(&mut self, qubit: u64, target_value: bool) -> Result<()> {
        let Some(postselect_fn) = self.interface.borrow_postselect_fn() else {
            return Err(anyhow!(
                "The chosen simulator does not support postselection"
            ));
        };
        check_errno(
            unsafe { postselect_fn(self.instance, qubit, target_value) },
            || {
                anyhow!(
                    "SimulatorPlugin({}): postselect failed",
                    self.interface.borrow_name()
                )
            },
        )
    }
    fn reset(&mut self, qubit: u64) -> Result<()> {
        check_errno(
            unsafe { (self.interface.borrow_reset_fn())(self.instance, qubit) },
            || {
                anyhow!(
                    "SimulatorPlugin({}): reset failed",
                    self.interface.borrow_name()
                )
            },
        )
    }
    fn get_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        let Some(get_metrics_fn) = self.interface.borrow_get_metrics_fn() else {
            return Ok(None);
        };
        read_raw_metric(|tag, data_type, data| unsafe {
            get_metrics_fn(self.instance, nth_metric, tag, data_type, data)
        })
    }
    fn dump_state(&mut self, file: &std::path::Path, qubits: &[u64]) -> Result<()> {
        let qubit_count = qubits.len() as u64;
        let filename = file.to_str().ok_or_else(|| {
            anyhow!(
                "SimulatorPlugin({}): dump_state failed: invalid filename: {}",
                self.interface.borrow_name(),
                file.to_string_lossy()
            )
        })?;
        let safe_filename = std::ffi::CString::new(filename).unwrap();
        check_errno(
            unsafe {
                (self.interface.borrow_dump_state_fn())(
                    self.instance,
                    safe_filename.as_ptr(),
                    qubits.as_ptr(),
                    qubit_count,
                )
            },
            || {
                anyhow!(
                    "SimulatorPlugin({}): dump_state failed",
                    self.interface.borrow_name()
                )
            },
        )
    }
}
