# Results Domain

This directory owns analysis result persistence.

- `store.ex`
  Result store facade used by the orchestrator.
- `memory_backend.ex`
  In-memory/local fallback backend.
- `postgres_backend.ex`
  SQL-backed result persistence.

Result payloads should stay solver-agnostic and API-oriented.
