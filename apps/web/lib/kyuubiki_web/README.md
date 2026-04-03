# Control Plane Modules

`apps/web/lib/kyuubiki_web` is organized by control-plane domain.

- `jobs/`
  Durable job state, progress events, stores, and watchdog handling.
- `results/`
  Analysis result persistence backends and store facade.
- `library/`
  Project, model, and model-version persistence backends.
- `playground/`
  Agent RPC client, pool, registry, and local solver helpers. This is
  effectively the runtime/agent integration boundary.
- `storage/`
  SQL backends, repo modules, schema setup, and persisted record structs.
- `workers/`
  Transitional worker adapters used by local integration flows.

Top-level modules such as `analysis.ex`, `library.ex`, `router.ex`, and
`application.ex` act as entry points and wiring rather than as domain buckets.
