//! Defines the [crate::export_runtime_plugin!] and helpers for implementing runtime
//! plugins as rust crates.
//!
//! See `selene-simple-runtime-plugin` for a fully worked example.
use std::{ffi, mem, sync::Arc};

use crate::utils::{convert_cargs_to_strings, result_of_errno_to_errno, result_to_errno};

use super::{
    Operation, RuntimeInterface,
    interface::RuntimeInterfaceFactory,
    plugin::{Errno, RuntimeGetOperationInstance, RuntimeGetOperationInterface, RuntimeInstance},
};

#[derive(Default)]
/// A helper struct used by [crate::export_runtime_plugin!] to implement the runtime
/// plugin entry points defined by [super::plugin::RuntimePluginInterface] using an
/// implentation of [RuntimeInterfaceFactory]`.
pub struct Helper<F>(Arc<F>);

impl<F: RuntimeInterfaceFactory> Helper<F> {
    fn into_runtime_instance(r: Box<F::Interface>) -> RuntimeInstance {
        Box::into_raw(r) as RuntimeInstance
    }

    unsafe fn from_runtime_instance(instance: RuntimeInstance) -> Box<F::Interface> {
        unsafe { Box::from_raw(instance as *mut F::Interface) }
    }

    fn with_runtime_instance<T>(
        instance: RuntimeInstance,
        mut go: impl FnMut(&mut F::Interface) -> T,
    ) -> T {
        assert!(!instance.is_null());
        let mut r = unsafe { Self::from_runtime_instance(instance) };
        let t = go(&mut r);
        mem::forget(r);
        t
    }

    fn factory(&self) -> Arc<F> {
        self.0.clone()
    }

    pub unsafe fn init(
        &self,
        instance: *mut RuntimeInstance,
        n_qubits: u64,
        start: u64,
        argc: u32,
        argv: *const *const ffi::c_char,
    ) -> i32 {
        if instance.is_null() {
            eprintln!("cannot initialize runtime plugin: provided instance is null");
            return -1;
        }

        let args: Vec<String> = unsafe {
            let mut v = vec!["lib".to_string()];
            v.extend(convert_cargs_to_strings(argc, argv));
            v
        };

        result_to_errno(
            "Failed to initialize the runtime plugin",
            self.factory()
                .init(n_qubits, start.into(), args.as_ref())
                .map(|runtime| unsafe {
                    *instance = Self::into_runtime_instance(runtime);
                }),
        )
    }

    pub unsafe fn exit(instance: RuntimeInstance) -> Errno {
        // For now we don't: unconditionally drop the box here.
        // `selene_runtime_exit` is specified to cause all operations to fail on
        // the RuntimeInstance, but we don't actually check for that right now.
        // If we uncommented the below, to drop the box, then future calls to
        // other functions with that instance would be undefined behaviour.
        // let _ = Self::from_runtime_instance(instance);
        result_to_errno(
            "Failed to exit the runtime plugin",
            Self::with_runtime_instance(instance, |runtime| runtime.exit()),
        )
    }
    pub unsafe fn get_next_operations(
        instance: RuntimeInstance,
        goi: RuntimeGetOperationInstance,
        callbacks: *const RuntimeGetOperationInterface,
    ) -> Errno {
        result_to_errno(
            "Failed in get_next_operations",
            Self::with_runtime_instance(instance, |runtime| {
                let Some(batch) = runtime.get_next_operations()? else {
                    return anyhow::Ok(());
                };

                let (start, duration) = (batch.start(), batch.duration());
                let RuntimeGetOperationInterface {
                    rzz_fn,
                    rxy_fn,
                    rz_fn,
                    measure_fn,
                    measure_leaked_fn,
                    reset_fn,
                    custom_fn,
                    set_batch_time_fn,
                    ..
                } = unsafe { &*callbacks };
                unsafe { set_batch_time_fn(goi, start.into(), duration.into()) };
                for op in batch {
                    match op {
                        Operation::Measure {
                            qubit_id,
                            result_id,
                        } => unsafe { measure_fn(goi, qubit_id, result_id) },
                        Operation::MeasureLeaked {
                            qubit_id,
                            result_id,
                        } => unsafe { measure_leaked_fn(goi, qubit_id, result_id) },
                        Operation::Reset { qubit_id } => unsafe { reset_fn(goi, qubit_id) },
                        Operation::RZGate { qubit_id, theta } => unsafe {
                            rz_fn(goi, qubit_id, theta)
                        },
                        Operation::RXYGate {
                            qubit_id,
                            theta,
                            phi,
                        } => unsafe { rxy_fn(goi, qubit_id, theta, phi) },
                        Operation::RZZGate {
                            qubit_id_1,
                            qubit_id_2,
                            theta,
                        } => unsafe { rzz_fn(goi, qubit_id_1, qubit_id_2, theta) },
                        Operation::Custom { custom_tag, data } => {
                            let (ptr, len) = (data.as_ptr() as *const ffi::c_void, data.len());
                            unsafe { custom_fn(goi, custom_tag, ptr, len) }
                        }
                    }
                }
                anyhow::Ok(())
            }),
        )
    }

    pub unsafe fn shot_start(instance: RuntimeInstance, shot_id: u64, seed: u64) -> Errno {
        result_to_errno(
            format!("Failed to start shot {shot_id}"),
            Self::with_runtime_instance(instance, |runtime| runtime.shot_start(shot_id, seed)),
        )
    }

    pub unsafe fn shot_end(instance: RuntimeInstance) -> Errno {
        result_to_errno(
            "Failed to end shot",
            Self::with_runtime_instance(instance, |runtime| runtime.shot_end()),
        )
    }

    pub unsafe fn custom_call(
        instance: RuntimeInstance,
        tag: u64,
        data: *const ffi::c_void,
        data_len: usize,
        result: *mut u64,
    ) -> Errno {
        let data = unsafe { std::slice::from_raw_parts(data as *const u8, data_len) };
        result_to_errno(
            "Failed in unsafe_call",
            Self::with_runtime_instance(instance, |runtime| {
                let r = runtime.custom_call(tag, data)?;
                unsafe { *result = r };
                anyhow::Ok(())
            }),
        )
    }

    pub unsafe fn get_metric(
        instance: RuntimeInstance,
        nth_metric: u8,
        tag_ptr: *mut ffi::c_char,
        datatype_ptr: *mut u8,
        data_ptr: *mut u64,
    ) -> i32 {
        result_of_errno_to_errno(
            "Failed in get_metric",
            Self::with_runtime_instance(instance, |runtime| {
                let Some((tag, metric)) = runtime.get_metric(nth_metric)? else {
                    return anyhow::Ok(1);
                };
                unsafe { metric.write_raw(tag, tag_ptr, datatype_ptr, data_ptr) }
                Ok(0)
            }),
        )
    }

    pub unsafe fn qalloc(instance: RuntimeInstance, result: *mut u64) -> Errno {
        result_to_errno(
            "Failed in qalloc",
            Self::with_runtime_instance(instance, |runtime| {
                let r = runtime.qalloc()?;
                unsafe { *result = r };
                anyhow::Ok(())
            }),
        )
    }

    pub unsafe fn qfree(instance: RuntimeInstance, qubit_id: u64) -> Errno {
        result_to_errno(
            "Failed in qfree",
            Self::with_runtime_instance(instance, |runtime| runtime.qfree(qubit_id)),
        )
    }

    pub unsafe fn rxy_gate(
        instance: RuntimeInstance,
        qubit_id: u64,
        theta: f64,
        phi: f64,
    ) -> Errno {
        result_to_errno(
            "Failed in rxy_gate",
            Self::with_runtime_instance(instance, |runtime| runtime.rxy_gate(qubit_id, theta, phi)),
        )
    }

    pub unsafe fn rzz_gate(
        instance: RuntimeInstance,
        qubit_id_1: u64,
        qubit_id_2: u64,
        theta: f64,
    ) -> Errno {
        result_to_errno(
            "Failed in rzz_gate",
            Self::with_runtime_instance(instance, |runtime| {
                runtime.rzz_gate(qubit_id_1, qubit_id_2, theta)
            }),
        )
    }

    pub unsafe fn rz_gate(instance: RuntimeInstance, qubit_id: u64, theta: f64) -> Errno {
        result_to_errno(
            "Failed in rz_gate",
            Self::with_runtime_instance(instance, |runtime| runtime.rz_gate(qubit_id, theta)),
        )
    }

    pub unsafe fn measure(instance: RuntimeInstance, qubit_id: u64, result: *mut u64) -> Errno {
        result_to_errno(
            "Failed in measure",
            Self::with_runtime_instance(instance, |runtime| {
                let r = runtime.measure(qubit_id)?;
                unsafe { *result = r };
                anyhow::Ok(())
            }),
        )
    }

    pub unsafe fn measure_leaked(
        instance: RuntimeInstance,
        qubit_id: u64,
        result: *mut u64,
    ) -> Errno {
        result_to_errno(
            "Failed in measure_leaked",
            Self::with_runtime_instance(instance, |runtime| {
                let r = runtime.measure_leaked(qubit_id)?;
                unsafe { *result = r };
                anyhow::Ok(())
            }),
        )
    }

    pub unsafe fn reset(instance: RuntimeInstance, qubit_id: u64) -> Errno {
        result_to_errno(
            "Failed in reset",
            Self::with_runtime_instance(instance, |runtime| runtime.reset(qubit_id)),
        )
    }

    pub unsafe fn force_result(instance: RuntimeInstance, result_id: u64) -> Errno {
        result_to_errno(
            "Failed in force_result",
            Self::with_runtime_instance(instance, |runtime| runtime.force_result(result_id)),
        )
    }

    pub unsafe fn get_bool_result(
        instance: RuntimeInstance,
        result_id: u64,
        result: *mut i8,
    ) -> Errno {
        result_to_errno(
            "Failed in get_bool_result",
            Self::with_runtime_instance(instance, |runtime| {
                match runtime.get_bool_result(result_id)? {
                    Some(r) => {
                        unsafe { *result = r as i8 };
                    }
                    None => unsafe {
                        *result = -1i8;
                    },
                };
                anyhow::Ok(())
            }),
        )
    }

    pub unsafe fn get_u64_result(
        instance: RuntimeInstance,
        result_id: u64,
        result: *mut u64,
    ) -> Errno {
        result_to_errno(
            "Failed in get_u64_result",
            Self::with_runtime_instance(instance, |runtime| {
                match runtime.get_u64_result(result_id)? {
                    Some(r) => {
                        unsafe { *result = r };
                    }
                    None => unsafe {
                        *result = u64::MAX;
                    },
                };
                anyhow::Ok(())
            }),
        )
    }

    pub unsafe fn set_bool_result(
        instance: RuntimeInstance,
        result_id: u64,
        result: bool,
    ) -> Errno {
        result_to_errno(
            "Failed in set_bool_result",
            Self::with_runtime_instance(instance, |runtime| {
                runtime.set_bool_result(result_id, result)
            }),
        )
    }

    pub unsafe fn set_u64_result(instance: RuntimeInstance, result_id: u64, result: u64) -> Errno {
        result_to_errno(
            "Failed in set_u64_result",
            Self::with_runtime_instance(instance, |runtime| {
                runtime.set_u64_result(result_id, result)
            }),
        )
    }

    pub unsafe fn increment_future_refcount(instance: RuntimeInstance, future_ref: u64) -> Errno {
        result_to_errno(
            "Failed in increment_future_refcount",
            Self::with_runtime_instance(instance, |runtime| {
                runtime.increment_future_refcount(future_ref)
            }),
        )
    }

    pub unsafe fn decrement_future_refcount(instance: RuntimeInstance, future_ref: u64) -> Errno {
        result_to_errno(
            "Failed in decrement_future_refcount",
            Self::with_runtime_instance(instance, |runtime| {
                runtime.decrement_future_refcount(future_ref)
            }),
        )
    }

    pub unsafe fn global_barrier(instance: RuntimeInstance, sleep_ns: u64) -> Errno {
        result_to_errno(
            "Failed in global barrier",
            Self::with_runtime_instance(instance, |runtime| runtime.global_barrier(sleep_ns)),
        )
    }
    pub unsafe fn local_barrier(
        instance: RuntimeInstance,
        qubits: *const u64,
        qubits_len: u64,
        sleep_ns: u64,
    ) -> Errno {
        let qubits = unsafe { std::slice::from_raw_parts(qubits, qubits_len as usize) };
        result_to_errno(
            "Failed in local barrier",
            Self::with_runtime_instance(instance, |runtime| {
                runtime.local_barrier(qubits, sleep_ns)
            }),
        )
    }
}

