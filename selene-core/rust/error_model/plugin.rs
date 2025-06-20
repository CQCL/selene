use super::{
    BatchResult, ErrorModelAPIVersion, ErrorModelInterface, ErrorModelInterfaceFactory,
    MeasurementResult,
};
use crate::runtime::BatchOperation;
use crate::utils::{MetricValue, check_errno, read_raw_metric, with_strings_to_cargs};
use anyhow::{Result, anyhow, bail};
use libloading;
use std::ffi::OsStr;
use std::marker::PhantomData;
use std::{ffi, sync::Arc};

pub type ErrorModelInstance = *mut ffi::c_void;
pub type Errno = i32;

/// Controls an error model plugin adhering to the interface:
///
/// - `int selene_error_model_init(
///      void** handle,    // user-settable opaque handle
///      uint64_t n_qubits, // max number of qubits to be available
///      uint32_t argc,     // number of additional arguments
///      char const** argv  // additional arguments
///      char const* simulator_plugin, // path to the simulator plugin
///      uint32_t simulator_argc, // number of additional arguments for the simulator
///      char const** simulator_argv // additional arguments for the simulator
///   )`
///   Called at startup to initialize the plugin. The plugin should return 0 on success, or
///   any other value on failure. It should allocate all state to a struct on the heap and
///   set the handle to point to it. Global state should be avoided where possible, as future
///   versions of selene may support multiple instances of the same plugin to be invoked in
///   parallel.
///
/// - `int selene_error_model_shot_start(
///      void* handle, // user-set state
///      uint64_t shot_id, // shot id
///      uint64_t error_model_seed, // new random seed for generating errors
///      uint64_t simulator_seed // new random seed for the simulator
///   )`
///   Called at the start of a shot to signal that the error model and simulator should prepare
///   to start receiving operations for a new simulation. The random number generators for the
///   error model and the simulator should be reseeded with the provided seeds for reproducibility.
///
/// - `int selene_error_model_shot_end(
///      void* handle, // user-set state
///   )`
///   Called at the end of a shot to signal that the error model and simulator should
///   end the current simulation. This is an ideal point to clean up any state, and perform
///   validation of the simulation if required. The plugin should return 0 on success, or
///   any other value on failure.
///
/// - `int selene_error_model_handle_operations(
///     void* handle, // user-set state
///     RuntimeExtractOperationInstance batch_instance, // batch instance
///     RuntimeExtractOperationInterface const* batch_interface, // batch interface
///     ErrorModelSetResultInstance result_instance, // result instance
///     ErrorModelSetResultInterface const* result_interface // result interface
///    )`
///    Handle a sequence of operations from the runtime. Individual operations may
///    be extracted through the batch_instance, utilising the batch_interface. See
///    [crate::runtime::plugin::RuntimeExtractOperationInterface] for more information
///    about this interface. The error model should apply errors to the operations as
///    it chooses, and can also handle custom data from the runtime if it chooses (
///    and if it is compatible). Results should be provided to the result_instance
///    using the result_interface function interface - see [ErrorModelSetResultInterface]
///    for more information.
///
/// - (optional)
///   `int selene_error_model_get_metrics(
///       *void  // user-set state
///       uint8_t nth_metric, // index of the metric to fetch
///       char* out_tag, // pointer to 256-byte char array to write a tag name (up to 255 chars) into.
///       u8* out_datatype // write 0 => bool, 1 => i64, 2 => u64, 3 => f64
///       u64* out_data // write the data here
///    )`
///    Provide a set of metrics to be provided to the user stream. Return zero if a metric
///    has been written, nonzero if there are no more metrics to write.
///
/// - (optional)
///   `int selene_error_model_get_simulator_metrics(
///       *void  // user-set state
///       uint8_t nth_metric, // index of the metric to fetch
///       char* out_tag, // pointer to 256-byte char array to write a tag name (up to 255 chars) into.
///       u8* out_datatype // write 0 => bool, 1 => i64, 2 => u64, 3 => f64
///       u64* out_data // write the data here
///    )`
///    Provide a set of metrics from the simulator to be provided to the user stream. Return zero if a metric
///    has been written, nonzero if there are no more metrics to write.
///
//// - (optional) `int selene_error_model_exit(void* handle)`
///    If present, is invoked at the end of the simulation to allow the plugin to clean up.
///    If the plugin returns a nonzero value, it is considered to be an error. This allows
///    a plugin to perform post-simulation validation.
///
/// This interface allows implementations of behaviour to be written and distributed independently
/// of selene. Users should be cautious about the plugins they use, as it is possible that mistakes
/// or malicious code could be present in the plugin, and as with all external libraries, due
/// dilligence must be done to verify the source and the trustworthiness of the provider.
#[ouroboros::self_referencing]
pub struct ErrorModelPluginInterface {
    lib: libloading::Library,
    version: ErrorModelAPIVersion,
    #[borrows(lib)]
    #[covariant]
    init_fn: libloading::Symbol<
        'this,
        unsafe extern "C" fn(
            handle: *mut ErrorModelInstance,
            n_qubits: u64,
            error_model_argc: u32,
            error_model_argv: *const *const ffi::c_char,
            simulator_plugin: *const ffi::c_char,
            simulator_argc: u32,
            simulator_argv: *const *const ffi::c_char,
        ) -> Errno,
    >,

