develop:
    uv sync

clean-artifacts:
    rm -rf **/_dist
    find . -wholename "*/c/build" -type d -exec rm -rf {} \;

build-wheels:
    uv build --all-packages

test-py *TEST_ARGS: develop
    uv run pytest {{TEST_ARGS}}

test-rs *TEST_ARGS:
    uv run cargo test {{TEST_ARGS}}

BIND_BUILD := "target/selene-bindings-build"
PLUGIN_EXPAND := "target/plugin-expand"

generate-plugin-header plugin:
    mkdir -p {{PLUGIN_EXPAND}}
    cd selene-core/examples/{{plugin}} && cargo expand > ../../../{{PLUGIN_EXPAND}}/{{plugin}}.rs
    cbindgen \
      --config selene-core/examples/cbindgen.toml \
      --output selene-core/c/include/selene/{{plugin}}.h \
      {{PLUGIN_EXPAND}}/{{plugin}}.rs
    rm -rf target/tmp


generate-selene-core-headers:
    cbindgen \
      --config selene-core/cbindgen.toml \
      --crate selene-core \
      --output selene-core/c/include/selene/core_types.h

    just generate-plugin-header error_model
    just generate-plugin-header simulator
    just generate-plugin-header runtime

generate-headers:
    just generate-selene-core-headers
    just generate-selene-sim-headers

generate-selene-sim-headers:
    cbindgen \
      --config selene-sim/cbindgen.toml \
      --crate selene-sim \
      --output selene-sim/c/include/selene/selene.h

generate-selene-sim-bindings: generate-selene-sim-headers
    mkdir -p {{BIND_BUILD}}

    cmake \
      -B{{BIND_BUILD}} \
      -DCMAKE_INSTALL_PREFIX=selene-sim/python/selene_sim/_dist \
      selene-sim/c

    cmake \
      --build {{BIND_BUILD}} \
      --target install

    rm -rf {{BIND_BUILD}}

generate-bindings: generate-selene-core-headers generate-selene-sim-bindings

build-ci: 
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p /tmp/ci-cache
    export CACHE_CARGO=true
    uv build --package selene-core --out-dir wheelhouse
    cibuildwheel .
