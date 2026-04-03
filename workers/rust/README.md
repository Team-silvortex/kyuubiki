# Rust Workspace

`workers/rust` is the Kyuubiki data-plane workspace.

It currently contains:

- `crates/protocol`
  Shared RPC messages, job/result payloads, and agent-wire contracts.
- `crates/engine`
  Engine-facing solve entry points and chunk helpers that sit above raw solver
  kernels.
- `crates/solver`
  FEM kernels, sparse/dense solve paths, and numerical utilities.
- `crates/cli`
  TCP solver agent, local runtime entry point, and remote self-registration
  behavior.
- `crates/benchmark`
  Benchmark profiles for medium, large, v2, and `10k` scale targets.
- `crates/installer`
  Cross-platform installer/deployment CLI reused by the Tauri installer GUI.

This workspace should remain frontend-agnostic and Phoenix-agnostic. The Rust
side is the reusable computation/runtime layer, not the control plane.
