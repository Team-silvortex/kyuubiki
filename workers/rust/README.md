# Rust Worker Boundary

`workers/rust` is reserved for the Rust workspace that will host compute and
transport crates.

Planned crates:

- `crates/protocol` for shared transport/domain types
- `crates/solver` for FEM execution logic
- `crates/cli` for local worker startup and diagnostics

The directory structure exists now so the workspace can be introduced without
reshuffling the repository.
