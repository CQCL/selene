/**
 * This file defines a shim for running Selene with a compiled
 * program for the Helios Quantum Instruction Set.
 *
 * The user's program is expected to have a `qmain` function.
 * This is invoked on each shot, and the calls it makes are
 * routed to the selene library.
 */

#define _USE_MATH_DEFINES
#include <setjmp.h>
#include <math.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#include <selene/selene.h>
#define BUILDING_HELIOS_QIS_INTERFACE
#include <helios_qis/interface.h>

#ifndef SELENE_LOG_LEVEL
#define SELENE_LOG_LEVEL 0
#endif

#define ERROR(...) fprintf(stderr, "[interface] error: " __VA_ARGS__)

#if (SELENE_LOG_LEVEL) == 0
#define INFO(...) printf("[interface] info: " __VA_ARGS__)
#define DEBUG(...)
#define DIAGNOSTIC(...)
#elif (SELENE_LOG_LEVEL) == 1
#define INFO(...) printf("[interface] info: " __VA_ARGS__)
#define DEBUG(...) fprintf(stderr, "[interface] debug: " __VA_ARGS__)
#define DIAGNOSTIC(...)
#elif (SELENE_LOG_LEVEL) == 2
#define INFO(...) printf("[interface] info: " __VA_ARGS__)
#define DEBUG(...) fprintf(stderr, "[interface] debug: " __VA_ARGS__)
#define DIAGNOSTIC(...) fprintf(stderr, "[interface] diagnostic: " __VA_ARGS__)
#else
#error "Invalid SELENE_LOG_LEVEL"
#endif

jmp_buf user_program_jmpbuf;

void panic_impl(int32_t error_code) {
    longjmp(user_program_jmpbuf, error_code);
}

#define unwrap(value) _Generic(value \
    , struct selene_u64_result_t: unwrap_u64 \
    , struct selene_u32_result_t: unwrap_u32 \
    , struct selene_f64_result_t: unwrap_f64 \
    , struct selene_void_result_t: unwrap_void \
    , struct selene_bool_result_t: unwrap_bool \
    , struct selene_future_result_t: unwrap_future \
)(value)

uint64_t unwrap_u64(struct selene_u64_result_t result){
    if (result.error_code != 0) {
        panic_impl(result.error_code);
    }
    return result.value;
}
uint32_t unwrap_u32(struct selene_u32_result_t result){
    if (result.error_code != 0) {
        panic_impl(result.error_code);
    }
    return result.value;
}
double unwrap_f64(struct selene_f64_result_t result){
    if (result.error_code != 0) {
        panic_impl(result.error_code);
    }
    return result.value;
}
void unwrap_void(struct selene_void_result_t result){
    if (result.error_code != 0) {
        panic_impl(result.error_code);
    }
}
bool unwrap_bool(struct selene_bool_result_t result){
    if (result.error_code != 0) {
        panic_impl(result.error_code);
    }
    return result.value;
}
uint64_t unwrap_future(struct selene_future_result_t result){
    if (result.error_code != 0) {
        panic_impl(result.error_code);
    }
    return result.reference;
}
struct selene_string_t parse_cl_string(char const* str) {
    uint8_t length = str[0];
    char const* contents = str + 1;
    return (struct selene_string_t){contents, length, false};
}
struct selene_string_t parse_c_string(char const* str) {
    if (str == 0) {
        str = "NULL";
    }
    return (struct selene_string_t){str, strlen(str), false};
}


SeleneInstance* selene_instance = 0;

// defined by hybrid user program compiler
extern uint64_t qmain(uint64_t);

