# Selene Developer Documentation

## Design Rationale

Selene is an emulator for hybrid quantum programs. This is a broad definition, and we
aim to fulfil it by providing a flexible and extensible framework on both the frontend
(user-facing python code) and the backend (compiled code).

To achieve this, the main Selene library itself acts as a routing frontend for the underlying
user's program, a quantum runtime optimiser, an error model and a simulation
engine, whereas the implementations of each such component are provided as plugins.
This allows developers, internally and externally, to create and distribute their own simulators.

We bundle some implementations of different components with selene for convenience, but
stress that these are not designed to cover all use cases.

Directories are structured as follows:

- `./selene-core`: Defining interfaces common to Selene itself and its plugins. This allows for
             plugins to depend on Selene related types without having to depend on Selene
             itself.
- `./selene-sim`: The main Selene library, providing the emulation library and the main entry
             point for the emulator.
- `./selene-ext`: Plugins to be bundled into selene, such as simulators, error models, etc.

Each library has a compiled component and a python component. Contrary to typical projects
which mix compiled code and python code, selene does *not* compile libraries as python
extensions. Instead, it bundles the compiled libraries within python wheels as a distribution
mechanism, and provides the functionality for using them to achieve emulation.

The python modules for extensions are presently lightweight: they define arguments for their
respective components, validate user inputs, format them in a manner suitable for passing
through to their compiled counterparts, and provide the path to the shared object bundled
with them.

## Fetching the project

We import Stim as a submodule in selene-ext/simulators/stim/upstream.
As such, when cloning selene, ensure that you clone with submodules:

```bash
git clone --recurse-submodules
```

If you have already cloned the repository and need to fetch the submodule, you can do so
by running:

```bash
git submodule update --init --recursive
```

## Building the project

### Build environment

We provide an optional devenv shell to get you setup for development. Devenv is based
around nix and provides reproducible environments, including dependencies and tools,
for MacOS and Linux users. To install devenv, head to devenv's
[getting started](https://devenv.sh/getting-started) page and follow the instructions.
To enter the devenv shell, run

```bash
devenv shell
```

This development shell provides the following:

- A stable rust toolchain (with rustc, cargo, clippy, rustfmt, and cbindgen)
- A python toolchain (with uv)
- Just (for running the justfile)
- gcc and libclang (for build stages)
- git hooks for pre-commit checks

Alternatively, you can install the required dependencies manually. The following are
required:

- uv >= 0.6
- rust >= 1.85 with the stable toolchain
- cbindgen ~= 0.28
- cargo-expand ~= 1.0
- cmake

Note: Cbindgen is used to generate C headers for selene and its extension plugins
based on rust implementations. These headers are shipped with selene and selene-core
wheels (in their `_dist/include` directory) respectively, allowing for downstream
implementations to depend on headers at build time using the python wheels as build
inputs.

### Building the rust library

All crates in the project can be built using `cargo build`, and all tests can be run
using `cargo test --workspace`.

To build a specific crate, you can use `cargo build -p <crate_name>`, where `<crate_name>` is
the name of the crate you want to build. Likewise, to test a specific create, you can use
`cargo test -p <crate_name>`. At the time of writing, the crates in the project are:

- selene-core
- selene-sim
- selene-simulator-stim
- selene-simulator-quest
- selene-simulator-coinflip
- selene-simulator-classical-replay
- selene-simulator-quantum-replay
- selene-simple-runtime
- selene-soft-rz-runtime
- selene-error-model-depolarizing
- selene-error-model-simple-leakage
- selene-error-model-ideal

### Building wheels

To build all python wheels using `just`, run:

```bash
uv run just build-wheels
```

Alternatively you can use `uv sync`.

We use Hatch for building the wheels and building and bundling artifacts
into a `_dist` directory for each python package.

### Python development environment

To enter a python development environment with all plugins initialised, run

```bash
uv run just develop
```

### Running tests

At current, python tests are confined to the selene library in the form of integration
tests from guppy through to simulation. They require the libraries to be built and available
either as installed in the python environment or locally by activating the python development
environment.

Python tests can be invoked using:

```bash
uv run pytest
```

When users develop their own plugins, it is likely that they will want to write isolated
tests of their plugins. We aim to provide a framework for comprehensive testing of plugins
in future.
