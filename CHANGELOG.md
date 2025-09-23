# Changelog

## [0.2.5](https://github.com/CQCL/selene/compare/selene-sim-v0.2.4...selene-sim-v0.2.5) (2025-09-23)


### Features

* **compiler:** Bump tket version; add wasm + gpu to the hugr-qis registry ([c69155d](https://github.com/CQCL/selene/commit/c69155d9717e942c6c67065dbf47cdb156542689))
* Emit a nicer error when trying to emulate unsupported pytket ops ([#72](https://github.com/CQCL/selene/issues/72)) ([d88a28a](https://github.com/CQCL/selene/commit/d88a28a827d15fb2fcbc036964452fdcfd7b1cd8))


### Bug Fixes

* **compiler:** error when entrypoint has arguments ([#84](https://github.com/CQCL/selene/issues/84)) ([604b131](https://github.com/CQCL/selene/commit/604b1311b96593609e699a6bb8251ad3c952ebdb))
* **compiler:** update tket-qystem to fix CZ bug ([#78](https://github.com/CQCL/selene/issues/78)) ([3991f11](https://github.com/CQCL/selene/commit/3991f11a73d8ceebf0346a8c43248fde73e1b549))

## [0.2.4](https://github.com/CQCL/selene/compare/selene-sim-v0.2.3...selene-sim-v0.2.4) (2025-08-28)


### Bug Fixes

* correct post_runtime duration metric ([#74](https://github.com/CQCL/selene/issues/74)) ([0bef66a](https://github.com/CQCL/selene/commit/0bef66aeaaccbadf08ba38a735a5146382326c2a))

## [0.2.3](https://github.com/CQCL/selene/compare/selene-sim-v0.2.2...selene-sim-v0.2.3) (2025-08-26)


### Features

* Better exception handling for parse_shots=False ([#70](https://github.com/CQCL/selene/issues/70)) ([3caf530](https://github.com/CQCL/selene/commit/3caf530dfcf616fa3f2e335692b6963a1b828b11))
* Fine-grained timeout configuration ([#69](https://github.com/CQCL/selene/issues/69)) ([072842e](https://github.com/CQCL/selene/commit/072842efa396ab9d964f4abd6ef4badb49bf002a))
* update to tket-qsystem 0.20 ([#66](https://github.com/CQCL/selene/issues/66)) ([7191b07](https://github.com/CQCL/selene/commit/7191b07c00571c0298b3cfc334058d3e649fe377))

## [0.2.2](https://github.com/CQCL/selene/compare/selene-sim-v0.2.1...selene-sim-v0.2.2) (2025-08-20)


### Features

* random_advance ([#55](https://github.com/CQCL/selene/issues/55)) ([974b496](https://github.com/CQCL/selene/commit/974b496e3bc15b8ce155542d4f31e4e9fad245ed))
