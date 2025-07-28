//! Defines the [crate::export_error_model_plugin!] and helpers for implementing
//! error model plugins as rust crates.
//!
//! See `selene-ideal-error-model-plugin` for a fully worked example.
use super::{
    ErrorModelInterface,
    interface::ErrorModelInterfaceFactory,
    plugin::{
        Errno, ErrorModelInstance, ErrorModelSetResultInstance, ErrorModelSetResultInterface,
    },
};
use crate::runtime::plugin::{
    BatchBuilder, RuntimeExtractOperationInstance, RuntimeExtractOperationInterface,
};
use crate::utils::{convert_cargs_to_strings, result_of_errno_to_errno, result_to_errno};
use std::{ffi, mem, sync::Arc};

#[derive(Default)]
/// A helper struct used by [crate::export_error_model_plugin!] to implement the error model
/// plugin entry points defined by [super::plugin::ErrorModelPluginInterface] using an implementation
/// of [ErrorModelInterfaceFactory].
pub struct Helper<F>(Arc<F>);

impl<F: ErrorModelInterfaceFactory> Helper<F> {
    fn into_error_model_instance(e: Box<F::Interface>) -> ErrorModelInstance {
        Box::into_raw(e) as ErrorModelInstance
    }

    unsafe fn from_error_model_instance(instance: ErrorModelInstance) -> Box<F::Interface> {
        assert!(!instance.is_null());
        unsafe { Box::from_raw(instance as *mut F::Interface) }
    }

    fn with_error_model_instance<T>(
        instance: ErrorModelInstance,
        mut go: impl FnMut(&mut F::Interface) -> T,
    ) -> T {
        let mut e = unsafe { Self::from_error_model_instance(instance) };
        let t = go(&mut e);
        mem::forget(e);
        t
    }

    fn factory(&self) -> Arc<F> {
        self.0.clone()
    }

    pub unsafe fn init(
        &self,
        instance: *mut ErrorModelInstance,
        n_qubits: u64,
        error_model_argc: u32,
        error_model_argv: *const *const ffi::c_char,
        simulator_plugin: *const ffi::c_char,
        simulator_argc: u32,
        simulator_argv: *const *const ffi::c_char,
    ) -> Errno {
        if instance.is_null() {
            eprintln!("cannot initialize error model plugin: provided instance is null");
            return -1;
        }

        let error_model_args: Vec<String> = {
            let mut v = vec!["lib".to_string()];
            v.extend(unsafe { convert_cargs_to_strings(error_model_argc, error_model_argv) });
            v
        };
        let simulator_args: Vec<String> =
            unsafe { convert_cargs_to_strings(simulator_argc, simulator_argv) }.collect();

        result_to_errno(
            "Failed to initialize the error model plugin",
            self.factory()
                .init(
                    n_qubits,
                    error_model_args.as_ref(),
                    &unsafe { std::ffi::CStr::from_ptr(simulator_plugin) }
                        .to_str()
                        .unwrap(),
                    simulator_args.as_ref(),
                )
                .map(|runtime| unsafe {
                    *instance = Self::into_error_model_instance(runtime);
                }),
        )
    }

    pub unsafe fn exit(instance: ErrorModelInstance) -> Errno {
        result_to_errno(
            "Failed to exit the error model plugin",
            Self::with_error_model_instance(instance, |e| e.exit()),
        )
    }

    pub unsafe fn shot_start(
        instance: ErrorModelInstance,
        shot_id: u64,
        error_model_seed: u64,
        simulator_seed: u64,
    ) -> Errno {
        result_to_errno(
            format!("Failed to start shot {shot_id}"),
            Self::with_error_model_instance(instance, |e| {
                e.shot_start(shot_id, error_model_seed, simulator_seed)
            }),
        )
    }
    pub unsafe fn shot_end(instance: ErrorModelInstance) -> Errno {
        result_to_errno(
            "Failed to end the current shot",
            Self::with_error_model_instance(instance, |e| e.shot_end()),
        )
    }

    pub unsafe fn dump_simulator_state(
        instance: ErrorModelInstance,
        filename: *const ffi::c_char,
        qubits: *const u64,
        qubits_length: u64,
    ) -> Errno {
        result_to_errno(
            "Failed to dump the simulator state",
            Self::with_error_model_instance(instance, |e| {
                let qubits = unsafe { std::slice::from_raw_parts(qubits, qubits_length as usize) };
                e.dump_simulator_state(
                    &std::path::PathBuf::from(
                        unsafe { std::ffi::CStr::from_ptr(filename) }
                            .to_str()
                            .unwrap(),
                    ),
                    qubits,
                )
            }),
        )
    }

