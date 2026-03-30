# TDD Guide

## Why TDD Here

This project spans two runtimes:

- Elixir for orchestration and UI-facing state
- Rust for worker execution and compute pipelines

That split makes regressions easy to hide in the seams. TDD keeps those seams
visible by making behavior explicit before implementation.

## Loop

1. Pick one behavior
2. Write the smallest test that proves it
3. Confirm it fails for the expected reason
4. Implement the smallest change
5. Refactor after the test turns green
6. Run the full relevant suite

## What To Test First

### Elixir

- job validation and lifecycle transitions
- orchestration-side progress application
- worker adapter behavior
- future LiveView event handling and boundary conditions

### Rust

- protocol and domain invariants
- solver progress semantics
- transport parsing and serialization
- checkpoint, retry, and cancellation edge cases

## Testing Rules

- Prefer behavior tests over implementation-detail tests
- One failing test should correspond to one behavior change
- Avoid adding production code without a test proving the need
- When fixing a bug, reproduce it with a regression test first
- When changing shared contracts, update tests in both runtimes if both consume the behavior

## Command Reference

- Fast Elixir loop: `make tdd-web FILE=test/kyuubiki_web/jobs/store_test.exs`
- Fast Rust loop: `make tdd-rust FILTER=protocol`
- Full project pass: `make test`
- Full verification pass: `make verify`
