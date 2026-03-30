# Contributing

## Default Workflow: TDD

This repository uses test-driven development by default.

For every behavior change:

1. Write or extend a test that captures the new behavior
2. Run the smallest possible test scope and watch it fail
3. Implement the minimum code needed to make it pass
4. Refactor while keeping the test suite green
5. Run the full verification pass before finishing

## Definition of Done

A change is not done until all of the following are true:

- the behavior is described by a test
- the smallest relevant test passed during implementation
- the full suite passes locally
- formatting checks pass locally
- docs or schemas are updated when contracts changed

## Useful Commands

- `make tdd-web FILE=test/kyuubiki_web/jobs/store_test.exs`
- `make tdd-web TEST="--only focus"`
- `make tdd-rust FILTER=solver`
- `./scripts/kyuubiki smoke`
- `./scripts/kyuubiki worker -- --job-id demo --project-id p1 --case-id c1 --steps 3`
- `make test`
- `make verify`

## Test Placement

- Elixir unit and integration tests go under `apps/web/test`
- Rust unit tests live next to code, integration tests go under each crate's `tests/`
- Shared contract changes should be covered in both stacks when relevant
