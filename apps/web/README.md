# Web Orchestrator

`apps/web` is the Elixir control plane for Kyuubiki.

It owns:

- HTTP APIs
- job lifecycle and cancellation
- SQLite/PostgreSQL persistence
- result chunk delivery
- watchdog and health surfaces
- distributed agent discovery, registry, and routing

It should not absorb browser-specific UI concerns or numerical solver internals.
