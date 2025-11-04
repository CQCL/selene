/* Generated with cbindgen:0.28.0 */

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include "selene/core_types.h"


#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * The API version comprises four unsigned 8-bit integers:
 *     - reserved: 8 bits (must be 0)
 *     - major: 8 bits
 *     - minor: 8 bits
 *     - patch: 8 bits
 *
 * Selene maintains its own API version for the runtime API
 * and is updated upon changes to the API depending on how
 * breaking the changes are. Selene is also responsible for
 * validating the API version of the plugin against its own
 * version.
 *
 * The plans for this validation are a work-in-progress, but
 * currently selene will reject any plugin that has a different
 * major or minor version than the current Selene version, or with
 * a reserved field that is not 0.
 */
uint64_t selene_runtime_get_api_version(void);

/**
 * When Selene is initialised, it is provided with a default argument
 * (the maximum number of qubits) and some custom arguments for the runtime.
 * These arguments are provided to the error model through this initialization
 * function, with the custom arguments passed in in an argc, argv format.
 *
 * It is this function's responsibility to parse and validate those user-provided
 * arguments and initialise a plugin instance ready for a call to
 * selene_runtime_shot_start(). The `instance` pointer is designed to be set
 * by this function and hold all relevant state, such that subsequent calls to the
 * corresponding instance will be able to access that state, and such that calls to
 * other instances will not be impacted.
 *
 * Runtime plugins should provide customisation of parameter values
 * within their python implementations. They should also define how those
 * parameters are converted to an argv list to be passed to their compiled
 * counterparts.
 */
int32_t selene_runtime_init(RuntimeInstance *instance,
                            uint64_t n_qubits,
                            uint64_t start,
                            uint32_t argc,
                            const char *const *argv);

/**
 * This function is called when Selene is exiting, and it is responsible for
 * cleaning up any resources that the runtime plugin has allocated.
 */
int32_t selene_runtime_exit(RuntimeInstance instance);

/**
 * This function is called at the start of a shot, and it is responsible for
 * initialising the runtime plugin for that shot. The seed is
 * provided for RNG seeding, and it is highly recommended that all randomness
 * used by the plugin is seeded with this value. Most runtimes will not require
 * randomness at all, but it is viable that some sorting algorithms or other
 * non-deterministic algorithms may make use of it.
 */
int32_t selene_runtime_shot_start(RuntimeInstance instance,
                                  uint64_t shot_id,
                                  uint64_t seed);

/**
 * This function is called at the end of a shot, and it is responsible for
 * cleaning up any resources that the runtime plugin has allocated for that shot.
 * For example, it may clean up any operation buffers or accumulators. A call to
 * this function will usually be followed either by a call to
 * `selene_runtime_shot_start` to prepare for the following shot, or by
 * a call to `selene_runtime_exit` to shut down the instance.
 */
int32_t selene_runtime_shot_end(RuntimeInstance instance,
                                uint64_t shot_id,
                                uint64_t seed);

/**
 * This function is called to provide a runtime with custom operations from
 * a user program if supported. The `tag` should be a unique identifier for
 * the runtime to interpret to identify the operation that is being requested,
 * and the data and data_len should contain any relevant data in a format
 * understood by the runtime.
 *
 * A hypothetical example of the use of this function is a proprietary runtime
 * that supports a custom operation that is not part of the standard interface,
 * and does not require direct Selene support. For example, if the runtime includes
 * functionality such as "run this tuned subroutine on these qubits", the
 * user program need not know about the fine details of that subroutine. It only
 * needs to encode the qubit list in a format that is understood by the proprietary
 * runtime, and it can effectively bypass Selene. Selene will nonetheless perform
 * a call to get the next operations immediately afterwards such that any required
 * operations are still performed.
 */
SeleneErrno selene_runtime_custom_call(RuntimeInstance instance,
                                       uint64_t tag,
                                       const void *data,
                                       size_t data_len,
                                       uint64_t *result);

/**
 * This function is called to get the next operations from the runtime. The
 * runtime should use the [RuntimeGetOperationInterface] callbacks along with
 * the [RuntimeGetOperationInstance] to provide a list of operations to Selene
 * for processing through the error model and simulator.
 *
 * Sometimes the runtime may wish to provide extra "Custom" operations that might
 * be understood by the error model, or might be useful for the user to extract
 * through the CircuitExtractor for interpretation.
 */
SeleneErrno selene_runtime_get_next_operations(RuntimeInstance instance,
                                               SeleneRuntimeGetOperationInstance goi,
                                               const SeleneRuntimeGetOperationInterface *callbacks);

/**
 * This function is called to retrieve any metrics that the runtime is willing
 * to provide, e.g. the number of operations left in its internal queue, the number
 * of qubits allocated, etc. An example of exposing metrics is provided in the
 * error model documentation.
 */
SeleneErrno selene_runtime_get_metrics(RuntimeInstance instance,
                                       uint8_t nth_metric,
                                       char *tag_ptr,
                                       uint8_t *datatype_ptr,
                                       uint64_t *data_ptr);

/**
 * Instruct the runtime to try to allocate a free qubit.
 *
 * If successful, write the qubit ID to the `result` pointer and return 0.
 * If the runtime is unable to allocate a qubit:
 * - If it is simply out of qubits, write `u64::MAX` to the `result` pointer
 *   and return 0. It is then up to the user program to handle this how it sees
 *   fit (e.g. error out, or try a different algorithm that doesn't require an
 *   additional qubit).
 * - If the runtime is unable to allocate a qubit for any other reason, such
 *   as an internal error, it should return a non-zero error code.
 *
 */
