# Rust Test Notes

Rust crate-level tests live with their crates.

Current examples:

- `crates/solver/tests/`
  Solver behavior tests such as mock/progress validation.

Prefer colocated crate tests for engine, solver, CLI, benchmark, and installer
behavior so runtime concerns stay near the code they validate.
