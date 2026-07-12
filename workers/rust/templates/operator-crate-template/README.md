# Operator Crate Template

This template is the quickest starting point for a repo-local or external-local
Rust operator crate.

## What it gives you

- a descriptor built with `OperatorDescriptorBuilder`
- a typed `JsonOperator` implementation
- a registration helper for `OperatorRegistry`
- a tiny example `main.rs` you can run immediately
- a `kyuubiki-operator.json` manifest for external-local package discovery
- package-level SDK compatibility and validation posture fields
- a descriptor readiness test using `operator_descriptor_readiness`

## Suggested workflow

1. copy this directory to a new location
2. rename the crate in `Cargo.toml`
3. rename the operator id / family in `src/lib.rs`
4. update `kyuubiki-operator.json` so package id, host version, operator id,
   validation notes, and entry symbol match
5. replace the sample input/output types with your real operator contract
6. keep the descriptor readiness test green while changing schemas and ports
7. add smoke and baseline tests before wiring it into a runtime

Keep `validation_status` at `partial` until the operator has repeatable
baseline evidence. Use `verified` only when the package has release-quality
checks, tolerances, and documented limits.

## Run the sample

From `workers/rust/templates/operator-crate-template`:

```bash
cargo run
```

The sample operator prints a JSON summary for a small temperature collection.
