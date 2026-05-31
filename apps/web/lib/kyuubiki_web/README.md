# Control Plane Modules

`apps/web/lib/kyuubiki_web` is organized by control-plane domain.

- `jobs/`
  Durable job state, progress events, stores, and watchdog handling.
- `results/`
  Analysis result persistence backends and store facade. Keep result payloads
  solver-agnostic and API-oriented.
- `library/`
  Project, model, and model-version persistence backends.
- `playground/`
  Agent RPC client, pool, registry, and local solver helpers. This is
  effectively the runtime/agent integration boundary despite the historical
  `playground/` name.
- `storage/`
  SQL backends, repo modules, schema setup, and persisted record structs. Keep
  SQL/runtime persistence concerns here rather than scattering them across job
  or frontend-facing modules.
- `workers/`
  Transitional worker adapters used by local integration flows.

Top-level modules such as `analysis.ex`, `library.ex`, `router.ex`, and
`application.ex` act as entry points and wiring rather than as domain buckets.

Additional orientation for the larger subdomains:

- `playground/`
  - `agent_client.ex`
    framed TCP RPC client, heartbeat handling, cancellation, and timeout logic
  - `agent_pool.ex`
    agent discovery and routing across static, manifest, and registry-backed
    endpoint sets
  - `agent_registry.ex`
    runtime registry for remotely deployed agents that self-register and
    heartbeat
  - `solver.ex`
    local Elixir-side helper solver used by smaller browser/playground flows
- `results/`
  - `store.ex`
    result store facade used by the orchestrator
  - `memory_backend.ex`
    in-memory/local fallback backend
  - `postgres_backend.ex`
    SQL-backed result persistence
- `storage/`
  - `storage.ex`
    storage mode selection and repo resolution
  - `persistence.ex`
    local file-backed persistence helpers used by fallback paths
  - `postgres_repo.ex` / `sqlite_repo.ex`
    Ecto repo modules for SQL-backed modes
  - `schema_setup.ex`
    bootstrap table setup for local development/runtime
  - `*_record.ex`
    persisted record structs for projects, models, versions, jobs, and results
