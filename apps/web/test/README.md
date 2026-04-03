# Web Tests

`apps/web/test` follows the same control-plane domain split as the Elixir
source tree.

- `kyuubiki_web/api/`
  Router- and HTTP-level integration tests.
- `kyuubiki_web/jobs/`
  Job state, store, and watchdog tests.
- `kyuubiki_web/playground/`
  Agent-runtime boundary tests such as agent client, pool, and registry.
- `kyuubiki_web/workers/`
  Transitional worker adapter tests.

Keep tests close to the source-domain they protect, even when they exercise
multiple modules together.
