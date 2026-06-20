# Rust Workspace

`workers/rust` is the Kyuubiki data-plane workspace.

It currently contains:

- `crates/protocol`
  Shared RPC messages, job/result payloads, and agent-wire contracts.
- `crates/operator-sdk`
  Author-facing Rust traits and registry helpers for building workflow
  operators against the shared protocol contract, including external-local
  manifest discovery, cross-platform load-plan resolution, and activation
  hooks.
- `templates/operator-crate-template`
  Copyable starter crate for external-local Rust operators built on top of the
  author-facing operator SDK, including a `cdylib` export symbol for runtime
  loading tests and host integration.
- `crates/engine`
  Engine-facing solve entry points and chunk helpers that sit above raw solver
  kernels.
- `crates/solver`
  FEM kernels, sparse/dense solve paths, and numerical utilities.
- `crates/cli`
  TCP solver agent, local runtime entry point, and remote self-registration
  behavior.
- `crates/deploy-server`
  Read-only Rust download/deployment metadata server for update catalogs,
  deploy descriptors, and artifact file distribution.
- `crates/benchmark`
  Benchmark profiles for medium, large, v2, and `10k`-plus scale targets, with
  `10k` now treated as the default local regression tier.
- `crates/installer`
  Cross-platform installer/deployment CLI reused by the Tauri installer GUI.

This workspace should remain frontend-agnostic and Phoenix-agnostic. The Rust
side is the reusable computation/runtime layer, not the control plane.

Crate-level test expectations stay close to the code:

- `crates/solver/tests/`
  solver behavior, numerical baselines, and progress/result validation
- other crate-local `tests/`
  engine, CLI agent, benchmark, and installer/runtime checks

Prefer colocated crate tests so runtime behavior stays near the code it
validates.

## Layout skeletons

- `linux/`
  repository-visible Linux install-layout skeleton used to keep expected
  runtime paths explicit even when no packaged output is committed yet
