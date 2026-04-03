# Rust Crates

`workers/rust/crates` is organized by runtime responsibility:

- `protocol`
  Shared wire/data contracts
- `engine`
  Engine-facing solve entry points and result chunk helpers
- `solver`
  Numerical kernels and linear-system implementations
- `cli`
  TCP solver agent and local runtime entry point
- `benchmark`
  Performance baselines and scaling profiles
- `installer`
  Cross-platform deployment and environment tooling

Keep these crates loosely coupled. Shared types should flow through
`protocol` or narrow engine-facing APIs rather than through ad hoc cross-crate
knowledge.
