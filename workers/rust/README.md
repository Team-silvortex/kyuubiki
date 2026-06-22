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
  behavior. The `kyuubiki-headless` binary also exposes workflow
  template/init/inspect/validate/plan/run flows for service-only, browser-only,
  and hybrid headless automation. `plan` emits executor compatibility for
  `mock`, `service`, and `hybrid` before a live run is attempted.
- `crates/headless-sdk`
  Rust-first headless workflow templates, execution plans, dry-run support, and
  concrete R&D examples such as `material_heat_spreader_screening` for thermal
  material candidate comparison.
- `crates/deploy-server`
  Read-only Rust download/deployment metadata server for update catalogs,
  deploy descriptors, and artifact file distribution.
- `crates/benchmark`
  Benchmark profiles for medium, large, v2, and `10k`-plus scale targets up to
  `100k`, with `10k` now treated as the default local regression tier.
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

For the currently retained thermal-quad matrix optimization set and its
benchmark evidence, also read:

- `../../docs/solver-matrix-optimization-pack.md`

## Layout skeletons

- `linux/`
  repository-visible Linux install-layout skeleton used to keep expected
  runtime paths explicit even when no packaged output is committed yet
