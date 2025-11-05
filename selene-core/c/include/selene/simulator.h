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
 * Selene maintains its own API version for the simulator
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
uint64_t selene_simulator_get_api_version(void);

/**
 * When Selene is initialised, it is provided with some default arguments
 * (the maximum number of qubits, the path to a simulator plugin to use, etc)
 * and some custom arguments for the simulator. These arguments are provided
 * to the simulator through this initialization function, with
 * the custom arguments passed in in an argc, argv format.
 *
 * It is this function's responsibility to parse and validate those user-provided
 * arguments and initialise a plugin instance ready for a call to
 * selene_simulator_shot_start(). The `instance` pointer is designed to be set
 * by this function and hold all relevant state, such that subsequent calls to the
 * corresponding instance will be able to access that state, and such that calls to
 * other instances will not be impacted.
 *
 * Simulator plugins should provide customisation of parameter values
 * within their python implementations. They should also define how those
 * parameters are converted to an argv list to be passed to their compiled
 * counterparts.
 */
int32_t selene_simulator_init(SeleneSimulatorInstance *instance,
                              uint64_t n_qubits,
                              uint32_t argc,
                              const char *const *argv);

/**
 * This function is called when Selene is exiting, and it is responsible for
 * cleaning up any resources that the simulator plugin has allocated.
 */
int32_t selene_simulator_exit(SeleneSimulatorInstance instance);

/**
 * This function is called at the start of a shot, and it is responsible for
 * initialising the simulator plugin for that shot. The seed is provided for
 * RNG seeding, and it is highly recommended that all randomness used by the
 * simulator is seeded with this value.
 */
int32_t selene_simulator_shot_start(SeleneSimulatorInstance instance,
                                    uint64_t shot_id,
                                    uint64_t seed);

/**
 * This function is called at the end of a shot, and it is responsible for
 * finalising the simulator plugin for that shot. For example, it may
 * clean up any state, such as accumulators or buffers, or set a state vector
 * to zero. We recommenA call to
 * this function will usually be followed either by a call to
 * `selene_simulator_shot_start` to prepare for the following shot, or by
 * a call to `selene_simulator_exit` to shut down the instance.
 */
int32_t selene_simulator_shot_end(SeleneSimulatorInstance instance,
                                  uint64_t seed);

/**
 * Apply an RXY gate to the qubit at the requested index, with the provided
 * angles. This gate is also commonly known as the PhasedX gate or the R1XY gate,
 * performing:
 * $R_z(\phi)R_x(\theta)R_z(-\phi)$
 * (in matrix-multiplication order).
 */
int32_t selene_simulator_operation_rxy(SeleneSimulatorInstance instance,
                                       uint64_t qubit,
                                       double theta,
                                       double phi);

/**
 * Apply an RZZ gate to the qubits at the requested indices, with the provided
 * angles. This gate is also commonly known as the ZZPhase gate or the R2ZZ gate,
 * performing
 * $diag(\chi^*, \chi, \chi, \chi^*)$
 * where
 * $\chi = \exp(i \pi \theta / 2)$
 */
int32_t selene_simulator_operation_rzz(SeleneSimulatorInstance instance,
                                       uint64_t qubit1,
                                       uint64_t qubit2,
                                       double theta);

/**
 * Apply an RZ gate to the qubit at the requested index, with the
 * provided angle. This should perform:
 * $diag(\chi^*, \chi)$
 * where
 * $chi = \exp(i \pi \theta / 2)$
 */
int32_t selene_simulator_operation_rz(SeleneSimulatorInstance instance,
                                      uint64_t qubit,
                                      double theta);

/**
 * Measure the qubit at the requested index. This is a destructive
 * operation.
 */
int32_t selene_simulator_operation_measure(SeleneSimulatorInstance instance,
                                           uint64_t qubit);

/**
 * Postselect the qubit at the requested index. Some simulators may
 * choose to not support post-selection, in which case this function
 * should return an error.
 */
int32_t selene_simulator_operation_postselect(SeleneSimulatorInstance instance,
                                              uint64_t qubit,
                                              bool target_value);

/**
 * Reset the qubit at the requested index to the |0> state.
 */
int32_t selene_simulator_operation_reset(SeleneSimulatorInstance instance,
                                         uint64_t qubit);

/**
 * Get a metric from the simulator instance.
 *
 * nth_metric is the index of the metric to retrieve, starting from 0,
 * and called with incrementing nth_metric until a nonzero value is
 * returned. The tag_ptr, datatype_ptr and data_ptr are out-parameters
 * for the plugin to write values into. See the example in the
 * error_model documentation for details on how to write those values.
 */
int32_t selene_simulator_get_metrics(SeleneSimulatorInstance instance,
                                     uint8_t nth_metric,
                                     char *tag_ptr,
                                     uint8_t *datatype_ptr,
                                     uint64_t *data_ptr);

/**
 * Dump the internal state of the simulator to a file. A list of
 * qubits is provided which corresponds to the ordering provided
 * by the user, and the simulator should represent these in the file
 * in a way that is parsable by the python component of the simulator,
 * as the addresses of those qubits are only known at runtime.
 *
 * Simulators can return an error if they do not support this
 * functionality, or if the file cannot be written to.
 *
 * The python component of the simulator should provide functionality
 * for reading the resulting file. The filename will be written to
 * the result stream.
 */
int32_t selene_simulator_dump_state(SeleneSimulatorInstance instance,
                                    const char *file,
                                    const uint64_t *qubits,
                                    uint64_t n_qubits);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus
