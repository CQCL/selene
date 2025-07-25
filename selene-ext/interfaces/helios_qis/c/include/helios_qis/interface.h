#ifndef HELIOS_QIS_INTERFACE_H
#define HELIOS_QIS_INTERFACE_H

#include <stdint.h>
#include <stdbool.h>
#include <inttypes.h>
#include <helios_qis/cl_types.h>

// when building, we mark the functions as exported on windows,
// and as used on other platforms.
#ifdef BUILDING_HELIOS_QIS_INTERFACE
#ifdef _WIN32
#define EXPORT __declspec(dllexport)
#else
#define EXPORT __attribute__((used, visibility("default")))
#endif
// When consuming, we mark the functions as imported on
// windows. No special marking is needed on other platforms.
#else
#define EXPORT __declspec(dllimport)
#endif



// The entrypoint of the resulting executable
EXPORT int main(int argc, char** argv);

EXPORT uint64_t ___qalloc();
EXPORT void ___qfree(uint64_t q);
EXPORT void ___rxy(uint64_t q, double theta, double phi);
EXPORT void ___rzz(uint64_t q1, uint64_t q2, double theta);
EXPORT void ___rz(uint64_t q, double theta);
EXPORT void ___reset(uint64_t q);
EXPORT bool ___measure(uint64_t q);
EXPORT uint64_t ___lazy_measure(uint64_t q);
EXPORT uint64_t ___lazy_measure_leaked(uint64_t q);
EXPORT void ___dec_future_refcount(uint64_t r);
EXPORT void ___inc_future_refcount(uint64_t r);
EXPORT bool ___read_future_bool(uint64_t r);
EXPORT uint64_t ___read_future_uint(uint64_t r);
EXPORT void print_bool(cl_string tag, uint64_t _unused, char value);
EXPORT void print_int(cl_string tag, uint64_t _unused, int64_t value);
EXPORT void print_uint(cl_string tag, uint64_t _unused, uint64_t value);
EXPORT void print_float(cl_string tag, uint64_t _unused, double value);
EXPORT void print_bool_arr(cl_string tag, uint64_t _unused, struct cl_array* arr);
EXPORT void print_int_arr(cl_string tag, uint64_t _unused, struct cl_array* arr);
EXPORT void print_uint_arr(cl_string tag, uint64_t _unused, struct cl_array* arr);
EXPORT void print_float_arr(cl_string tag, uint64_t _unused, struct cl_array* arr);
EXPORT void print_state_result(cl_string tag, uint64_t unused, struct cl_array* qubits);
EXPORT void panic(int32_t error_code, cl_string message);
EXPORT void panic_str(int32_t error_code, char const* message);
EXPORT void random_seed(uint64_t seed);
EXPORT uint32_t random_int();
EXPORT uint32_t random_rng(uint32_t bound);
EXPORT double random_float();
EXPORT uint64_t get_current_shot();
EXPORT void set_tc(uint64_t time_cursor);
EXPORT uint64_t get_tc();
EXPORT void setup(uint64_t time_cursor);
EXPORT uint64_t teardown();
EXPORT void* heap_alloc(size_t size);
EXPORT void heap_free(void* ptr);

#endif // HELIOS_QIS_INTERFACE_H