// The entrypoint of the resulting executable
int main(int argc, char** argv) {
    DIAGNOSTIC("selene_init() with args:\n");
    for (int i = 0; i < argc; ++i) {
        DIAGNOSTIC("   %d: %s\n", i, argv[i]);
    }
    if(argc < 3 || strcmp(argv[1], "--configuration") != 0){
        ERROR("Usage: %s --configuration <configuration_file>\n", argv[0]);
        return 1;
    }
    char const* configuration_file = argv[2];
    struct selene_void_result_t void_result = selene_load_config(&selene_instance, configuration_file);
    if (void_result.error_code != 0) {
        ERROR("Error initializing selene: error code %" PRIu32 "\n", void_result.error_code);
        return void_result.error_code;
    }
    struct selene_u64_result_t n_shots = selene_shot_count(selene_instance);
    if (n_shots.error_code != 0) {
        ERROR("Error fetching shot count from selene: error code %" PRIu32 "\n", void_result.error_code);
        return n_shots.error_code;
    }
    DIAGNOSTIC("Number of shots: %" PRIu64 "\n", n_shots.value);
    bool do_continue = true;
    for(uint64_t current_shot = 0; do_continue && (current_shot < n_shots.value); ++current_shot){
        DIAGNOSTIC("Starting shot %" PRIu64 "\n", current_shot);
        DIAGNOSTIC("----------------------------");
        void_result = selene_on_shot_start(selene_instance, current_shot);
        if (void_result.error_code != 0) {
            fprintf(stderr, "Error starting shot %" PRIu64 ": error code %" PRIu32 "\n", current_shot, void_result.error_code);
            return void_result.error_code;
        }
        int error_code = setjmp(user_program_jmpbuf);
        if (error_code == 0) {
            qmain(0);
        } else {
            // error_code >= 1000 means that the program should stop entirely,
            // error_code < 1000 means the shot stops but the next one is allowed
            do_continue = error_code < 1000;
        }
        void_result = selene_on_shot_end(selene_instance);
        if (void_result.error_code != 0) {
            ERROR("Error ending shot %" PRId64 ": error code %" PRIu32 "\n", current_shot, void_result.error_code);
            return void_result.error_code;
        }
    }
    selene_exit(selene_instance);
}

uint64_t ___qalloc() {
    DIAGNOSTIC("___qalloc()\n");
    uint64_t addr = unwrap(selene_qalloc(selene_instance));
    DIAGNOSTIC("   address: %" PRIu64 "\n", addr);
    return addr;
}
void ___qfree(uint64_t q) {
    DIAGNOSTIC("___qfree(%" PRIu64 ")\n", q);
    unwrap(selene_qfree(selene_instance, q));
    DIAGNOSTIC("   [done]\n");
}
void ___rxy(uint64_t q, double theta, double phi) {
    DIAGNOSTIC("___rxy(%" PRIu64 ", %f, %f)\n", q, theta, phi);
    unwrap(selene_rxy(selene_instance, q, theta, phi));
    DIAGNOSTIC("   [done]\n");
}
void ___rzz(uint64_t q1, uint64_t q2, double theta) {
    DIAGNOSTIC("___rzz(%" PRIu64 ", %" PRIu64 ", %f)\n", q1, q2, theta);
    unwrap(selene_rzz(selene_instance, q1, q2, theta));
    DIAGNOSTIC("   [done]\n");
}
void ___zz(uint64_t q1, uint64_t q2) { // deprecated
    ___rzz(q1, q2, M_PI / 2);
}

