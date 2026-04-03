# Web Orchestrator

`apps/web` is the Elixir control plane for Kyuubiki.

It owns:

- HTTP APIs
- job lifecycle and cancellation
- SQLite/PostgreSQL persistence
- result chunk delivery
- watchdog and health surfaces
- distributed agent discovery, registry, and routing

Key internal domains:

- `jobs/`
  job state, progress events, watchdog, and store backends
- `results/`
  analysis result persistence and retrieval
- `library/`
  project/model/model-version persistence
- `playground/`
  agent RPC client, pool, registry, and solver integration helpers
- `storage/`
  repo modules, schema setup, record structs, and storage-mode selection

It should not absorb browser-specific UI concerns or numerical solver internals.