    #[borrows(lib)]
    #[covariant]
    exit_fn: Option<
        libloading::Symbol<'this, unsafe extern "C" fn(handle: ErrorModelInstance) -> Errno>,
    >,

    #[borrows(lib)]
    #[covariant]
    shot_start_fn: libloading::Symbol<
        'this,
        unsafe extern "C" fn(
            handle: ErrorModelInstance,
            shot_id: u64,
            error_model_seed: u64,
            simulator_seed: u64,
        ) -> Errno,
    >,

    #[borrows(lib)]
    #[covariant]
    shot_end_fn:
        libloading::Symbol<'this, unsafe extern "C" fn(handle: ErrorModelInstance) -> Errno>,

    #[borrows(lib)]
    #[covariant]
    handle_operations_fn: libloading::Symbol<
        'this,
        unsafe extern "C" fn(
            handle: ErrorModelInstance,
            batch_instance: crate::runtime::plugin::RuntimeExtractOperationInstance,
            batch_interface: *const crate::runtime::plugin::RuntimeExtractOperationInterface,
            result_instance: ErrorModelSetResultInstance,
            result_interface: *const ErrorModelSetResultInterface,
        ) -> Errno,
    >,

    #[borrows(lib)]
    #[covariant]
    dump_simulator_state_fn: Option<
        libloading::Symbol<
            'this,
            unsafe extern "C" fn(
                handle: ErrorModelInstance,
                filename: *const ffi::c_char,
                qubits: *const u64,
                qubits_length: u64,
            ) -> Errno,
        >,
    >,

    #[borrows(lib)]
    #[covariant]
    get_metrics_fn: Option<
        libloading::Symbol<
            'this,
            unsafe extern "C" fn(
                handle: ErrorModelInstance,
                nth_metric: u8,
                out_tag_str: *mut ffi::c_char,
                out_datatype: *mut u8,
                out_data: *mut u64,
            ) -> Errno,
        >,
    >,