void ___rz(uint64_t q, double theta) {
    DIAGNOSTIC("___rz(%" PRIu64 ", %f)\n", q, theta);
    unwrap(selene_rz(selene_instance, q, theta));
    DIAGNOSTIC("   [done]\n");
}
void ___reset(uint64_t q) {
    DIAGNOSTIC("___reset(%" PRIu64 ")\n", q);
    unwrap(selene_qubit_reset(selene_instance, q));
    DIAGNOSTIC("   [done]\n");
}
bool ___measure(uint64_t q) {
    DIAGNOSTIC("___measure(%" PRIu64 ")\n", q);
    bool result = unwrap(selene_qubit_measure(selene_instance, q));
    DIAGNOSTIC("   returned %s\n", result ? "true" : "false");
    return result;
}
uint64_t ___lazy_measure(uint64_t q) {
    DIAGNOSTIC("___lazy_measure(%" PRIu64 ")\n", q);
    uint64_t reference = unwrap(selene_qubit_lazy_measure(selene_instance, q));
    DIAGNOSTIC("   reference: %" PRIu64 "\n", reference);
    return reference;
}
uint64_t ___lazy_measure_leaked(uint64_t q) {
    DIAGNOSTIC("___lazy_measure_leaked(%" PRIu64 ")\n", q);
    uint64_t reference = unwrap(selene_qubit_lazy_measure_leaked(selene_instance, q));
    DIAGNOSTIC("   reference: %" PRIu64 "\n", reference);
    return reference;
}
void ___dec_future_refcount(uint64_t r) {
    DIAGNOSTIC("___dec_future_refcount(%" PRIu64 ")\n", r);
    unwrap(selene_refcount_decrement(selene_instance, r));
    DIAGNOSTIC("   [done]\n");
}
void ___inc_future_refcount(uint64_t r) {
    DIAGNOSTIC("___inc_future_refcount(%" PRIu64 ")\n", r);
    unwrap(selene_refcount_increment(selene_instance, r));
    DIAGNOSTIC("   [done]\n");
}
bool ___read_future_bool(uint64_t r) {
    DIAGNOSTIC("___read_future_bool(%" PRIu64 ")\n", r);
    bool result = unwrap(selene_future_read_bool(selene_instance, r));
    DIAGNOSTIC("   returned %s\n", result ? "true" : "false");
    return result;
}
uint64_t ___read_future_uint(uint64_t r) {
    DIAGNOSTIC("___read_future_uint(%" PRIu64 ")\n", r);
    uint64_t result = unwrap(selene_future_read_u64(selene_instance, r));
    DIAGNOSTIC("   returned %" PRIu64 "\n", result);
    return result;
}