int32_t selene_runtime_qalloc(RuntimeInstance instance,
                              uint64_t *result);

/**
 * Instruct the runtime to free a qubit with the given ID.
 *
 * If the qubit is successfully freed, return 0.
 * If the qubit ID is invalid or the runtime is unable to free the qubit for any
 * reason, return a non-zero error code.
 */
int32_t selene_runtime_qfree(RuntimeInstance instance,
                             uint64_t qubit_id);

/**
 * Instruct the runtime to enforce a local optimisation barrier on the provided
 * qubits.
 */
int32_t selene_runtime_local_barrier(RuntimeInstance instance,
                                     const uint64_t *qubits,
                                     uint64_t qubits_len,
                                     uint64_t sleep_ns);

/**
 * Instruct the runtime to enforce a global optimisation barrier.
 */
int32_t selene_runtime_global_barrier(RuntimeInstance instance,
                                      uint64_t sleep_ns);

/**
 * Instruct the runtime to apply an RXY gate to the qubit with the given ID.
 * Note that it is up to the runtime whether or not this gate is applied immediately:
 * The runtime might act lazily and apply the gate at a later time when an observable
 * outcome is requested.
 */
int32_t selene_runtime_rxy_gate(RuntimeInstance instance,
                                uint64_t qubit_id,
                                double theta,
                                double phi);

/**
 * Instruct the runtime to apply an RZZ gate to the qubits with the given IDs.
 * Note that it is up to the runtime whether or not this gate is applied
 * immediately: The runtime might act lazily and apply the gate at a later time.
 */
int32_t selene_runtime_rzz_gate(RuntimeInstance instance,
                                uint64_t qubit_id_1,
                                uint64_t qubit_id_2,
                                double theta);

/**
 * Instruct the runtime to apply an RZ gate to the qubit with the given ID.
 * Note that it is up to the runtime whether or not this gate is applied
 * immediately: The runtime might act lazily and apply the gate at a later time.
 * It might not apply it at all, as RZ may be elided in code.
 */
int32_t selene_runtime_rz_gate(RuntimeInstance instance,
                               uint64_t qubit_id,
                               double theta);

/**
 * Instruct the runtime that a measurement is to be requested and to write
 * a reference ID to the result to the `result` pointer.
 *
 * Note that this doesn't return a boolean, as it is not providing the measurement
 * result itself. Instead, it is providing a reference ID that can be used later
 * to retrieve the measurement result if it is available. This allows for queuing up
 * several measurements to be performed at once, then reading the results one by one.
 */
int32_t selene_runtime_measure(RuntimeInstance instance,
                               uint64_t qubit_id,
                               uint64_t *result);

/**
 * Instruct the runtime that a measurement is to be requested with additional
 * capabilities to provide leakage information, and to write a reference ID
 * to the result to the `result` pointer. This result is a u64, packed with
 * appropriate information for the additional capabilities.
 */
int32_t selene_runtime_measure_leaked(RuntimeInstance instance,
                                      uint64_t qubit_id,
                                      uint64_t *result);

/**
 * Instruct the runtime to reset a qubit to the |0> state with the given ID.
 * Note that it is up to the runtime whether or not this reset is applied immediately.
 * It may wait until it is required, e.g. another gate is applied to the qubit.
 */
int32_t selene_runtime_reset(RuntimeInstance instance,
                             uint64_t qubit_id);

/**
 * Instruct the runtime to force a result with the given ID to be made available, e.g.
 * that a measurement must be performed now if it has not been performed yet.
 */
int32_t selene_runtime_force_result(RuntimeInstance instance,
                                    uint64_t result_id);

/**
 * Read a bool result from the runtime.
 */
int32_t selene_runtime_get_bool_result(RuntimeInstance instance,
                                       uint64_t result_id,
                                       int8_t *result);

/**
 * Read a u64 result from the runtime.
 */
int32_t selene_runtime_get_u64_result(RuntimeInstance instance,
                                      uint64_t result_id,
                                      uint64_t *result);

/**
 * If the result relies on a quantum operation such as a measurement, then forcing the
 * result might flush a measurement operation (amongst other things) to Selene for the
 * error model and simulator to act upon. set_bool_result is used by Selene to provide
 * the result back to the runtime for later reporting to the user program.
 */
int32_t selene_runtime_set_bool_result(RuntimeInstance instance,
                                       uint64_t result_id,
                                       bool result);

/**
 * If the result relies on a quantum operation such as a measurement, then forcing the
 * result might flush a measurement operation (amongst other things) to Selene for the
 * error model and simulator to act upon. set_bool_result is used by Selene to provide
 * the result back to the runtime for later reporting to the user program.
 */
int32_t selene_runtime_set_u64_result(RuntimeInstance instance,
                                      uint64_t result_id,
                                      uint64_t result);

/**
 * Increment the reference count of a future ID.
 */
SeleneErrno selene_runtime_increment_future_refcount(RuntimeInstance instance,
                                                     uint64_t future_ref);

/**
 * Decrement the reference count of a future ID. If the reference count reaches
 * 0, nothing should be waiting upon it and it is considered ready to be deallocated.
 */
SeleneErrno selene_runtime_decrement_future_refcount(RuntimeInstance instance,
                                                     uint64_t future_ref);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus
