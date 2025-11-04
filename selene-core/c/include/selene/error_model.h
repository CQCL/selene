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
 * Selene maintains its own API version for the error model
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
uint64_t selene_error_model_get_api_version(void);

/**
 * When Selene is initialised, it is provided with some default arguments
 * (the maximum number of qubits, the path to a simulator plugin to use, etc)
 * and some custom arguments for the error model and simulator. These arguments
 * are provided to the error model through this initialization function, with
 * the custom arguments passed in in an argc, argv format.
 *
 * It is this function's responsibility to parse and validate those user-provided
 * arguments and initialise a plugin instance ready for a call to
 * selene_error_model_shot_start(). The `instance` pointer is designed to be set
 * by this function and hold all relevant state, such that subsequent calls to the
 * corresponding instance will be able to access that state, and such that calls to
 * other instances will not be impacted.
 *
 * Error model plugins should provide customisation of parameter values
 * within their python implementations. They should also define how those
 * parameters are converted to an argv list to be passed to their compiled
 * counterparts.
 */
SeleneErrno selene_error_model_init(SeleneErrorModelInstance *instance,
                                    uint64_t n_qubits,
                                    uint32_t error_model_argc,
                                    const char *const *error_model_argv,
                                    const char *simulator_plugin,
                                    uint32_t simulator_argc,
                                    const char *const *simulator_argv);

/**
 * This function is called when Selene is exiting, and it is responsible for
 * cleaning up any resources that the error model plugin has allocated.
 */
SeleneErrno selene_error_model_exit(SeleneErrorModelInstance instance);

/**
 * This function is called at the start of a shot, and it is responsible for
 * initialising the error model plugin for that shot. The error_model_seed is
 * provided for RNG seeding, and it is highly recommended that all randomness
 * used by the error model plugin is seeded with this value.
 *
 * As the error model currently owns the simulator, the simulator_seed is also
 * provided to allow the error model to seed the simulator's RNG. This should
 * result in a call to the simulator's shot_start function.
 */
SeleneErrno selene_error_model_shot_start(SeleneErrorModelInstance instance,
                                          uint64_t shot_id,
                                          uint64_t error_model_seed,
                                          uint64_t simulator_seed);

/**
 * This function is called at the end of a shot, and it is responsible for
 * finalising the error model plugin for that shot. For example, it may
 * clean up any intra-shot state, such as accumulators or buffers. A call to
 * this function will usually be followed either by a call to
 * `selene_error_model_shot_start` to prepare for the following shot, or by
 * a call to `selene_error_model_exit` to shut down the instance.
 */
SeleneErrno selene_error_model_shot_end(SeleneErrorModelInstance instance);

/**
 * This function is called to handle a batch of operations extracted from the
 * runtime. It is responsible for processing the operations and returning the
 * results of any measurements provided in the operation list.
 *
 * As a batch of operations takes the form of a container, we provide extraction
 * through a RuntimeExtractOperationInterface (essentially a list of function
 * pointers) and a corresponding RuntimeExtractOperationInstance that provides
 * the underlying state. See the documentation for [RuntimeExtractOperationInterface]
 * and [RuntimeExtractOperationInstance] for more details.
 *
 * Likewise, as the results of the operations are also in the form of a container,
 * we provide an ErrorModelSetResultInterface that allows the error model to set
 * the results of the measurements in the runtime.
 */
SeleneErrno selene_error_model_handle_operations(SeleneErrorModelInstance instance,
                                                 SeleneRuntimeExtractOperationInstance extract_ops_instance,
                                                 const SeleneRuntimeExtractOperationInterface *extract_ops_interface,
                                                 SeleneErrorModelSetResultInstance result_instance,
                                                 const SeleneErrorModelSetResultInterface *result_interface);

/**
 * This function is called to dump the current state of the simulator to a file.
 * This is niche functionality and any error model that 'wraps' the simulator state
 * in a non-trivial manner (such that the underlying simulator state is not reflective
 * of the state itself, e.g. in the case of leakage) should return an error from this
 * function, as the simulator state is not meaningful in that case.
 */
SeleneErrno selene_error_model_dump_simulator_state(SeleneErrorModelInstance instance,
                                                    const char *filename,
                                                    const uint64_t *qubits,
                                                    uint64_t qubits_length);

/**
 * This is a passthrough function to the simulator's get_metric function. The
 * error model should invoke the simulator's metric function directly unless it
 * has reason to modify the output in some way.
 */
SeleneErrno selene_error_model_get_simulator_metrics(SeleneErrorModelInstance instance,
                                                     uint8_t nth_metric,
                                                     char *tag_ptr,
                                                     uint8_t *datatype_ptr,
                                                     uint64_t *data_ptr);

/**
 * This function is called to get a metric from the error model. When the time comes
 * to gather metrics, Selene will call this function with `nth_metric` set to 0, then
 * 1, then 2, and so on until it returns a nonzero value indicating the end of the
 * available metrics, or until the nth_metric is 255.
 *
 * Three parameters are provided for writing metric information. The first is a
 * pointer to a 255-character buffer used for writing the tag of the metric.
 * The second is a pointer to a u8 that will be set to the data type of the metric,
 * with:
 * - 0 for boolean
 * - 1 for i64
 * - 2 for u64
 * - 3 for f64
 * The third is a pointer to a u64 that will be set to the value of the metric, and
 * should be interpreted as the datatype indicated by the second parameter.
 *
 * For example:
 * ```c
 * void selene_error_model_get_simulator_metrics(
 *     SomeInstance* instance,
 *     uint8_t nth_metric,
 *     char* tag,
 *     u8* datatype,
 *     u64* data,
 * ) {
 *     if (nth_metric == 0) {
 *        strcpy(tag, "number_of_bitflips");
 *        *datatype = 2; // u64
 *        *data = instance->number_of_bitflips();
 *        return 0; // metric written, call again for next metric.
 *     } else if (nth_metric == 1) {
 *        strcpy(tag, "has_leaked_qubits");
 *        *datatype = 0; // boolean
 *        *data = instance->has_leaked_qubits();
 *        return 0; // metric written, call again for next metric.
 *     } else {
 *        uint64_t nth_qubit = nth_metric - 2;
 *        if (nth_qubit <= instance->n_qubits()) {
 *           sprintf(tag, "added_phase_qubit_%llu", nth_qubit);
 *           *datatype = 3; // f64
 *           *data = instance->get_added_phase(nth_qubit);
 *           return 0; // metric written, call again for next metric.
 *        }
 *     }
 *     return 1; // no metric written, end of metrics.
 * }
 */
SeleneErrno selene_error_model_get_metrics(SeleneErrorModelInstance instance,
                                           uint8_t nth_metric,
                                           char *tag_ptr,
                                           uint8_t *datatype_ptr,
                                           uint64_t *data_ptr);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus
