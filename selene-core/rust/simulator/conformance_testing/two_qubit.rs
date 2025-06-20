use super::framework::TestFramework;
use crate::simulator::interface::SimulatorInterfaceFactory;
use std::sync::Arc;

fn basic_entanglement(interface: Arc<impl SimulatorInterfaceFactory + 'static>, args: Vec<String>) {
    /*!
    ```ignore

          ╭─ assert measurement
          │  ratio is (1:0:0:0)
          │   ╭───╮
     |0⟩──┴───┤ H ├──●─────────┬──---
              ╰───╯  │         ╰───╮
                     │       assert equal
                     │         ╭───╯
     |0⟩─────────────⊕─────────┴──---

    ```
    */
    TestFramework::new(2)
        .test(100, vec![0, 1], |populations| {
            populations.test_measurements[0b00] == 100
        })
        .h(0)
        .cnot(0, 1)
        .test(100, vec![0, 1], |populations| {
            populations.test_measurements[0b00] > 0
                && populations.test_measurements[0b11] > 0
                && populations.test_measurements[0b01] == 0
                && populations.test_measurements[0b10] == 0
        })
        .run(interface, args);
}

pub fn two_qubit_operations(
    interface: Arc<impl SimulatorInterfaceFactory + 'static>,
    args: Vec<String>,
) {
    basic_entanglement(interface, args);
}
