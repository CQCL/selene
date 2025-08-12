#include "stim.h"

extern "C" {
    void * cstim_TableauSimulator64_create(unsigned int numQubits, unsigned long long randomSeed);
    void cstim_TableauSimulator64_destroy(void * rawptr);
    void cstim_TableauSimulator64_do_SQRT_X(void * rawptr,unsigned int q);
    void cstim_TableauSimulator64_do_SQRT_Z(void * rawptr,unsigned int q);
    void cstim_TableauSimulator64_do_SQRT_X_DAG(void * rawptr,unsigned int q);
    void cstim_TableauSimulator64_do_SQRT_Z_DAG(void * rawptr,unsigned int q);
    void cstim_TableauSimulator64_do_SQRT_ZZ(void * rawptr,unsigned int q0, unsigned int q1);
    void cstim_TableauSimulator64_do_SQRT_ZZ_DAG(void * rawptr,unsigned int q0, unsigned int q1);
    void cstim_TableauSimulator64_do_X(void * rawptr,unsigned int q);
    void cstim_TableauSimulator64_do_Z(void * rawptr,unsigned int q);
    void cstim_TableauSimulator64_do_H_XZ(void * rawptr,unsigned int q);
    bool cstim_TableauSimulator64_do_MZ(void * rawptr,unsigned int q);
    bool cstim_TableauSimulator64_do_POSTSELECT_Z(void * rawptr,unsigned int q, bool target_result);
}
