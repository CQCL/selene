/*!
This module provides a test framework for selene simulators.
One can specify a test circuit in a fluent manner, with the
added ability to insert tests within the sequence.

Each test provides a number of shots, a list of qubits to measure,
and a test function that validates measurement outcomes. Tests
do not impact subsequent operations, so one can test the state
of the simulator at any point in the circuit.

This works by generating a new simulator for each test shot, running
the circuit up until the test, and then measuring and invoking the
test function. Once this test is complete, it won't be invoked for
subsequent tests.

Consider the following example:

```ignore

TestFramework::new(4 /* number of qubits */)
    .h(0)
    .cx(0, 1)
        .test(
            100,      /* iterations */
            vec![0,1],  /* qubits to measure */
            |outcomes| {
                /* Outcomes is a vector, indexed by
                 * the measurement result. Return true
                 * to pass the test */
                // We expect the state to be |00⟩ + |11⟩

                if outcomes[0b01] + outcomes[0b10] > 0 {
                    // uh oh, we have some |01⟩ or |10⟩ states!
                    return false;
                }

                // We expect a 50/50 distribution of |00⟩ and |11⟩.
                // We can't guarantee an exact distribution, so we'll
                // accept at least 10 of each
                outcomes[0b00] < 10 && outcomes[0b11] > 10
            }
        )
    .cx(0, 2)
    .cx(0, 3)
        .test(
            200 /* iterations */,
            vec![0,1,2,3]  /* qubits to measure */,
            |outcomes| {
                // We expect the state to be |0000⟩ + |1111⟩
                for i in 0b0001..0b1111 {
                    if outcomes[i] > 0 {
                        return false;
                    }
                }
                outcomes[0b0000] > 20 && outcomes[0b1111] > 20
            }
        )
    .run(simulator_generator);
