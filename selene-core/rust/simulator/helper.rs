//! Defines the [crate::export_simulator_plugin] and helpers for implementing
//! error model plugins as rust crates.
//!
//! See `selene-coinflip-plugin` for an example of how to implement a plugin.
use std::{ffi, mem, sync::Arc};

use super::{
    SimulatorInterface,
    interface::SimulatorInterfaceFactory,
    plugin::{Errno, SimulatorInstance},
};
use crate::utils::{convert_cargs_to_strings, result_of_errno_to_errno, result_to_errno};

#[derive(Default)]
/// A helper struct used by [crate::export_simulator_plugin] to implement the simulator
/// plugin entry points defined by [super::plugin::SimulatorPluginInterface] using an
/// implementation of [SimulatorInterfaceFactory].
pub struct Helper<F>(Arc<F>);

impl<F: SimulatorInterfaceFactory> Helper<F> {
    fn into_simulator_instance(s: Box<F::Interface>) -> SimulatorInstance {
        Box::into_raw(s) as SimulatorInstance
    }

    unsafe fn from_simulator_instance(instance: SimulatorInstance) -> Box<F::Interface> {
        assert!(!instance.is_null());
        unsafe { Box::from_raw(instance as *mut F::Interface) }
    }

    fn with_simulator_instance<T>(
        instance: SimulatorInstance,
        mut go: impl FnMut(&mut F::Interface) -> T,
    ) -> T {
        let mut s = unsafe { Self::from_simulator_instance(instance) };
        let t = go(&mut s);
        mem::forget(s);
        t
    }

    fn factory(&self) -> Arc<F> {
        self.0.clone()
    }

    pub unsafe fn init(
        &self,
        instance: *mut SimulatorInstance,
        n_qubits: u64,
        argc: u32,
        argv: *const *const ffi::c_char,
    ) -> Errno {
        if instance.is_null() {
            eprintln!("cannot initialize plugin: provided instance is null");
            return -1;
        }

        let args: Vec<String> = {
            let mut v = vec!["lib".to_string()];
            v.extend(unsafe { convert_cargs_to_strings(argc, argv) });
            v
        };

        result_to_errno(
            "Failed to initialize the simulator plugin",
            self.factory()
                .init(n_qubits, args.as_ref())
                .map(|simulator| unsafe {
                    *instance = Self::into_simulator_instance(simulator);
                }),
        )
    }

    pub unsafe fn exit(instance: SimulatorInstance) -> Errno {
        result_to_errno(
            "Failed to exit the simulator plugin",
            Self::with_simulator_instance(instance, |simulator| simulator.exit()),
        )
    }
    pub unsafe fn shot_start(instance: SimulatorInstance, shot_id: u64, seed: u64) -> Errno {
        result_to_errno(
            format!("Failed to start shot {shot_id}"),
            Self::with_simulator_instance(instance, |simulator| {
                simulator.shot_start(shot_id, seed)
            }),
        )
    }
    pub unsafe fn shot_end(instance: SimulatorInstance) -> Errno {
        result_to_errno(
            "Failed to end the current shot",
            Self::with_simulator_instance(instance, |simulator| simulator.shot_end()),
        )
    }
    pub unsafe fn get_metric(
        instance: SimulatorInstance,
        nth_metric: u8,
        tag_ptr: *mut ffi::c_char,
        datatype_ptr: *mut u8,
        data_ptr: *mut u64,
    ) -> Errno {
        result_of_errno_to_errno(
            "Failed in get_metric",
            Self::with_simulator_instance(instance, |simulator| {
                let Some((tag, metric)) = simulator.get_metric(nth_metric)? else {
                    return anyhow::Ok(1);
                };
                unsafe { metric.write_raw(tag, tag_ptr, datatype_ptr, data_ptr) }
                Ok(0)
            }),
        )
    }
    pub unsafe fn dump_state(
        instance: SimulatorInstance,
        file: *const ffi::c_char,
        qubits: *const u64,
        n_qubits: u64,
    ) -> Errno {
        let path = unsafe { std::path::PathBuf::from(ffi::CStr::from_ptr(file).to_str().unwrap()) };
        result_to_errno(
            format!("Failed to dump state to {path:?}"),
            Self::with_simulator_instance(instance, |simulator| {
                simulator.dump_state(&path, unsafe {
                    std::slice::from_raw_parts(qubits, n_qubits as usize)
                })
            }),
        )
    }
    pub unsafe fn rxy(instance: SimulatorInstance, qubit: u64, theta: f64, phi: f64) -> Errno {
        result_to_errno(
            "Failed to apply RXY gate",
            Self::with_simulator_instance(instance, |simulator| simulator.rxy(qubit, theta, phi)),
        )
    }
    pub unsafe fn rz(instance: SimulatorInstance, qubit: u64, theta: f64) -> Errno {
        result_to_errno(
            "Failed to apply RZ gate",
            Self::with_simulator_instance(instance, |simulator| simulator.rz(qubit, theta)),
        )
    }
    pub unsafe fn rzz(instance: SimulatorInstance, qubit1: u64, qubit2: u64, theta: f64) -> Errno {
        result_to_errno(
            "Failed to apply RZZ gate",
            Self::with_simulator_instance(instance, |simulator| {
                simulator.rzz(qubit1, qubit2, theta)
            }),
        )
    }
    pub unsafe fn measure(instance: SimulatorInstance, qubit: u64) -> Errno {
        let result = Self::with_simulator_instance(instance, |simulator| simulator.measure(qubit));
        match result {
            Ok(false) => 0,
            Ok(true) => 1,
            Err(e) => {
                eprintln!("Failed to measure qubit {qubit}: {e:?}");
                -1
            }
        }
    }
    pub unsafe fn postselect(instance: SimulatorInstance, qubit: u64, target_value: bool) -> Errno {
        result_to_errno(
            "Failed to postselect qubit",
            Self::with_simulator_instance(instance, |simulator| {
                simulator.postselect(qubit, target_value)
            }),
        )
    }
    pub unsafe fn reset(instance: SimulatorInstance, qubit: u64) -> Errno {
        result_to_errno(
            "Failed to reset qubit",
            Self::with_simulator_instance(instance, |simulator| simulator.reset(qubit)),
        )
    }
}