    pub unsafe fn handle_operations(
        instance: ErrorModelInstance,
        extract_ops_instance: RuntimeExtractOperationInstance,
        extract_ops_interface: *const RuntimeExtractOperationInterface,
        result_instance: ErrorModelSetResultInstance,
        result_interface: *const ErrorModelSetResultInterface,
    ) -> Errno {
        result_to_errno(
            "Failed to handle operations",
            Self::with_error_model_instance(instance, |error_model| unsafe {
                let mut batch_builder: BatchBuilder = BatchBuilder::default();
                let (builder_instance, builder_interface) = batch_builder.runtime_get_operation();
                let RuntimeExtractOperationInterface { extract_fn, .. } = &*extract_ops_interface;
                extract_fn(extract_ops_instance, builder_instance, builder_interface);
                let results = error_model.handle_operations(batch_builder.finish())?;
                let ErrorModelSetResultInterface {
                    set_bool_result_fn,
                    set_u64_result_fn,
                    ..
                } = &*result_interface;
                for bool_result in results.bool_results {
                    set_bool_result_fn(result_instance, bool_result.result_id, bool_result.value);
                }
                for u64_result in results.u64_results {
                    set_u64_result_fn(result_instance, u64_result.result_id, u64_result.value);
                }
                anyhow::Ok(())
            }),
        )
    }

    pub unsafe fn get_metric(
        instance: ErrorModelInstance,
        nth_metric: u8,
        tag_ptr: *mut ffi::c_char,
        datatype_ptr: *mut u8,
        data_ptr: *mut u64,
    ) -> Errno {
        result_of_errno_to_errno(
            "Failed to get metric",
            Self::with_error_model_instance(instance, |runtime| {
                let Some((tag, metric)) = runtime.get_metric(nth_metric)? else {
                    return anyhow::Ok(1);
                };
                unsafe { metric.write_raw(tag, tag_ptr, datatype_ptr, data_ptr) }
                Ok(0)
            }),
        )
    }

    pub unsafe fn get_simulator_metric(
        instance: ErrorModelInstance,
        nth_metric: u8,
        tag_ptr: *mut ffi::c_char,
        datatype_ptr: *mut u8,
        data_ptr: *mut u64,
    ) -> Errno {
        result_of_errno_to_errno(
            "Failed to get metric",
            Self::with_error_model_instance(instance, |runtime| {
                let Some((tag, metric)) = runtime.get_simulator_metric(nth_metric)? else {
                    return anyhow::Ok(1);
                };
                unsafe { metric.write_raw(tag, tag_ptr, datatype_ptr, data_ptr) }
                Ok(0)
            }),
        )
    }
}

