#ifndef CL_TYPES_H
#define CL_TYPES_H

#include <stdint.h>

typedef struct cl_array {
    uint32_t x;
    uint32_t y;
    union {
        uint64_t* u64s;
        int64_t*  i64s; 
        double*   f64s;
        uint8_t*  bytes;
    };
} cl_array;

typedef char* cl_string;

#endif // CL_TYPES_H
