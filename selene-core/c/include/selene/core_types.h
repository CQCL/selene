/* Generated with cbindgen:0.28.0 */

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>


typedef struct SeleneErrorModelAPIVersion {
  /**
   * Reserved for future use, must be 0.
   */
  uint8_t reserved;
  /**
   * Major version of the API.
   */
  uint8_t major;
  /**
   * Minor version of the API.
   */
  uint8_t minor;
  /**
   * Patch version of the API.
   */
  uint8_t patch;
} SeleneErrorModelAPIVersion;

typedef void *SeleneErrorModelInstance;

/**
 * An instance is provided to `selene_runtime_get_next_operations`, which must
 * pass that back to any function it calls in it's provided
 * [ErrorModelSetResultInterface].
 */
typedef void *SeleneErrorModelSetResultInstance;

/**
 * A plugin's implementation of `selene_runtime_get_next_operations` is provided
 * a pointer to a `ErrorModelSetResultInterface` as well as a
 * [ErrorModelSetResultInstance]. It should call the functions
 * within to populate a batch. All such calls must pass the instance as the
 * first parameter.
 */
typedef struct SeleneErrorModelSetResultInterface {
  void (*set_bool_result_fn)(SeleneErrorModelSetResultInstance,
                             uint64_t,
                             bool);
  void (*set_u64_result_fn)(SeleneErrorModelSetResultInstance,
                            uint64_t,
                            uint64_t);
} SeleneErrorModelSetResultInterface;

typedef struct SeleneSimulatorAPIVersion {
  /**
   * Reserved for future use, must be 0.
   */
  uint8_t reserved;
  /**
   * Major version of the API.
   */
  uint8_t major;
  /**
   * Minor version of the API.
   */
  uint8_t minor;
  /**
   * Patch version of the API.
   */
  uint8_t patch;
} SeleneSimulatorAPIVersion;

typedef void *SeleneSimulatorInstance;

typedef struct SeleneRuntimeAPIVersion {
  /**
   * Reserved for future use, must be 0.
   */
  uint8_t reserved;
  /**
   * Major version of the API.
   */
  uint8_t major;
  /**
   * Minor version of the API.
   */
  uint8_t minor;
  /**
   * Patch version of the API.
   */
  uint8_t patch;
} SeleneRuntimeAPIVersion;

/**
 * An instance is provided to `selene_runtime_get_next_operations`, which must
 * pass that back to any function it calls in it's provided
 * [RuntimeGetOperationInterface].
 */
typedef void *SeleneRuntimeGetOperationInstance;

/**
 * A plugin's implementation of `selene_runtime_get_next_operations` is provided
 * a pointer to a `RuntimeGetOperationInterface` as well as a
 * [RuntimeGetOperationInstance]. It should call the functions
 * within to populate a batch. All such calls must pass the instance as the
 * first parameter.
 */
typedef struct SeleneRuntimeGetOperationInterface {
  void (*rzz_fn)(SeleneRuntimeGetOperationInstance,
                 uint64_t,
                 uint64_t,
                 double);
  void (*rxy_fn)(SeleneRuntimeGetOperationInstance,
                 uint64_t,
                 double,
                 double);
  void (*rz_fn)(SeleneRuntimeGetOperationInstance,
                uint64_t,
                double);
  void (*measure_fn)(SeleneRuntimeGetOperationInstance,
                     uint64_t,
                     uint64_t);
  void (*measure_leaked_fn)(SeleneRuntimeGetOperationInstance,
                            uint64_t,
                            uint64_t);
  void (*reset_fn)(SeleneRuntimeGetOperationInstance,
                   uint64_t);
  void (*custom_fn)(SeleneRuntimeGetOperationInstance,
                    size_t,
                    const void*,
                    size_t);
  void (*set_batch_time_fn)(SeleneRuntimeGetOperationInstance,
                            uint64_t,
                            uint64_t);
} SeleneRuntimeGetOperationInterface;

typedef void *SeleneRuntimeExtractOperationInstance;

typedef struct SeleneRuntimeExtractOperationInterface {
  void (*extract_fn)(SeleneRuntimeExtractOperationInstance,
                     SeleneRuntimeGetOperationInstance,
                     struct SeleneRuntimeGetOperationInterface);
} SeleneRuntimeExtractOperationInterface;

typedef int32_t SeleneErrno;