#[macro_export]
/// A macro to export a runtime plugin from a crate.
///
/// Expands to a module named `_plugin` with `pub extern "C" unsafe` definitions
/// of all entry points specified in the documentation of
/// [super::plugin::RuntimePluginInterface].
///
/// Those functions are all implemented using the `$factory_type`'s impl of
/// [RuntimeInterfaceFactory].
///
/// A crate can only export a single runtime plugin (although it may export
/// other plugins of other types). See the `selene-simple-runtime-plugin` python
/// package for a fully worked example.
macro_rules! export_runtime_plugin {
    ($factory_type:ty) => {
        mod _plugin {
            use selene_core::runtime::{
                interface::RuntimeInterfaceFactory,
                plugin::{
                    Errno, RuntimeGetOperationInstance, RuntimeGetOperationInterface,
                    RuntimeInstance,
                },
                version::CURRENT_API_VERSION,
            };

            use std::cell::LazyCell;
            use std::ffi::{c_char, c_void};

            /// cbindgen:ignore
            type Helper = selene_core::runtime::helper::Helper<$factory_type>;

            // Enforce that factory_type implements the RuntimeInterfaceFactory trait
            const _: fn() = || {
                fn _assert_impl<F: RuntimeInterfaceFactory>() {}
                _assert_impl::<$factory_type>();
            };

            /// The API version comprises four unsigned 8-bit integers:
            ///     - reserved: 8 bits (must be 0)
            ///     - major: 8 bits
            ///     - minor: 8 bits
            ///     - patch: 8 bits
            ///
            /// Selene maintains its own API version for the runtime API
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
            pub unsafe extern "C" fn selene_runtime_get_api_version() -> u64 {
                CURRENT_API_VERSION.into()
            }

            /// When Selene is initialised, it is provided with a default argument
            /// (the maximum number of qubits) and some custom arguments for the runtime.
            /// These arguments are provided to the error model through this initialization
            /// function, with the custom arguments passed in in an argc, argv format.
            ///
            /// It is this function's responsibility to parse and validate those user-provided
            /// arguments and initialise a plugin instance ready for a call to
            /// selene_runtime_shot_start(). The `instance` pointer is designed to be set
            /// by this function and hold all relevant state, such that subsequent calls to the
            /// corresponding instance will be able to access that state, and such that calls to
            /// other instances will not be impacted.
            ///
            /// Runtime plugins should provide customisation of parameter values
            /// within their python implementations. They should also define how those
            /// parameters are converted to an argv list to be passed to their compiled
            /// counterparts.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_init(
                instance: *mut RuntimeInstance,
                n_qubits: u64,
                start: u64,
                argc: u32,
                argv: *const *const c_char,
            ) -> i32 {
                use std::cell::OnceCell;
                use std::sync::Mutex;
                static FACTORY: Mutex<OnceCell<Helper>> = Mutex::new(OnceCell::new());
                FACTORY
                    .lock()
                    .unwrap()
                    .get_or_init(|| Helper::default())
                    .init(instance, n_qubits, start.into(), argc, argv)
            }

            /// This function is called when Selene is exiting, and it is responsible for
            /// cleaning up any resources that the runtime plugin has allocated.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_exit(instance: RuntimeInstance) -> i32 {
                Helper::exit(instance)
            }

            /// This function is called at the start of a shot, and it is responsible for
            /// initialising the runtime plugin for that shot. The seed is
            /// provided for RNG seeding, and it is highly recommended that all randomness
            /// used by the plugin is seeded with this value. Most runtimes will not require
            /// randomness at all, but it is viable that some sorting algorithms or other
            /// non-deterministic algorithms may make use of it.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_shot_start(
                instance: RuntimeInstance,
                shot_id: u64,
                seed: u64,
            ) -> i32 {
                Helper::shot_start(instance, shot_id, seed)
            }

            /// This function is called at the end of a shot, and it is responsible for
            /// cleaning up any resources that the runtime plugin has allocated for that shot.
            /// For example, it may clean up any operation buffers or accumulators. A call to
            /// this function will usually be followed either by a call to
            /// `selene_runtime_shot_start` to prepare for the following shot, or by
            /// a call to `selene_runtime_exit` to shut down the instance.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_shot_end(
                instance: RuntimeInstance,
                shot_id: u64,
                seed: u64,
            ) -> i32 {
                Helper::shot_end(instance)
            }

            /// This function is called to provide a runtime with custom operations from
            /// a user program if supported. The `tag` should be a unique identifier for
            /// the runtime to interpret to identify the operation that is being requested,
            /// and the data and data_len should contain any relevant data in a format
            /// understood by the runtime.
            ///
            /// A hypothetical example of the use of this function is a proprietary runtime
            /// that supports a custom operation that is not part of the standard interface,
            /// and does not require direct Selene support. For example, if the runtime includes
            /// functionality such as "run this tuned subroutine on these qubits", the
            /// user program need not know about the fine details of that subroutine. It only
            /// needs to encode the qubit list in a format that is understood by the proprietary
            /// runtime, and it can effectively bypass Selene. Selene will nonetheless perform
            /// a call to get the next operations immediately afterwards such that any required
            /// operations are still performed.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_custom_call(
                instance: RuntimeInstance,
                tag: u64,
                data: *const c_void,
                data_len: usize,
                result: *mut u64,
            ) -> Errno {
                Helper::custom_call(instance, tag, data, data_len, result)
            }

            /// This function is called to get the next operations from the runtime. The
            /// runtime should use the [RuntimeGetOperationInterface] callbacks along with
            /// the [RuntimeGetOperationInstance] to provide a list of operations to Selene
            /// for processing through the error model and simulator.
            ///
            /// Sometimes the runtime may wish to provide extra "Custom" operations that might
            /// be understood by the error model, or might be useful for the user to extract
            /// through the CircuitExtractor for interpretation.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_get_next_operations(
                instance: RuntimeInstance,
                goi: RuntimeGetOperationInstance,
                callbacks: *const RuntimeGetOperationInterface,
            ) -> Errno {
                Helper::get_next_operations(instance, goi, callbacks)
            }

            /// This function is called to retrieve any metrics that the runtime is willing
            /// to provide, e.g. the number of operations left in its internal queue, the number
            /// of qubits allocated, etc. An example of exposing metrics is provided in the
            /// error model documentation.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_get_metrics(
                instance: RuntimeInstance,
                nth_metric: u8,
                tag_ptr: *mut c_char,
                datatype_ptr: *mut u8,
                data_ptr: *mut u64,
            ) -> Errno {
                Helper::get_metric(instance, nth_metric, tag_ptr, datatype_ptr, data_ptr)
            }

            /// Instruct the runtime to try to allocate a free qubit.
            ///
            /// If successful, write the qubit ID to the `result` pointer and return 0.
            /// If the runtime is unable to allocate a qubit:
            /// - If it is simply out of qubits, write `u64::MAX` to the `result` pointer
            ///   and return 0. It is then up to the user program to handle this how it sees
            ///   fit (e.g. error out, or try a different algorithm that doesn't require an
            ///   additional qubit).
            /// - If the runtime is unable to allocate a qubit for any other reason, such
            ///   as an internal error, it should return a non-zero error code.
            ///
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_qalloc(
                instance: RuntimeInstance,
                result: *mut u64,
            ) -> i32 {
                Helper::qalloc(instance, result)
            }

            /// Instruct the runtime to free a qubit with the given ID.
            ///
            /// If the qubit is successfully freed, return 0.
            /// If the qubit ID is invalid or the runtime is unable to free the qubit for any
            /// reason, return a non-zero error code.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_qfree(
                instance: RuntimeInstance,
                qubit_id: u64,
            ) -> i32 {
                Helper::qfree(instance, qubit_id)
            }

            /// Instruct the runtime to enforce a local optimisation barrier on the provided
            /// qubits.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_local_barrier(
                instance: RuntimeInstance,
                qubits: *const u64,
                qubits_len: u64,
                sleep_ns: u64,
            ) -> i32 {
                Helper::local_barrier(instance, qubits, qubits_len, sleep_ns)
            }

            /// Instruct the runtime to enforce a global optimisation barrier.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_global_barrier(
                instance: RuntimeInstance,
                sleep_ns: u64,
            ) -> i32 {
                Helper::global_barrier(instance, sleep_ns)
            }

            /// Instruct the runtime to apply an RXY gate to the qubit with the given ID.
            /// Note that it is up to the runtime whether or not this gate is applied immediately:
            /// The runtime might act lazily and apply the gate at a later time when an observable
            /// outcome is requested.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_rxy_gate(
                instance: RuntimeInstance,
                qubit_id: u64,
                theta: f64,
                phi: f64,
            ) -> i32 {
                Helper::rxy_gate(instance, qubit_id, theta, phi)
            }

            /// Instruct the runtime to apply an RZZ gate to the qubits with the given IDs.
            /// Note that it is up to the runtime whether or not this gate is applied
            /// immediately: The runtime might act lazily and apply the gate at a later time.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_rzz_gate(
                instance: RuntimeInstance,
                qubit_id_1: u64,
                qubit_id_2: u64,
                theta: f64,
            ) -> i32 {
                Helper::rzz_gate(instance, qubit_id_1, qubit_id_2, theta)
            }

            /// Instruct the runtime to apply an RZ gate to the qubit with the given ID.
            /// Note that it is up to the runtime whether or not this gate is applied
            /// immediately: The runtime might act lazily and apply the gate at a later time.
            /// It might not apply it at all, as RZ may be elided in code.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_rz_gate(
                instance: RuntimeInstance,
                qubit_id: u64,
                theta: f64,
            ) -> i32 {
                Helper::rz_gate(instance, qubit_id, theta)
            }

            /// Instruct the runtime that a measurement is to be requested and to write
            /// a reference ID to the result to the `result` pointer.
            ///
            /// Note that this doesn't return a boolean, as it is not providing the measurement
            /// result itself. Instead, it is providing a reference ID that can be used later
            /// to retrieve the measurement result if it is available. This allows for queuing up
            /// several measurements to be performed at once, then reading the results one by one.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_measure(
                instance: RuntimeInstance,
                qubit_id: u64,
                result: *mut u64,
            ) -> i32 {
                Helper::measure(instance, qubit_id, result)
            }

            /// Instruct the runtime that a measurement is to be requested with additional
            /// capabilities to provide leakage information, and to write a reference ID
            /// to the result to the `result` pointer. This result is a u64, packed with
            /// appropriate information for the additional capabilities.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_measure_leaked(
                instance: RuntimeInstance,
                qubit_id: u64,
                result: *mut u64,
            ) -> i32 {
                Helper::measure_leaked(instance, qubit_id, result)
            }

            /// Instruct the runtime to reset a qubit to the |0> state with the given ID.
            /// Note that it is up to the runtime whether or not this reset is applied immediately.
            /// It may wait until it is required, e.g. another gate is applied to the qubit.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_reset(
                instance: RuntimeInstance,
                qubit_id: u64,
            ) -> i32 {
                Helper::reset(instance, qubit_id)
            }

            /// Instruct the runtime to force a result with the given ID to be made available, e.g.
            /// that a measurement must be performed now if it has not been performed yet.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_force_result(
                instance: RuntimeInstance,
                result_id: u64,
            ) -> i32 {
                Helper::force_result(instance, result_id)
            }

            /// Read a bool result from the runtime.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_get_bool_result(
                instance: RuntimeInstance,
                result_id: u64,
                result: *mut i8,
            ) -> i32 {
                Helper::get_bool_result(instance, result_id, result)
            }

            /// Read a u64 result from the runtime.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_get_u64_result(
                instance: RuntimeInstance,
                result_id: u64,
                result: *mut u64,
            ) -> i32 {
                Helper::get_u64_result(instance, result_id, result)
            }

            /// If the result relies on a quantum operation such as a measurement, then forcing the
            /// result might flush a measurement operation (amongst other things) to Selene for the
            /// error model and simulator to act upon. set_bool_result is used by Selene to provide
            /// the result back to the runtime for later reporting to the user program.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_set_bool_result(
                instance: RuntimeInstance,
                result_id: u64,
                result: bool,
            ) -> i32 {
                Helper::set_bool_result(instance, result_id, result)
            }

            /// If the result relies on a quantum operation such as a measurement, then forcing the
            /// result might flush a measurement operation (amongst other things) to Selene for the
            /// error model and simulator to act upon. set_bool_result is used by Selene to provide
            /// the result back to the runtime for later reporting to the user program.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_runtime_set_u64_result(
                instance: RuntimeInstance,
                result_id: u64,
                result: u64,
            ) -> i32 {
                Helper::set_u64_result(instance, result_id, result)
            }

            /// Increment the reference count of a future ID.
            #[unsafe(no_mangle)]
            unsafe extern "C" fn selene_runtime_increment_future_refcount(
                instance: RuntimeInstance,
                future_ref: u64,
            ) -> Errno {
                Helper::increment_future_refcount(instance, future_ref)
            }

            /// Decrement the reference count of a future ID. If the reference count reaches
            /// 0, nothing should be waiting upon it and it is considered ready to be deallocated.
            #[unsafe(no_mangle)]
            unsafe extern "C" fn selene_runtime_decrement_future_refcount(
                instance: RuntimeInstance,
                future_ref: u64,
            ) -> Errno {
                Helper::decrement_future_refcount(instance, future_ref)
            }
        }
    };
}