void print_bool(cl_string tag, uint64_t _unused, char value) {
    DIAGNOSTIC("print_bool(\"%.*s\", %02X)\n", tag[0], tag+1, value);
    // Rust bools are specifically 0x00 for false and 0x01 for true,
    // but in HUGR compilation to LLVM, an i1 is passed here. This means
    // that what we receive is not a rust-compatible bool, but 8 bits where
    // only the least significant bit matters. If we accept it as a bool here,
    // the compiler is very good at eliding conversion to 0x00 and 0x01 on the
    // assumption that the value is already a well-formed bool. As such, we accept
    // a char for print_bool and then convert it to a char of the form 0x00 or 0x01.
    char safe_value = (value & 1) == 1 ? 0x01 : 0x00;
    DIAGNOSTIC("   converted to %02X\n", safe_value);
    unwrap(selene_print_bool(selene_instance, parse_cl_string(tag), safe_value));
    DIAGNOSTIC("   [done]\n");
}
void print_int(cl_string tag, uint64_t _unused, int64_t value) {
    DIAGNOSTIC("print_int(\"%.*s\", %" PRId64 ")\n", tag[0], tag+1, value);
    unwrap(selene_print_i64(selene_instance, parse_cl_string(tag), value));
    DIAGNOSTIC("   [done]\n");
}
void print_uint(cl_string tag, uint64_t _unused, uint64_t value) {
    DIAGNOSTIC("print_uint(\"%.*s\", %" PRIu64 ")\n", tag[0], tag+1, value);
    unwrap(selene_print_u64(selene_instance, parse_cl_string(tag), value));
    DIAGNOSTIC("   [done]\n");
}
void print_float(cl_string tag, uint64_t _unused, double value) {
    DIAGNOSTIC("print_float(\"%.*s\", %f)\n", tag[0], tag+1, value);
    unwrap(selene_print_f64(selene_instance, parse_cl_string(tag), value));
    DIAGNOSTIC("   [done]\n");
}
void print_bool_arr(cl_string tag, uint64_t _unused, struct cl_array* arr) {
    uint8_t* array = arr->bytes;
    uint64_t length = arr->x;
    // TODO: check whether the array needs to be copied/modified to convert data into
    // 0x00 and 0x01 based on the last bit of each byte
    DIAGNOSTIC("print_bool_array(\"%.*s\", ptr, %" PRIu64 ")\n", tag[0], tag+1, length);
    for (uint64_t i = 0; i < length; ++i) {
        DIAGNOSTIC("   %" PRIu64 ": %s (%02x)\n", i, array[i] ? "true" : "false", (unsigned char)array[i]);
    }
    unwrap(selene_print_bool_array(selene_instance, parse_cl_string(tag), (bool*)array, length));
    DIAGNOSTIC("   [done]\n");
}
void print_int_arr(cl_string tag, uint64_t _unused, struct cl_array* arr) {
    int64_t* array = arr->i64s;
    uint64_t length = arr->x;
    DIAGNOSTIC("print_int_array(%s, ptr, %" PRId64 ")\n", tag, length);
    for (uint64_t i = 0; i < length; ++i) {
        DIAGNOSTIC("   %" PRIu64 ": %" PRId64 "\n", i, array[i]);
    }
    unwrap(selene_print_i64_array(selene_instance, parse_cl_string(tag), array, length));
    DIAGNOSTIC("   [done]\n");
}
void print_uint_arr(cl_string tag, uint64_t _unused, struct cl_array* arr) {
    uint64_t* array = arr->u64s;
    uint64_t length = arr->x;
    DIAGNOSTIC("print_uint_array(%s, ptr, %" PRIu64 ")\n", tag, length);
    for (uint64_t i = 0; i < length; ++i) {
        DIAGNOSTIC("   %" PRIu64 ": %" PRIu64 "\n", i, array[i]);
    }
    unwrap(selene_print_u64_array(selene_instance, parse_cl_string(tag), array, length));
    DIAGNOSTIC("   [done]\n");
}
void print_float_arr(cl_string tag, uint64_t _unused, struct cl_array* arr) {
    double* array = arr->f64s;
    uint64_t length = arr->x;
    DIAGNOSTIC("print_float_array(%s, ptr, %" PRIu64 ")\n", tag, length);
    for (uint64_t i = 0; i < length; ++i) {
        DIAGNOSTIC("   %" PRIu64 ": %f\n", i, array[i]);
    }
    unwrap(selene_print_f64_array(selene_instance, parse_cl_string(tag), array, length));
    DIAGNOSTIC("   [done]\n");
}

