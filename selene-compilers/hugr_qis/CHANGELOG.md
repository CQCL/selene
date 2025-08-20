# Changelog

## [0.3.0](https://github.com/CQCL/selene/compare/selene-hugr-qis-compiler-v0.2.1...selene-hugr-qis-compiler-v0.3.0) (2025-08-20)


### ⚠ BREAKING CHANGES

* add measure_leaked, add bool/uint future results, add simple leakage error model ([#25](https://github.com/CQCL/selene/issues/25))

### Features

* add measure_leaked, add bool/uint future results, add simple leakage error model ([#25](https://github.com/CQCL/selene/issues/25)) ([32c0215](https://github.com/CQCL/selene/commit/32c021524f10de86a78f2b22a0b697fbb6936e07))
* add support for aarch64 linux ([#16](https://github.com/CQCL/selene/issues/16)) ([2a11286](https://github.com/CQCL/selene/commit/2a1128675d15d400b6fee20c6500aabad94509a5))
* **compiler:** `check_hugr` function for early faily on invalid HUGR ([#7](https://github.com/CQCL/selene/issues/7)) ([af79b38](https://github.com/CQCL/selene/commit/af79b385bdb8487cd95bac0722c42a23b9a24c96))
* **compiler:** debug print HugrReadError ([#27](https://github.com/CQCL/selene/issues/27)) ([fa0de7f](https://github.com/CQCL/selene/commit/fa0de7f85deb44d8fd6efbe7ed891cb69d59fdc7))
* **compiler:** include generator data if present in validation error ([#30](https://github.com/CQCL/selene/issues/30)) ([8c0d503](https://github.com/CQCL/selene/commit/8c0d503d2f70509c0746ea9932804cd9518e8611))
* **compiler:** update to tket v0.13, hugr v0.22 ([#28](https://github.com/CQCL/selene/issues/28)) ([49c200c](https://github.com/CQCL/selene/commit/49c200ccf4460ed3e9d5c7225fea559d39c4f0e0))
* Lower heap arrays using the selene heap ([#42](https://github.com/CQCL/selene/issues/42)) ([9465f6a](https://github.com/CQCL/selene/commit/9465f6a96afd16e0d87daeb23b9abcb14dfa724b))
* Switch to heap array lowering ([#36](https://github.com/CQCL/selene/issues/36)) ([2cfbfab](https://github.com/CQCL/selene/commit/2cfbfab75fa02044091aa94272b971e9c132aa12))
* use hugr entrypoint to avoid looking for "main"  ([#9](https://github.com/CQCL/selene/issues/9)) ([4bd8bce](https://github.com/CQCL/selene/commit/4bd8bce4c09b09f8e7e1b2ece04acf1b6d32819a))


### Bug Fixes

* Fixes from integration tests - rotation codegen, array results, process environment management ([#1](https://github.com/CQCL/selene/issues/1)) ([59b1315](https://github.com/CQCL/selene/commit/59b13151ea0ab7063bb3220364d608c1b051618b))
* Update snapshots ([#49](https://github.com/CQCL/selene/issues/49)) ([457729f](https://github.com/CQCL/selene/commit/457729ffb49439cceb97c0e3bbbb38f775c4d3b5))
