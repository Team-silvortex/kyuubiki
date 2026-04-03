# Workers

Compute and runtime tooling live here.

- `rust/`
  Rust workspace containing:
  - `protocol/` shared RPC and result payload definitions
  - `engine/` reusable engine-facing solve and chunk helpers
  - `solver/` FEM kernels and numerical paths
  - `cli/` TCP agent and local worker executable
  - `benchmark/` benchmark workloads and profiles
  - `installer/` cross-platform installer/deployment CLI

The `workers/` tree should stay UI-agnostic. It is the data plane, not the
control plane.
