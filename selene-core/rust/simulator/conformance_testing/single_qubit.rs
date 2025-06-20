use super::framework::TestFramework;
use crate::simulator::SimulatorInterfaceFactory;
use std::sync::Arc;

fn single_qubit_collapse(
    interface: Arc<impl SimulatorInterfaceFactory + 'static>,
    args: Vec<String>,
) {
    /*!
       ```ignore


          ╭─ assert measurement            ╭─ assert measurement
          │  ratio is (1:0)                │  is consistent
          │     ╭───╮         ╔═════════╗  │
     |0⟩──┴─────┤ H ├──┬──────║ Measure ║──┴───-----
                ╰───╯  │      ╚═════════╝
                       ╰─ assert measurement
                          ratio is ~ (1:1)
        ```
    !*/
    TestFramework::new(1)
        .test(
            100,     /* iterations */
            vec![0], /* qubits to measure */
            |populations| {
                // We expect 100% of measurements to be |0⟩
                populations.test_measurements[0] == 100 && populations.test_measurements[1] == 0
            },
        )
        .h(0)
        .test(
            100,     /* iterations */
            vec![0], /* qubits to measure */
            |populations| {
                // We expect a 50/50 distribution of |0⟩ and |1⟩.
                // It won't be exact, so demand at least 10 of each.
                populations.test_measurements[0] > 10 && populations.test_measurements[1] > 10
            },
        )
        .measure(0)
        .test(
            100,     /* iterations */
            vec![0], /* qubits to measure */
            |populations| {
                // We expect 100% of measurements to be |0⟩,
                //        or 100% of measurements to be |1⟩.
                // We should see no mixing.
                (populations.test_measurements[0] == populations.circuit_measurements[0])
                    && (populations.test_measurements[1] == populations.circuit_measurements[1])
            },
        )
        .run(interface, args);
}
fn single_qubit_hzh(interface: Arc<impl SimulatorInterfaceFactory + 'static>, args: Vec<String>) {
    /*!
      ```ignore

          ╭─ assert measurement        ╭─ assert measurement
          │  ratio is (1:0)            │  ratio is (0:1)
          │   ╭───╮     ╭───╮    ╭───╮ │
     |0⟩──┴───┤ H ├──┬──┤ Z ├────┤ H ├─┴───---
              ╰───╯  │  ╰───╯    ╰───╯
                     │
                     ╰─ assert measurement
                         ratio is ~ (1:1)

    ```

    !*/

    TestFramework::new(1)
        .test(
            100,     /* iterations */
            vec![0], /* qubits to measure */
            |populations| {
                // We expect 100% of measurements to be |0⟩
                populations.test_measurements[0] == 100 && populations.test_measurements[1] == 0
            },
        )
        .h(0)
        .test(
            100,     /* iterations */
            vec![0], /* qubits to measure */
            |populations| {
                // We expect a 50/50 distribution of |0⟩ and |1⟩.
                // It won't be exact, so demand at least 10 of each.
                populations.test_measurements[0] > 10 && populations.test_measurements[1] > 10
            },
        )
        .z(0)
        .h(0)
        .test(
            100,     /* iterations */
            vec![0], /* qubits to measure */
            |populations| {
                // We expect 100% of measurements to be |0⟩
                populations.test_measurements[0] == 0 && populations.test_measurements[1] == 100
            },
        )
        .run(interface, args);
}

pub fn single_qubit_operations(
    interface: Arc<impl SimulatorInterfaceFactory + 'static>,
    args: Vec<String>,
) {
    single_qubit_collapse(interface.clone(), args.clone());
    single_qubit_hzh(interface.clone(), args.clone());
}