#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! crate_to_inline_simulator {
    () => {
        SimulatorInlineInterface {
            init_fn: crate::selene_simulator_init,
            exit_fn: Some(crate::selene_simulator_exit),
            shot_start_fn: crate::selene_simulator_shot_start,
            shot_end_fn: crate::selene_simulator_shot_end,
            rxy_fn: crate::selene_simulator_operation_rxy,
            rzz_fn: crate::selene_simulator_operation_rzz,
            rx_fn: crate::selene_simulator_operation_rz,
            measure_fn: crate::selene_simulator_operation_measure,
            reset_fn: crate::selene_simulator_operation_reset,
            postselect_fn: Some(crate::selene_simulator_operation_postselect),
            get_metrics_fn: Some(crate::selene_simulator_get_metrics),
            dump_state_fn: crate::selene_simulator_dump_state,
        }
    };
}

#[macro_export]
/// A macro to export a simulator plugin from a crate
///
/// Expands to a module named `_plugin` with `pub extern "C" unsafe` definitions
/// of all entry points specified in the documentation of
/// [super::plugin::SimulatorPluginInterface]
///
/// Those functions are all implemented using the `$factory_type`'s impl of
/// [SimulatorInterfaceFactory].
///
/// A crate can only export a single runtime plugin (although it may export
/// other plugins of other types). See the `selene-coinflip-plugin` python package
/// for a fully worked example.
macro_rules! export_simulator_plugin {
    ($factory_type:ty) => {
        mod _plugin {
            use selene_core::simulator::{
                interface::SimulatorInterfaceFactory,
                plugin::{Errno, SimulatorInstance},
                version::CURRENT_API_VERSION,
            };

            use std::cell::LazyCell;
            use std::ffi::c_char;

            /// cbindgen:ignore
            type Helper = selene_core::simulator::helper::Helper<$factory_type>;

            // Enforce that $struct_name implements the SimulatorInterfaceFactory trait
            const _: fn() = || {
                fn _assert_impl<T: SimulatorInterfaceFactory>() {}
                _assert_impl::<$factory_type>();
            };

            /// The API version comprises four unsigned 8-bit integers:
            ///     - reserved: 8 bits (must be 0)
            ///     - major: 8 bits
            ///     - minor: 8 bits
            ///     - patch: 8 bits
            ///
            /// Selene maintains its own API version for the simulator
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
            pub unsafe extern "C" fn selene_simulator_get_api_version() -> u64 {
                // The API version is defined in the version module
                // and is used to check compatibility between the plugin and the runtime.
                CURRENT_API_VERSION.into()
            }

            /// When Selene is initialised, it is provided with some default arguments
            /// (the maximum number of qubits, the path to a simulator plugin to use, etc)
            /// and some custom arguments for the simulator. These arguments are provided
            /// to the simulator through this initialization function, with
            /// the custom arguments passed in in an argc, argv format.
            ///
            /// It is this function's responsibility to parse and validate those user-provided
            /// arguments and initialise a plugin instance ready for a call to
            /// selene_simulator_shot_start(). The `instance` pointer is designed to be set
            /// by this function and hold all relevant state, such that subsequent calls to the
            /// corresponding instance will be able to access that state, and such that calls to
            /// other instances will not be impacted.
            ///
            /// Simulator plugins should provide customisation of parameter values
            /// within their python implementations. They should also define how those
            /// parameters are converted to an argv list to be passed to their compiled
            /// counterparts.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_init(
                instance: *mut SimulatorInstance,
                n_qubits: u64,
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
                    .init(instance, n_qubits, argc, argv)
            }

            /// This function is called when Selene is exiting, and it is responsible for
            /// cleaning up any resources that the simulator plugin has allocated.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_exit(instance: SimulatorInstance) -> i32 {
                Helper::exit(instance)
            }

            /// This function is called at the start of a shot, and it is responsible for
            /// initialising the simulator plugin for that shot. The seed is provided for
            /// RNG seeding, and it is highly recommended that all randomness used by the
            /// simulator is seeded with this value.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_shot_start(
                instance: SimulatorInstance,
                shot_id: u64,
                seed: u64,
            ) -> i32 {
                Helper::shot_start(instance, shot_id, seed)
            }

            /// This function is called at the end of a shot, and it is responsible for
            /// finalising the simulator plugin for that shot. For example, it may
            /// clean up any state, such as accumulators or buffers, or set a state vector
            /// to zero. We recommenA call to
            /// this function will usually be followed either by a call to
            /// `selene_simulator_shot_start` to prepare for the following shot, or by
            /// a call to `selene_simulator_exit` to shut down the instance.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_shot_end(
                instance: SimulatorInstance,
                seed: u64,
            ) -> i32 {
                Helper::shot_end(instance)
            }

            /// Apply an RXY gate to the qubit at the requested index, with the provided
            /// angles. This gate is also commonly known as the PhasedX gate or the R1XY gate,
            /// performing:
            /// $R_z(\phi)R_x(\theta)R_z(-\phi)$
            /// (in matrix-multiplication order).
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_operation_rxy(
                instance: SimulatorInstance,
                qubit: u64,
                theta: f64,
                phi: f64,
            ) -> i32 {
                Helper::rxy(instance, qubit, theta, phi)
            }

            /// Apply an RZZ gate to the qubits at the requested indices, with the provided
            /// angles. This gate is also commonly known as the ZZPhase gate or the R2ZZ gate,
            /// performing
            /// $diag(\chi^*, \chi, \chi, \chi^*)$
            /// where
            /// $\chi = \exp(i \pi \theta / 2)$
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_operation_rzz(
                instance: SimulatorInstance,
                qubit1: u64,
                qubit2: u64,
                theta: f64,
            ) -> i32 {
                Helper::rzz(instance, qubit1, qubit2, theta)
            }

            /// Apply an RZ gate to the qubit at the requested index, with the
            /// provided angle. This should perform:
            /// $diag(\chi^*, \chi)$
            /// where
            /// $chi = \exp(i \pi \theta / 2)$
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_operation_rz(
                instance: SimulatorInstance,
                qubit: u64,
                theta: f64,
            ) -> i32 {
                Helper::rz(instance, qubit, theta)
            }

            /// Measure the qubit at the requested index. This is a destructive
            /// operation.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_operation_measure(
                instance: SimulatorInstance,
                qubit: u64,
            ) -> i32 {
                Helper::measure(instance, qubit)
            }

            /// Postselect the qubit at the requested index. Some simulators may
            /// choose to not support post-selection, in which case this function
            /// should return an error.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_operation_postselect(
                instance: SimulatorInstance,
                qubit: u64,
                target_value: bool,
            ) -> i32 {
                Helper::postselect(instance, qubit, target_value)
            }

            /// Reset the qubit at the requested index to the |0> state.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_operation_reset(
                instance: SimulatorInstance,
                qubit: u64,
            ) -> i32 {
                Helper::reset(instance, qubit)
            }

            /// Get a metric from the simulator instance.
            ///
            /// nth_metric is the index of the metric to retrieve, starting from 0,
            /// and called with incrementing nth_metric until a nonzero value is
            /// returned. The tag_ptr, datatype_ptr and data_ptr are out-parameters
            /// for the plugin to write values into. See the example in the
            /// error_model documentation for details on how to write those values.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_get_metrics(
                instance: SimulatorInstance,
                nth_metric: u8,
                tag_ptr: *mut c_char,
                datatype_ptr: *mut u8,
                data_ptr: *mut u64,
            ) -> i32 {
                Helper::get_metric(instance, nth_metric, tag_ptr, datatype_ptr, data_ptr)
            }

            /// Dump the internal state of the simulator to a file. A list of
            /// qubits is provided which corresponds to the ordering provided
            /// by the user, and the simulator should represent these in the file
            /// in a way that is parsable by the python component of the simulator,
            /// as the addresses of those qubits are only known at runtime.
            ///
            /// Simulators can return an error if they do not support this
            /// functionality, or if the file cannot be written to.
            ///
            /// The python component of the simulator should provide functionality
            /// for reading the resulting file. The filename will be written to
            /// the result stream.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn selene_simulator_dump_state(
                instance: SimulatorInstance,
                file: *const c_char,
                qubits: *const u64,
                n_qubits: u64,
            ) -> i32 {
                Helper::dump_state(instance, file, qubits, n_qubits)
            }
        }
    };
}