#[macro_export]
/// A macro to export an error model plugin from a crate
///
/// Expands to a module named _plugin with `pub extern "C" unsafe` definitions
/// of all entry points specified in the documentation of
/// [super::plugin::ErrorModelPluginInterface].
///
/// Those functions are all implemented using the `$factory_type`'s implementation
/// of [ErrorModelInterfaceFactory].
///
/// A crate can only export a single runtime plugin (although it may export other
/// plugins of other types). See the `selene-ideal-error-model-plugin` python package
/// for a fully worked example.
macro_rules! export_error_model_plugin {
    ($factory_type:ty) => {
        mod _plugin {
            use selene_core::{
                error_model::{
                    ErrorModelInterfaceFactory,
                    plugin::{
                        Errno, ErrorModelInstance, ErrorModelSetResultInstance,
                        ErrorModelSetResultInterface,
                    },
                    version::CURRENT_API_VERSION,
                },
                runtime::plugin::{
                    BatchBuilder, RuntimeExtractOperationInstance, RuntimeExtractOperationInterface,
                },
            };

            use std::ffi::c_char;

            /// cbindgen:ignore
            type Helper = selene_core::error_model::helper::Helper<$factory_type>;

            // Enforce that $struct_name implements the ErrorModelInterface trait
            const _: fn() = || {
                fn _assert_impl<T: ErrorModelInterfaceFactory>() {}
                _assert_impl::<$factory_type>();
            };

            /// The API version comprises four unsigned 8-bit integers:
            ///     - reserved: 8 bits (must be 0)
            ///     - major: 8 bits
            ///     - minor: 8 bits
            ///     - patch: 8 bits
            ///
            /// Selene maintains its own API version for the error model
            /// and is updated upon changes to the API depending on how
            /// breaking the changes are. Selene is also responsible for
            /// validating the API version of the plugin against its own
            /// version.
            ///
            /// The plans for this validation are a work-in-progress, but
            /// currently selene will reject any plugin that has a different
            /// major or minor version than the current Selene version, or with
            /// a reserved field that is not 0.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_error_model_get_api_version() -> u64 {
                // The API version is defined in the version module
                // and is used to check compatibility between the plugin and the runtime.
                CURRENT_API_VERSION.into()
            }

            /// When Selene is initialised, it is provided with some default arguments
            /// (the maximum number of qubits, the path to a simulator plugin to use, etc)
            /// and some custom arguments for the error model and simulator. These arguments
            /// are provided to the error model through this initialization function, with
            /// the custom arguments passed in in an argc, argv format.
            ///
            /// It is this function's responsibility to parse and validate those user-provided
            /// arguments and initialise a plugin instance ready for a call to
            /// selene_error_model_shot_start(). The `instance` pointer is designed to be set
            /// by this function and hold all relevant state, such that subsequent calls to the
            /// corresponding instance will be able to access that state, and such that calls to
            /// other instances will not be impacted.
            ///
            /// Error model plugins should provide customisation of parameter values
            /// within their python implementations. They should also define how those
            /// parameters are converted to an argv list to be passed to their compiled
            /// counterparts.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_error_model_init(
                instance: *mut ErrorModelInstance,
                n_qubits: u64,
                error_model_argc: u32,
                error_model_argv: *const *const c_char,
                simulator_plugin: *const c_char,
                simulator_argc: u32,
                simulator_argv: *const *const c_char,
            ) -> Errno {
                use std::cell::OnceCell;
                use std::sync::Mutex;
                static FACTORY: Mutex<OnceCell<Helper>> = Mutex::new(OnceCell::new());
                FACTORY
                    .lock()
                    .unwrap()
                    .get_or_init(|| Helper::default())
                    .init(
                        instance,
                        n_qubits,
                        error_model_argc,
                        error_model_argv,
                        simulator_plugin,
                        simulator_argc,
                        simulator_argv,
                    )
            }

            /// This function is called when Selene is exiting, and it is responsible for
            /// cleaning up any resources that the error model plugin has allocated.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_error_model_exit(
                instance: ErrorModelInstance,
            ) -> Errno {
                Helper::exit(instance)
            }

            /// This function is called at the start of a shot, and it is responsible for
            /// initialising the error model plugin for that shot. The error_model_seed is
            /// provided for RNG seeding, and it is highly recommended that all randomness
            /// used by the error model plugin is seeded with this value.
            ///
            /// As the error model currently owns the simulator, the simulator_seed is also
            /// provided to allow the error model to seed the simulator's RNG. This should
            /// result in a call to the simulator's shot_start function.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_error_model_shot_start(
                instance: ErrorModelInstance,
                shot_id: u64,
                error_model_seed: u64,
                simulator_seed: u64,
            ) -> Errno {
                Helper::shot_start(instance, shot_id, error_model_seed, simulator_seed)
            }

            /// This function is called at the end of a shot, and it is responsible for
            /// finalising the error model plugin for that shot. For example, it may
            /// clean up any intra-shot state, such as accumulators or buffers. A call to
            /// this function will usually be followed either by a call to
            /// `selene_error_model_shot_start` to prepare for the following shot, or by
            /// a call to `selene_error_model_exit` to shut down the instance.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_error_model_shot_end(
                instance: ErrorModelInstance,
            ) -> Errno {
                Helper::shot_end(instance)
            }

            /// This function is called to handle a batch of operations extracted from the
            /// runtime. It is responsible for processing the operations and returning the
            /// results of any measurements provided in the operation list.
            ///
            /// As a batch of operations takes the form of a container, we provide extraction
            /// through a RuntimeExtractOperationInterface (essentially a list of function
            /// pointers) and a corresponding RuntimeExtractOperationInstance that provides
            /// the underlying state. See the documentation for [RuntimeExtractOperationInterface]
            /// and [RuntimeExtractOperationInstance] for more details.
            ///
            /// Likewise, as the results of the operations are also in the form of a container,
            /// we provide an ErrorModelSetResultInterface that allows the error model to set
            /// the results of the measurements in the runtime.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_error_model_handle_operations(
                instance: ErrorModelInstance,
                extract_ops_instance: RuntimeExtractOperationInstance,
                extract_ops_interface: *const RuntimeExtractOperationInterface,
                result_instance: ErrorModelSetResultInstance,
                result_interface: *const ErrorModelSetResultInterface,
            ) -> Errno {
                Helper::handle_operations(
                    instance,
                    extract_ops_instance,
                    extract_ops_interface,
                    result_instance,
                    result_interface,
                )
            }

            /// This function is called to dump the current state of the simulator to a file.
            /// This is niche functionality and any error model that 'wraps' the simulator state
            /// in a non-trivial manner (such that the underlying simulator state is not reflective
            /// of the state itself, e.g. in the case of leakage) should return an error from this
            /// function, as the simulator state is not meaningful in that case.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_error_model_dump_simulator_state(
                instance: ErrorModelInstance,
                filename: *const c_char,
                qubits: *const u64,
                qubits_length: u64,
            ) -> Errno {
                Helper::dump_simulator_state(instance, filename, qubits, qubits_length)
            }

            /// This is a passthrough function to the simulator's get_metric function. The
            /// error model should invoke the simulator's metric function directly unless it
            /// has reason to modify the output in some way.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_error_model_get_simulator_metrics(
                instance: ErrorModelInstance,
                nth_metric: u8,
                tag_ptr: *mut c_char,
                datatype_ptr: *mut u8,
                data_ptr: *mut u64,
            ) -> Errno {
                Helper::get_simulator_metric(instance, nth_metric, tag_ptr, datatype_ptr, data_ptr)
            }

            /// This function is called to get a metric from the error model. When the time comes
            /// to gather metrics, Selene will call this function with `nth_metric` set to 0, then
            /// 1, then 2, and so on until it returns a nonzero value indicating the end of the
            /// available metrics, or until the nth_metric is 255.
            ///
            /// Three parameters are provided for writing metric information. The first is a
            /// pointer to a 255-character buffer used for writing the tag of the metric.
            /// The second is a pointer to a u8 that will be set to the data type of the metric,
            /// with:
            /// - 0 for boolean
            /// - 1 for i64
            /// - 2 for u64
            /// - 3 for f64
            /// The third is a pointer to a u64 that will be set to the value of the metric, and
            /// should be interpreted as the datatype indicated by the second parameter.
            ///
            /// For example:
            /// ```c
            /// void selene_error_model_get_simulator_metrics(
            ///     SomeInstance* instance,
            ///     uint8_t nth_metric,
            ///     char* tag,
            ///     u8* datatype,
            ///     u64* data,
            /// ) {
            ///     if (nth_metric == 0) {
            ///        strcpy(tag, "number_of_bitflips");
            ///        *datatype = 2; // u64
            ///        *data = instance->number_of_bitflips();
            ///        return 0; // metric written, call again for next metric.
            ///     } else if (nth_metric == 1) {
            ///        strcpy(tag, "has_leaked_qubits");
            ///        *datatype = 0; // boolean
            ///        *data = instance->has_leaked_qubits();
            ///        return 0; // metric written, call again for next metric.
            ///     } else {
            ///        uint64_t nth_qubit = nth_metric - 2;
            ///        if (nth_qubit <= instance->n_qubits()) {
            ///           sprintf(tag, "added_phase_qubit_%llu", nth_qubit);
            ///           *datatype = 3; // f64
            ///           *data = instance->get_added_phase(nth_qubit);
            ///           return 0; // metric written, call again for next metric.
            ///        }
            ///     }
            ///     return 1; // no metric written, end of metrics.
            /// }
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_error_model_get_metrics(
                instance: ErrorModelInstance,
                nth_metric: u8,
                tag_ptr: *mut c_char,
                datatype_ptr: *mut u8,
                data_ptr: *mut u64,
            ) -> Errno {
                Helper::get_metric(instance, nth_metric, tag_ptr, datatype_ptr, data_ptr)
            }
        }
    };
}
