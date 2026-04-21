# Workers

Compute and runtime tooling live here.

## Quick Map

- `rust/`
  Shared protocol contracts, reusable engine logic, solver kernels, the TCP
  agent runtime, benchmark tooling, and installer/deployment CLI.

## Ownership Boundary

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

See also:

- [workers/rust/README.md](/Users/Shared/chroot/dev/kyuubiki/workers/rust/README.md)
- [docs/repository-structure.md](/Users/Shared/chroot/dev/kyuubiki/docs/repository-structure.md)
- [docs/protocols.md](/Users/Shared/chroot/dev/kyuubiki/docs/protocols.md)
