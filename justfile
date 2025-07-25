develop:
    uv sync

clean-artifacts:
    rm -rf **/_dist
    find . -wholename "*/c/build" -type d -exec rm -rf {} \;

build-wheels:
    uv build --all-packages

test-py: develop
    pytest

test-rs:
    cargo test

BIND_BUILD := "/tmp/selene-bindings-build"

generate-selene-headers:
    cd selene-sim && cbindgen -o c/include/selene/selene.h

generate-selene-bindings: generate-selene-headers
    mkdir -p {{BIND_BUILD}}
    cmake -B{{BIND_BUILD}} -DCMAKE_INSTALL_PREFIX=selene-sim/python/selene_sim/_interface selene-sim/c
    cmake --build {{BIND_BUILD}} --target install
    rm -rf {{BIND_BUILD}}

build-ci: 
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p /tmp/ci-cache
    export CACHE_CARGO=true

    uv build --package selene-core --out-dir wheelhouse
    export SELENE_CORE_WHL=$(ls -t wheelhouse/selene_core-*.whl | xargs readlink -f | head -n1)

    cibuildwheel selene-compilers/hugr_qis
    export SELENE_HUGR_QIS_WHL=$(ls -t wheelhouse/selene_hugr_qis_compiler-*.whl | xargs readlink -f | head -n1)

    cibuildwheel .