```

In this example, this is what will happen:

# Run test 1
- create a vector V of 2^2 elements, all initialized to 0
- for i in 0..100:
  -  create a new 4-qubit simulator
  -  run h(0)
  -  run cx(0, 1)
  -  measure qubits 0 and 1, concatenated as an int e.g. 0b00, 0b01, 0b10 or 0b11
  -  increment the element in V corresponding to the measurement result
- run the first test function against V

# Run test 2
- create a vector V' of 2^4 elements, all initialized to 0
- for i in 0..200:
  - create a new 4-qubit simulator
  - run h(0)
  - run cx(0, 1)
  - run cx(0, 2)
  - run cx(0, 3)
  - measure all qubits, concatenated as an int, i.e. one of 0b0000, 0b0001, ..., 0b1111
  - increment the element in V' corresponding to the measurement result
- run the second test function against V'

If any operations come *after* the last test, they will never be invoked.

*/

use crate::simulator::{Simulator, SimulatorInterface, SimulatorInterfaceFactory};
use std::sync::Arc;

const PI: f64 = std::f64::consts::PI;
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
pub const QUARTER_PI: f64 = std::f64::consts::FRAC_PI_4;

enum Operation {
    Rz(u64, f64),
    Rxy(u64, f64, f64),
    Rzz(u64, u64, f64),
    Reset(u64),
    Measure(u64),
}

struct EngineState {
    simulator: Simulator,
    qubit_phases: Vec<f64>,
    circuit_measurements: usize,
    n_measurements: u64,
}
impl EngineState {
    pub fn new(
        simulator_interface: Arc<impl SimulatorInterfaceFactory + 'static>,
        n_qubits: u64,
        shot_id: u64,
        random_seed: u64,
        args: Vec<String>,
    ) -> Self {
        let mut simulator = Simulator::new(simulator_interface, n_qubits, &args).unwrap();
        simulator.shot_start(shot_id, random_seed).unwrap();
        Self {
            simulator,
            qubit_phases: vec![0.0; n_qubits as usize],
            circuit_measurements: 0,
            n_measurements: 0,
        }
    }
    pub fn run_operation(&mut self, operation: &Operation) {
        match operation {
            Operation::Rz(q, theta) => {
                self.qubit_phases[*q as usize] += theta;
            }
            Operation::Rxy(q, theta, phi) => {
                self.simulator
                    .rxy(*q, *theta, *phi - self.qubit_phases[*q as usize])
                    .unwrap();
            }
            Operation::Rzz(q0, q1, theta) => {
                self.simulator.rzz(*q0, *q1, *theta).unwrap();
            }
            Operation::Reset(q) => {
                self.simulator.reset(*q).unwrap();
            }
            Operation::Measure(q) => {
                if self.measure(*q) {
                    self.circuit_measurements |= 1 << self.n_measurements;
                }
                self.n_measurements += 1;
            }
        }
    }
    pub fn measure(&mut self, q: u64) -> bool {
        self.simulator.measure(q).unwrap()
    }
}

pub struct PopulationResult {
    /** Represents the result of several shots.
     A shot may contain measurements that are performed
     in the running of the normal circuit, and measurements
     that are running purely due to the test. These are provided
     separately, so that e.g. a collapse test could verify that
     a test measurement always matches an immediately preceding
     circuit measurement.
    **/
    /// The measurements as performed in the main circuit before the
    /// current test, as a binary string.
    pub circuit_measurements: Vec<u64>,
    /// Measurements as performed by the current test
    pub test_measurements: Vec<u64>,
}

impl PopulationResult {
    pub fn new(n_circuit_measurements: u64, n_test_qubits_to_measure: u64) -> Self {
        Self {
            circuit_measurements: vec![0; 2_usize.pow(n_circuit_measurements.try_into().unwrap())],
            test_measurements: vec![0; 2_usize.pow(n_test_qubits_to_measure.try_into().unwrap())],
        }
    }
    pub fn add_circuit_measurements(&mut self, measurements: u64) {
        self.circuit_measurements[measurements as usize] += 1;
    }
    pub fn add_test_measurements(&mut self, measurements: u64) {
        self.test_measurements[measurements as usize] += 1;
    }
}

struct MeasurementTest {
    n_iterations: u64,
    qubits_to_measure: Vec<u64>,
    test: fn(&PopulationResult) -> bool,
}

impl MeasurementTest {
    pub fn new(
        n_iterations: u64,
        qubits_to_measure: Vec<u64>,
        test: fn(&PopulationResult) -> bool,
    ) -> Self {
        Self {
            n_iterations,
            qubits_to_measure,
            test,
        }
    }
    pub fn run(
        &self,
        circuit: &TestFramework,
        interface: Arc<impl SimulatorInterfaceFactory + 'static>,
        args: Vec<String>,
        shot_id: u64,
        stage: usize,
        n_circuit_measurements: u64,
    ) -> Option<PopulationResult> {
        //! Generate `n_iterations` circuits and perform the requested qubit measurements on each,
        //! gathering populations of resulting states in a vector (indexed such that the first
        //! qubit to measure is the LSB). The resulting vector is tested according to the supplied
        //! function.
        //!
        //! If the test fails, return the population result for analysis.
        //! Otherwise, return None.
        let mut result =
            PopulationResult::new(n_circuit_measurements, self.qubits_to_measure.len() as u64);
        for i in 0..self.n_iterations {
            let mut simulator_state =
                circuit.run_to_stage(interface.clone(), args.clone(), shot_id, i, stage);
            result.add_circuit_measurements(simulator_state.circuit_measurements as u64);

            let mut test_measurement: u64 = 0;
            for q in self.qubits_to_measure.iter() {
                if simulator_state.measure(*q) {
                    test_measurement |= 1 << q;
                }
            }

            result.add_test_measurements(test_measurement);
        }
        if (self.test)(&result) {
            None
        } else {
            Some(result)
        }
    }
}

enum CircuitTestOperation {
    Operation(Operation),
    Test(MeasurementTest),
}

pub struct TestFramework {
    n_qubits: u64,
    stages: Vec<CircuitTestOperation>,
}
impl TestFramework {
    pub fn new(n_qubits: u64) -> Self {
        Self {
            n_qubits,
            stages: Vec::new(),
        }
    }
    fn add_operation(&mut self, operation: Operation) {
        self.stages.push(CircuitTestOperation::Operation(operation));
    }

    pub fn rz(&mut self, qubit: u64, theta: f64) -> &mut Self {
        self.add_operation(Operation::Rz(qubit, theta));
        self
    }
    pub fn rxy(&mut self, qubit: u64, theta: f64, phi: f64) -> &mut Self {
        self.add_operation(Operation::Rxy(qubit, theta, phi));
        self
    }
    pub fn rzz(&mut self, qubit1: u64, qubit2: u64, theta: f64) -> &mut Self {
        self.add_operation(Operation::Rzz(qubit1, qubit2, theta));
        self
    }
    pub fn reset(&mut self, qubit: u64) -> &mut Self {
        self.add_operation(Operation::Reset(qubit));
        self
    }
    pub fn measure(&mut self, qubit: u64) -> &mut Self {
        self.add_operation(Operation::Measure(qubit));
        self
    }

    pub fn rx(&mut self, qubit: u64, theta: f64) -> &mut Self {
        self.rxy(qubit, theta, 0.0);
        self
    }
    pub fn ry(&mut self, qubit: u64, theta: f64) -> &mut Self {
        self.rxy(qubit, theta, HALF_PI);
        self
    }
    pub fn x(&mut self, qubit: u64) -> &mut Self {
        self.rx(qubit, PI);
        self
    }
    pub fn y(&mut self, qubit: u64) -> &mut Self {
        self.ry(qubit, PI);
        self
    }
    pub fn z(&mut self, qubit: u64) -> &mut Self {
        self.rz(qubit, PI);
        self
    }

    pub fn h(&mut self, qubit: u64) -> &mut Self {
        self.rxy(qubit, HALF_PI, -HALF_PI);
        self.z(qubit);
        self
    }

    pub fn cnot(&mut self, control: u64, target: u64) -> &mut Self {
        self.rxy(target, HALF_PI, HALF_PI);
        self.rzz(control, target, HALF_PI);
        self.rz(control, HALF_PI);
        self.rxy(target, HALF_PI, 0.0);
        self.rz(target, -HALF_PI);
        self
    }

    pub fn test(
        &mut self,
        n_iterations: u64,
        qubits_to_measure: Vec<u64>,
        test: fn(&PopulationResult) -> bool,
    ) -> &mut Self {
        self.stages
            .push(CircuitTestOperation::Test(MeasurementTest::new(
                n_iterations,
                qubits_to_measure,
                test,
            )));
        self
    }
    fn run_to_stage(
        &self,
        interface: Arc<impl SimulatorInterfaceFactory + 'static>,
        args: Vec<String>,
        shot_id: u64,
        random_seed: u64,
        stage_index: usize,
    ) -> EngineState {
        //! Run the operations up to the specified stage,
        //! returning the state of the simulator at that point.
        //! Tests are skipped, as they are destructive.
        let mut simulator_state =
            EngineState::new(interface, self.n_qubits, shot_id, random_seed, args);

        for stage in 0..stage_index {
            match &self.stages[stage] {
                CircuitTestOperation::Operation(operation) => {
                    simulator_state.run_operation(operation);
                }
                CircuitTestOperation::Test(_) => {}
            }
        }
        simulator_state
    }
    pub fn run(&self, interface: Arc<impl SimulatorInterfaceFactory + 'static>, args: Vec<String>) {
        let mut nth_test = 0;
        let mut n_circuit_measurements = 0;
        for stage in 0..self.stages.len() {
            match &self.stages[stage] {
                CircuitTestOperation::Operation(operation) => {
                    if let Operation::Measure(_) = operation {
                        n_circuit_measurements += 1;
                    }
                }
                CircuitTestOperation::Test(test) => {
                    nth_test += 1;
                    if let Some(measurements) = test.run(
                        self,
                        interface.clone(),
                        args.clone(),
                        0, // shot_id - TODO is this a good assumption?
                        stage,
                        n_circuit_measurements,
                    ) {
                        panic!(
                            "Test #{nth_test} failed. Prior circuit measurements: {:?}, test measurements: {:?}",
                            measurements.circuit_measurements, measurements.test_measurements
                        );
                    }
                }
            }
        }
    }
}
