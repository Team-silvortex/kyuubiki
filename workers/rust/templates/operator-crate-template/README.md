# Operator Crate Template

This template is the quickest starting point for a repo-local or external-local
Rust operator crate.

## What it gives you

- a descriptor built with `OperatorDescriptorBuilder`
- a typed `JsonOperator` implementation
- a registration helper for `OperatorRegistry`
- a tiny example `main.rs` you can run immediately
- a `kyuubiki-operator.json` manifest for external-local package discovery

## Suggested workflow

1. copy this directory to a new location
2. rename the crate in `Cargo.toml`
3. rename the operator id / family in `src/lib.rs`
4. update `kyuubiki-operator.json` so package id, operator id, and entry symbol match
5. replace the sample input/output types with your real operator contract
6. add smoke and baseline tests before wiring it into a runtime

## Run the sample

From `workers/rust/templates/operator-crate-template`:

```bash
cargo run
```

The sample operator prints a JSON summary for a small temperature collection.
