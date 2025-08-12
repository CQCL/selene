#include "wrapper.h"
#include "stim/circuit/gate_target.h"

extern "C" {
    void * cstim_TableauSimulator64_create(unsigned int numQubits,unsigned long long randomSeed) {
        stim::TableauSimulator<64>* obj = new stim::TableauSimulator<64>(std::mt19937_64{randomSeed},numQubits);
        return (void *) obj;
    }
    void cstim_TableauSimulator64_destroy(void * rawptr) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        delete obj;
    }

    void cstim_TableauSimulator64_do_SQRT_X(void * rawptr,unsigned int q) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        obj->inv_state.prepend_SQRT_X(q);
    }

    void cstim_TableauSimulator64_do_SQRT_Z(void * rawptr,unsigned int q) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        obj->inv_state.prepend_SQRT_Z(q);
    }

    void cstim_TableauSimulator64_do_SQRT_X_DAG(void * rawptr,unsigned int q) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        obj->inv_state.prepend_SQRT_X_DAG(q);
    }

    void cstim_TableauSimulator64_do_SQRT_Z_DAG(void * rawptr,unsigned int q) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        obj->inv_state.prepend_SQRT_Z_DAG(q);
    }

    void cstim_TableauSimulator64_do_SQRT_ZZ(void * rawptr,unsigned int q0, unsigned int q1) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        obj->inv_state.prepend_SQRT_ZZ(q0,q1);
    }

    void cstim_TableauSimulator64_do_SQRT_ZZ_DAG(void * rawptr,unsigned int q0, unsigned int q1) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        obj->inv_state.prepend_SQRT_ZZ_DAG(q0,q1);
    }

    void cstim_TableauSimulator64_do_X(void * rawptr,unsigned int q) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        obj->inv_state.prepend_X(q);
    }

    void cstim_TableauSimulator64_do_Z(void * rawptr,unsigned int q) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        obj->inv_state.prepend_Z(q);
    }

    void cstim_TableauSimulator64_do_H_XZ(void * rawptr,unsigned int q) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        obj->inv_state.prepend_H_XZ(q);
    }

    bool cstim_TableauSimulator64_do_MZ(void * rawptr,unsigned int q) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        if (!obj->is_deterministic_z(q)) {
            stim::TableauTransposedRaii<64> temp_transposed(obj->inv_state);
            obj->collapse_qubit_z(q, temp_transposed);
        }
        return obj->inv_state.zs.signs[q];
    }

    bool cstim_TableauSimulator64_do_POSTSELECT_Z(void * rawptr,unsigned int q, bool target_result) {
        stim::TableauSimulator<64>* obj = (stim::TableauSimulator<64>*) rawptr;
        try {
            std::array<stim::GateTarget, 1> targets = {stim::GateTarget::qubit(q)};
            obj->postselect_z(targets, target_result);
            return true;
        } catch (const std::invalid_argument& e) {
            std::cerr << "Error: " << e.what() << std::endl;
            return false;
        }
    }
}
