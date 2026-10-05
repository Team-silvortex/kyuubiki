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

## Local Tests

Run `mix test` from `apps/web`. Default SQLite tests receive a fresh, uniquely
named directory under the platform temporary directory. They never reuse
`tmp/data/kyuubiki_dev.sqlite3`. After the suite, the test application stops and
only that run's database, SQLite sidecars, and empty directory are removed.
An unexpected file prevents directory removal rather than recursive deletion.

`SQLITE_DATABASE_PATH` overrides remain caller-owned and are not automatically
cleaned. Use only a disposable test database for this override: API tests reset
stores and are not safe against a user's active data. Development/production
paths and memory/PostgreSQL backend defaults are unchanged. For memory-mode
coverage, run `KYUUBIKI_STORAGE_BACKEND=memory mix test`.

The native repository entries `web-test`, `headless-test`, and
`headless-live-test` also need permission for local loopback TCP sockets. That
requirement comes from Mix/HTTP/Agent test communication, not a remote service.
The live Headless fixture owns its child before waiting for readiness and
removes its own JSON/SQLite scratch data when the fixture is released.
That target includes nine real Rust Agent + Orchestra HTTP cases: native
TaskIR execution and failure recovery, mixed-batch checkpoint targets, and the
CLI's blocked exit/report contract, plus 2D/3D modal result and repair checks.
Three cancellation cases cover precomputation, 2D modal matrix multiplication,
and 3D modal final validation. They stop downstream project creation and rerun
the identical task on the same Agent after cancellation. They cancel an owned
Agent directly, not through Orchestra's multi-Agent cancellation router.
A separate two-real-Agent case covers public asynchronous cancellation routing:
both slots are saturated, a queued spring job is held on its owner, cancellation
reaches only that owner, and the peer and following jobs still compute correctly.
The API's separate `cancellation` acknowledgement distinguishes registration
from stopped execution and cache cleanup; missing live ownership never causes
a cancellation broadcast to arbitrary pool peers. Queue, reservation, partial
delivery, and total-budget target capture have controlled regression coverage.
Modal checks inspect full mode arrays before report compaction and use independent
reference roots and matrix residuals. Native failure receipts retain task identity,
engine stage, and recovery actions through batch/checkpoint/SDK reporting; a task
failure never automatically replays on another Agent. Other workflow cases use
controlled Agent fixtures. The real Agent cases use local static endpoints
with unknown package readiness, not a registered/authenticated deployment. An explicitly detached
package runtime is checked separately by the API routing regressions.