    #[borrows(lib)]
    #[covariant]
    get_simulator_metrics_fn: Option<
        libloading::Symbol<
            'this,
            unsafe extern "C" fn(
                handle: ErrorModelInstance,
                nth_metric: u8,
                out_tag_str: *mut ffi::c_char,
                out_datatype: *mut u8,
                out_data: *mut u64,
            ) -> Errno,
        >,
    >,
}
impl ErrorModelPluginInterface {
    pub fn new_from_file(plugin_file: impl AsRef<OsStr>) -> Result<Arc<Self>> {
        let lib = unsafe { libloading::Library::new(plugin_file.as_ref()) }.map_err(|e| {
            anyhow!(
                "Failed to load error model plugin: {}. Error: {}",
                plugin_file.as_ref().to_string_lossy(),
                e
            )
        })?;
        let version: ErrorModelAPIVersion = unsafe {
            if let Ok(func) =
                lib.get::<unsafe extern "C" fn() -> u64>(b"selene_error_model_get_api_version")
            {
                func().into()
            } else {
                return Err(anyhow!(
                    "Failed to load version from error model at '{}'. The plugin is not compatible with this version of selene.",
                    plugin_file.as_ref().to_string_lossy(),
                ));
            }
        };
        version.validate()?;

        let result = ErrorModelPluginInterfaceTryBuilder {
            lib,
            version,
            init_fn_builder: |lib| unsafe { lib.get(b"selene_error_model_init") },
            exit_fn_builder: |lib| unsafe { Ok(lib.get(b"selene_error_model_exit").ok()) },
            shot_start_fn_builder: |lib| unsafe { lib.get(b"selene_error_model_shot_start") },
            shot_end_fn_builder: |lib| unsafe { lib.get(b"selene_error_model_shot_end") },
            handle_operations_fn_builder: |lib| unsafe {
                lib.get(b"selene_error_model_handle_operations")
            },
            dump_simulator_state_fn_builder: |lib| unsafe {
                Ok(lib.get(b"selene_error_model_dump_simulator_state").ok())
            },
            get_metrics_fn_builder: |lib| unsafe {
                Ok(lib.get(b"selene_error_model_get_metrics").ok())
            },
            get_simulator_metrics_fn_builder: |lib| unsafe {
                Ok(lib.get(b"selene_error_model_get_simulator_metrics").ok())
            },
        }
        .try_build()?;
        Ok(Arc::new(result))
    }
}

impl ErrorModelInterfaceFactory for ErrorModelPluginInterface {
    type Interface = ErrorModelPlugin;

    fn init(
        self: Arc<Self>,
        n_qubits: u64,
        error_model_args: &[impl AsRef<str>],
        simulator_plugin: &impl AsRef<OsStr>,
        simulator_args: &[impl AsRef<str>],
    ) -> Result<Box<Self::Interface>> {
        let mut instance = std::ptr::null_mut();
        let safe_simulator_plugin =
            ffi::CString::new(simulator_plugin.as_ref().as_encoded_bytes()).unwrap();
        with_strings_to_cargs(
            error_model_args,
            |error_model_argc, error_model_argv| -> Result<()> {
                with_strings_to_cargs(
                    simulator_args,
                    |simulator_argc, simulator_argv| -> Result<()> {
                        check_errno(
                            unsafe {
                                self.borrow_init_fn()(
                                    &mut instance,
                                    n_qubits,
                                    error_model_argc,
                                    error_model_argv,
                                    safe_simulator_plugin.as_ptr() as *const ffi::c_char,
                                    simulator_argc,
                                    simulator_argv,
                                )
                            },
                            || anyhow!("ErrorModelPluginInterface: init failed"),
                        )
                    },
                )
            },
        )?;
        Ok(Box::new(ErrorModelPlugin {
            interface: self.clone(),
            instance,
        }))
    }
}

pub struct ErrorModelPlugin {
    interface: Arc<ErrorModelPluginInterface>,
    instance: ErrorModelInstance,
}

