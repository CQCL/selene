# Overview

The intent of Selene is to be an emulation platform for quantum/classical hybrid programs.
The primary usecase is for HUGR programs targeting the Helios Quantum Instruction Set, but
as the project has grown it has been clear that we need more than one hardcoded pipeline.
Instead, we need a flexible pipeline that can handle a wide range of inputs, configuration,
and various intermediate stages. It may be the case that users wish to start a build already
having LLVM bitcode for the user program, or an object file, or a pytket circuit.

To cater for this, we formalise the build pipeline with three main concepts: ArtifactKind,
Artifact and Step.

Before we go into the details of these concepts, we will first describe the general
concepts used in building a selene instance.

## General Concepts

It is a key assumption that hybrid programs target a quantum control system, and are likely
to be compiled rather than interpreted. It is also assumed that the classical code runs
natively on the CPU, and the quantum aspects of the code are achieved by calling functions
defined on the target platform. The program may also call library functions available on
the platform, such as `malloc` or BLAS' `gemm` function.

There may be several ways of building such a program, but a manner that I am most familiar
with looks like:
- The user writes a program in Guppy.
- They compile this program to HUGR.
- This HUGR gets compiled to one of:
  - An object file, making use of external quantum function calls such as
    `my_quantum_platform_do_rz(qubit: u64, theta: f64)`.
  - LLVM IR or LLVM bitcode
    - This is then compiled in a subsequent step into an object file with the
      above calling convention
- The object file is linked against a "shim" library which maps the appropriate
  platform calls to selene functions, e.g. `selene_rz(qubit: u64, theta: f64)`.
- The object file is linked against `libselene.so`, which contains the selene hooks
  (such as `selene_rz`) for invoking quantum operations.
- If any non-quantum external functions are used (such as `gemm`), then the program
  is further linked against the appropriate libraries.

The result is an executable that can provide configurable runs of a hybrid program
for the purpose of emulation.

Although this pipeline is the original inspiration for Selene, we aim to generalise
the concept of a build pipeline to allow for alternative workflows, including alternative
steps and intermediate artifacts. It may be possible that users wish to provide a
pytket Circuit object as an input, compile it to QIR, and build that into a selene executable.
An implementer may work on creating the steps to make this possible, expose it either in
Selene itself or in an external repository, and after an appropriate `import` a general user
may be able to invoke `selene_sim.build` with a pytket Circuit as input without needing
to get bogged down in implementation detail.

To achieve this, we need a formal, configurable build pipeline.

## Core Types (types.py)

### `ArtifactKind`

When the Selene build pipeline is invoked, it is given an arbitrary object (which we call the
*input resource*). Selene needs to understand what this object is in the context of a build, and what
to do with it.

The specific term we use for "what this object is in the context of a build" is *kind*, and is
represented by the `ArtifactKind` class.

`ArtifactKind` comprises a *name* (`str`), a priority (`int`), and a function `matches` that is used to
determine whether a resource (of type `any`) is a resource of that kind. When selene_sim's build pipeline
is invoked with some object (which we call a *resource*), the pipeline will check the resource against
a set of known `ArtifactKind`s (see the Build Planner section below), in descending order of priority,
until it finds an `ArtifactKind` that is satisfied by the resource. The first match is the chosen kind.

Examples of ArtifactKind provided in ./builtin.py include:
<table>
<thead>
    <tr><th>ArtifactKind</th><th>Resource Type</th></tr>
</thead>
<tbody>
    <tr><th>HUGR Package</th><td>hugr.package.Package</td></tr>
    <tr><th>HUGR Package Pointer</th><td>hugr.package.PackagePointer</td></tr>
    <tr><th>HUGR File</th><td>pathlib.Path (.hugr)</td></tr>
    <tr><th>LLVM IR File</th><td>pathlib.Path (.llvm)</td></tr>
    <tr><th>LLVM Bitcode File</th><td>pathlib.Path (.bc)</td></tr>
    <tr><th>Helios Object File</th><td>pathlib.Path (.o)</td></tr>
    <tr><th>Selene Object File</th><td>pathlib.Path (.o)</td></tr>
    <tr><th>Final Executable</th><td>pathlib.Path (.x)</td></tr>
</tbody>
</table>

While it is clear that many ArtifactKinds represent paths to files, they each
have very different actions that should be performed on them.

- the `HUGR File` kind may simply check that the path points to an existing file with
  the `.hugr` extension, satisfied that a corrupt file will fail in the following
  stage.
- the `Selene Object File` has the same extension as the `Helios Object File`, but
  the `matches` function will check that the file is a valid selene object file by
  checking that the external calls that it makes are valid selene calls.

### `Artifact`

An artifact is a manifestation of an *ArtifactKind*. It comprises a *resource* object (of
type `any`), a *kind* (of type `ArtifactKind`) that the *resource* resembles, and *metadata*
(of type `dict[str, str]`).

Artifacts have a `validate` method that verifies that the *resource* it holds is valid according
to *kind*.

Once the *input resource* has successfully matched against an `ArtifactKind`, an *input artifact*
is created.

### `Step`

A step defines a mapping from one ArtifactKind to another. For example, a HUGR file may be transformed
into an LLVM Bitcode file, which may be transformed into a Helios Object file. It comprises:
- an *input kind* (of type `ArtifactKind`),
- an *output kind* (of type `ArtifactKind`),
- a cost (of type `float`),
- a metadata filter (which may reject Artifacts with certain metadata), and
- a function that performs the transformation.

## Build Planner

The build planner is responsible for maintaining a directed graph, with nodes representing
`ArtifactKind`s and edges representing `Step`s that transform from the source kind to the
destination kind. This acts as the registry for `ArtifactKind`s and `Step`s that are visible
to the build pipeline. A global `DEFAULT_BUILD_PLANNER` is provided by `builtins.py` that
covers a reasonable set of possible input types and intermediate artifacts, as well as steps
that transform between them.

`selene_sim`'s `build` function (defined in [__init__.py](__init__.py)) accepts a `planner`
argument of type `BuildPlanner`. If one is not provided, then the `DEFAULT_BUILD_PLANNER` is used
instead.

This allows external libraries to define their own `ArtifactKind`s and `Step`s, and either
register them with the `DEFAULT_BUILD_PLANNER` or create their own `BuildPlanner` instance
for users to use.
