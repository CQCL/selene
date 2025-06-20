# Selene-Core

Selene is designed to be extensible through the use of plugins, in the form
of compiled libraries and lightweight python interfaces that provide configuration
for the selene-sim frontend. We achieve this through this selene-core crate and
python module.

Each plugin should comprise a python component and a compiled library component.
The compiled library implements the Selene plugin API, and the python component
provides configuration, link information and the path to the compiled library to
the selene frontend.

## The python module

The selene-core python module provides interfaces for plugins to adhere to. It also
provides a bundled include directory, containing C headers for the Selene plugin API
for each type of component.

To access the C headers in the build stage of a python package, depend on selene-core
as a build dependency and call `selene_core.get_include_directory()`. The resulting
path can be provided to a build system for C or C++ and the plugin APIs can be included
through:
```c
#include <selene/simulator.h>   # for the simulator API
#include <selene/error_model.h> # for the error model API
#include <selene/runtime.h>     # for the runtime API
```

By implementing the required functions, the plugin can be dynamically loaded by Selene
at runtime.

## The rust crate

The selene-core rust crate defines the compiled plugin interfaces for the Selene
backend to use. It additionally provides helper functionality for rust-based plugins
to expose the Selene plugin APIs while providing a more idiomatic trait interface.

See the following for examples:
- [example simulator](examples/simulator/rust/lib.rs)
- [example error model](examples/error_model/rust/lib.rs)
- [example runtime](examples/runtime/rust/lib.rs)
