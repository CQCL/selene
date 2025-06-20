# Simulator Design

Selene is designed to accept simulators as plugins. These plugins are provided
to selene from outside of selene itself, and thus the interface to them is primarily
designed around extern "C" function calls in order to provide a stable ABI.

This leads to some design decisions that aim to strike a balance between ownership
of resources (by the plugin), while providing selene with the ability to own the
simulator's lifecycle.

## The extern "C" mindset for simulators

Fundamentally, simulators are objects. They have state, and they have methods
that mutate this state.

Of course, C has a somewhat tenuous relationship with object oriented programming.
Rather than the interface one might be used to from C++:

```cpp
class MySimulator{
    QuantumRegister register;
public:
    MySimulator(std::uint64_t n_qubits);
    ~MySimulator();
    void next_shot(std::uint64_t seed);
    void rzz(std::uint64_t q0, std::uint64_t q1, double theta);
    void rxy(std::uint64_t q);
    bool measure(std::uint64_t q);
    void reset(std::uint64_t q);
}
```

or Rust:

```
struct MySimulator {
    register: QuantumRegister,
}
impl MySimulator {
    fn new(n_qubits: u64) -> Self;
    fn next_shot(seed: u64);
    fn rzz(&mut self, q0: u64, q1: u64, theta: f64);
    fn rxy(&mut self, q: u64);
    fn measure(&mut self, q: u64) -> bool;
    fn reset(&mut self, q: u64);
}
```

A C interface in which the type of the internal state is exposed
to the user might look like:

```c
struct MySimulator {
    QuantumRegister register;
};

void MySimulator_create(struct MySimulator** sim, uint64_t n_qubits);
void MySimulator_destroy(struct MySimulator* sim);
void MySimulator_next_shot(struct MySimulator* sim, uint64_t seed);
void MySimulator_rzz(struct MySimulator* sim, uint64_t q0, uint64_t q1, double theta);
void MySimulator_rxy(struct MySimulator* sim, uint64_t q);
bool MySimulator_measure(struct MySimulator* sim, uint64_t q);
void MySimulator_reset(struct MySimulator* sim, uint64_t q);
```

and one that does not expose the internal state might look like:

```c
void MySimulator_create(void** sim, uint64_t n_qubits, uint64_t random_seed);
void MySimulator_destroy(void* sim);
void MySimulator_next_shot(void* sim, uint64_t seed);
void MySimulator_rzz(void* sim, uint64_t q0, uint64_t q1, double theta);
void MySimulator_rxy(void* sim, uint64_t q);
bool MySimulator_measure(void* sim, uint64_t q);
void MySimulator_reset(void* sim, uint64_t q);
```

Note that in the second example, the internal state of the simulator is generic for
the purposes of a user. They cannot know of the type of `MySimulator`, they cannot
easily access internal fields of `MySimulator`, or keep its state on the stack (as
its size is unknown). What they can do, though is keep track of a memory address
through a void pointer:

```c
void* instance_1;
void* instance_2;
MySimulator_create(&instance_1, 10);
MySimulator_create(&instance_2, 10);
MySimulator_next_shot(instance_1, 1234);
MySimulator_next_shot(instance_2, 1235);
MySimulator_rzz(instance_1, 0, 1, 0.123);
MySimulator_rzz(instance_2, 0, 1, 0.456);
MySimulator_destroy(instance_1);
MySimulator_destroy(instance_2);
```

In this case, you are entrusting ownership of the state of the simulator entirely
to the simulator itself, but you still have sufficient information to pick and choose
between different simulator instances. The C implementation could later completely change
the memory contents of its internal state and the implementation of its functions, yet
it would still function with the above code.

## The actual simulator interface

With the above in mind, let's discuss the interface that selene provides for
simulators.

Simulators can be plugged into various interfaces to selene functionality as a
shared object (from a built dynamic library), and they can be provided as a
struct that acts as a function table. In the case of a shared object, libloading
is used to load the shared object and retrieve the function table, and this is
done in [plugin.rs](./plugin.rs). In the case of a function table, this interface
is defined in [inline.rs](./inline.rs).

In both cases, the interface is wrapped in a struct that adheres to the SimulationInterface trait
defined in [./interface.rs](./interface.rs). For the purposes of simulation, a
`GenericSimulationInterface` is provided, which wraps an Arc<dyn SimulationInterface>,
allowing the simulation interface to be shared generically around selene as it sees fit,
and keeping lifetime management simple.

One can define both of these with help from [helper.rs](./helper.rs), which allows
them to use a normal Rust struct and implement the SimulatorHelper trait. A
`export_simulator_plugin!(YourStructName)` macro invocation will provide
`pub extern "C"` functions automatically. For testing purposes, the
`crate_to_inline_simulator!()` macro utilises those same exposed functions to
create an InlineSimulator, which is then usable in the selene-core simulator test framework.