void panic(int32_t error_code, cl_string message) {
    DIAGNOSTIC("panic(%d, %.*s)\n", error_code, message[0], message+1);
    unwrap(selene_print_panic(selene_instance, parse_cl_string(message), error_code));
    DIAGNOSTIC("   Jumping\n");
    longjmp(user_program_jmpbuf, error_code);
}
void panic_str(int32_t error_code, char const* message) {
    DIAGNOSTIC("panic_str(%d, %s)\n", error_code, message);
    unwrap(selene_print_panic(selene_instance, parse_c_string(message), error_code));
    DIAGNOSTIC("   Jumping\n");
    longjmp(user_program_jmpbuf, error_code);
}
void random_seed(uint64_t seed) {
    DIAGNOSTIC("random_seed(%" PRIu64 ")\n", seed);
    unwrap(selene_random_seed(selene_instance, seed));
    DIAGNOSTIC("   [done]\n");
}
uint32_t random_int() {
    DIAGNOSTIC("random_int()\n");
    uint32_t result = unwrap(selene_random_u32(selene_instance));
    DIAGNOSTIC("   result: %" PRIu32 "\n", (uint32_t)result);
    return result;
}
uint32_t random_rng(uint32_t bound) {
    DIAGNOSTIC("random_rng(%d)\n", bound);
    uint32_t result = unwrap(selene_random_u32_bounded(selene_instance, bound));
    DIAGNOSTIC("   result: %" PRIu32 "\n", (uint32_t)result);
    return result;
}
double random_float() {
    DIAGNOSTIC("random_float()\n");
    double result = unwrap(selene_random_f64(selene_instance));
    DIAGNOSTIC("   result: %f\n", result);
    return result;
}
uint64_t get_current_shot() {
    DIAGNOSTIC("get_current_shot()\n");
    uint64_t result = unwrap(selene_get_current_shot(selene_instance));
    DIAGNOSTIC("   result: %" PRIu64 "\n", result);
    return result;
}
void set_tc(uint64_t time_cursor) {
    DIAGNOSTIC("set_tc(%" PRIu64 ")\n", time_cursor);
    unwrap(selene_set_tc(selene_instance, time_cursor));
    DIAGNOSTIC("   [done]\n");
}
uint64_t get_tc() {
    DIAGNOSTIC("get_tc()\n");
    uint64_t result = unwrap(selene_get_tc(selene_instance));
    DIAGNOSTIC("   result: %" PRIu64 "\n", result);
}
void setup(uint64_t time_cursor) {
    DIAGNOSTIC("setup(%" PRIu64 ")\n", time_cursor);
    set_tc(time_cursor);
}
uint64_t teardown() {
    DIAGNOSTIC("teardown()\n");
    return get_tc();
}
void print_state_result(cl_string tag, uint64_t _unused, struct cl_array* qubits) {
    uint64_t* qubits_ptr = qubits->u64s;
    uint64_t qubits_length = qubits->x;
    DIAGNOSTIC("print_state(\"%.*s\", %" PRIu64 ")\n", tag[0], tag+1, qubits_length);
    DIAGNOSTIC("Qubits:\n");
    for (uint64_t i = 0; i < qubits_length; ++i) {
        DIAGNOSTIC("   %" PRIu64 ": %" PRIu64 "\n", i, qubits_ptr[i]);
    }
    unwrap(selene_dump_state(selene_instance, parse_cl_string(tag), qubits_ptr, qubits_length));
    DIAGNOSTIC("   [done]\n");
}
void ___sleep(uint64_t* qubits, uint64_t qubits_len, uint64_t sleep_time) {
    DIAGNOSTIC("___sleep(%p, %" PRIu64 ", %" PRIu64 ")\n", qubits, qubits_len, sleep_time);
    for (uint64_t i = 0; i < qubits_len; ++i) {
        DIAGNOSTIC("   %" PRIu64 ": %" PRIu64 "\n", i, qubits[i]);
    }
    unwrap(selene_local_barrier(selene_instance, qubits, qubits_len, sleep_time));
    DIAGNOSTIC("   [done]\n");
}
void* heap_alloc(size_t size) {
    DIAGNOSTIC("heap_alloc(%zu)\n", size);
    // Later we might want to make a classical runtime plugin
    // and route heap usage to that via selene. However, for
    // now we just use malloc.
    void* ptr = malloc(size);
    DIAGNOSTIC("   allocated %p\n", ptr);
    return ptr;
}
void heap_free(void* ptr) {
    DIAGNOSTIC("heap_free(%p)\n", ptr);
    // Again, we just use free for now.
    free(ptr);
    DIAGNOSTIC("   [done]\n");
}
uint64_t custom_runtime_call(uint64_t tag, void* data, uint64_t data_len) {
    DIAGNOSTIC("custom_runtime_call(%" PRIu64 ", %p, %" PRIu64 ")\n", tag, data, data_len);
    DIAGNOSTIC("Data:\n");
    for (uint64_t i = 0; i < data_len; ++i) {
        DIAGNOSTIC("   %02X ", ((uint8_t*)data)[i]);
    }
    unwrap(selene_custom_runtime_call(selene_instance, tag, data, data_len));
}