impl ErrorModelInterface for ErrorModelPlugin {
    fn exit(&mut self) -> Result<()> {
        let Some(exit_fn) = self.interface.borrow_exit_fn() else {
            return Ok(());
        };
        check_errno(unsafe { exit_fn(self.instance) }, || {
            anyhow!("ErrorModelPlugin: exit failed")
        })
    }
    fn shot_start(
        &mut self,
        shot_id: u64,
        error_model_seed: u64,
        simulator_seed: u64,
    ) -> Result<()> {
        check_errno(
            unsafe {
                self.interface.borrow_shot_start_fn()(
                    self.instance,
                    shot_id,
                    error_model_seed,
                    simulator_seed,
                )
            },
            || anyhow!("ErrorModelPlugin: shot_start failed"),
        )
    }
    fn shot_end(&mut self) -> Result<()> {
        check_errno(
            unsafe { self.interface.borrow_shot_end_fn()(self.instance) },
            || anyhow!("ErrorModelPlugin: shot_end failed"),
        )
    }
    fn handle_operations(&mut self, operations: BatchOperation) -> Result<BatchResult> {
        let mut batch_extractor =
            crate::runtime::plugin::BatchExtractor::from_batch_operation(operations);
        let (batch_instance, batch_interface) = batch_extractor.runtime_batch_extraction();
        let mut result_builder = BatchResultBuilder::default();
        let (result_instance, result_interface) = result_builder.error_model_set_result();
        check_errno(
            unsafe {
                self.interface.borrow_handle_operations_fn()(
                    self.instance,
                    batch_instance,
                    &raw const batch_interface,
                    result_instance,
                    &raw const result_interface,
                )
            },
            || anyhow!("ErrorModelPlugin: handle_operations failed"),
        )?;
        Ok(result_builder.finish())
    }
    fn dump_simulator_state(&mut self, file: &std::path::Path, qubits: &[u64]) -> Result<()> {
        let Some(dump_fn) = self.interface.borrow_dump_simulator_state_fn() else {
            bail!("Dumping simulator state is unsupported for this error model.");
        };
        let filename = file.to_str().ok_or_else(|| {
            anyhow!("ErrorModelPlugin: dump_simulator_state failed, invalid filename")
        })?;
        let safe_filename = ffi::CString::new(filename).unwrap();
        check_errno(
            unsafe {
                dump_fn(
                    self.instance,
                    safe_filename.as_ptr(),
                    qubits.as_ptr(),
                    qubits.len() as u64,
                )
            },
            || anyhow!("ErrorModelPlugin: dump_simulator_state failed"),
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
    fn get_simulator_metric(&mut self, nth_metric: u8) -> Result<Option<(String, MetricValue)>> {
        let Some(get_simulator_metrics_fn) = self.interface.borrow_get_simulator_metrics_fn()
        else {
            return Ok(None);
        };
        read_raw_metric(|tag, data_type, data| unsafe {
            get_simulator_metrics_fn(self.instance, nth_metric, tag, data_type, data)
        })
    }
}

#[derive(Default)]
/// A helper type used by the plugin tooling above to implement
/// [ErrorModelSetResultInterface].
struct BatchResultBuilder(BatchResult);

impl BatchResultBuilder {
    unsafe extern "C" fn set_measurement_result(
        interface: ErrorModelSetResultInstance,
        result_id: u64,
        measurement: bool,
    ) {
        let result = interface as *mut BatchResult;
        (unsafe { &mut *result })
            .measurements
            .push(MeasurementResult {
                result_id,
                measurement,
            })
    }

    /// The plugin calls this to obtain an instance and an interface.
    /// The lifetime parameter of the interface ensures that it cannot outlive the `Vec`
    /// that the functions will mutate.
    fn error_model_set_result(
        &mut self,
    ) -> (
        ErrorModelSetResultInstance,
        ErrorModelSetResultInterface<'_>,
    ) {
        let instance = &raw mut self.0 as ErrorModelSetResultInstance;
        let interface = ErrorModelSetResultInterface {
            set_measurement_result_fn: Self::set_measurement_result,
            _marker: PhantomData,
        };
        (instance, interface)
    }

    /// Consumes the `BatchBuilder` returning the accumulated operations.
    fn finish(self) -> BatchResult {
        self.0
    }
}

/// An instance is provided to `selene_runtime_get_next_operations`, which must
/// pass that back to any function it calls in it's provided
/// [ErrorModelSetResultInterface].
pub type ErrorModelSetResultInstance = *mut ffi::c_void;

#[repr(C)]
#[non_exhaustive]
/// A plugin's implementation of `selene_runtime_get_next_operations` is provided
/// a pointer to a `ErrorModelSetResultInterface` as well as a
/// [ErrorModelSetResultInstance]. It should call the functions
/// within to populate a batch. All such calls must pass the instance as the
/// first parameter.
pub struct ErrorModelSetResultInterface<'a> {
    pub set_measurement_result_fn: unsafe extern "C" fn(ErrorModelSetResultInstance, u64, bool),
    _marker: PhantomData<&'a ()>,
}
