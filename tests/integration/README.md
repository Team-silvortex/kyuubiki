# Integration Tests

This directory holds cross-process integration tests that exercise multiple
Kyuubiki programs working together.

The first target focuses on the local workstation stack:

- unified launcher
- orchestrator API
- Rust solver agents
- real HTTP job submission and polling

Run with:

- `make test-integration-api`
