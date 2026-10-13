# Standalone Agent recovery controls, 2026-10-13

Version: daji 3.6.0. Scope: durable admission discovery, exact async-owner
cancellation, native gateway validation, and Workbench/PWDT recovery controls.

## Delivered contract

- `list_durable_agent_executions` reads bounded admission metadata, never solver
  result receipts. Pages contain 1 to 64 entries in ascending execution-ID order.
  A cursor is not a snapshot or a time ordering. Refresh the first page for new
  IDs that precede the previous cursor. There is no automatic pruning or replay.
- `cancel_durable_agent_execution` requires the inspected execution ID, producer
  process incarnation and request digest. It selects the exact live async owner,
  not whichever execution currently has a matching job or transport request ID.
- Cancellation acknowledges `requested`, `not_running` or `owner_not_observed`.
  It always states `execution_terminal_confirmed: false`; only a subsequent
  durable execution inspection can confirm a final result. No pending job-wide
  cancellation is created. Synchronous wrappers are not async cancellation owners.
- Native `/api/direct-mesh/tasks/list` and `/api/direct-mesh/tasks/cancel` routes
  retain gateway authorization, endpoint allowlisting and one-endpoint pinning.
  They validate identity, schemas, page ordering, size, cursors and cancellation
  semantics before returning a receipt. No automatic retry or failover is added.
- In direct mode, System > Runtime exposes a collapsed Direct Execution Recovery
  panel. Reads are explicit, the page size is 24, and only the selected receipt is
  inspected. Results are not applied to the current model or visualization.
- GUI controls use the same `directExecution/list`, `directExecution/inspect` and
  `directExecution/cancel` action handlers as PWDT. Cancellation requires user
  confirmation. Logs retain bounded identity/status metadata rather than numerical
  arrays or detailed error payloads. Backend authority changes fence late replies.

## Integration defects found during browser qualification

1. The initial panel mount reused a recovery slot visible only on the Watchdog
   subpage. The final mount is directly under Runtime, visible in direct mode and
   collapsed by default.
2. The existing system-sidebar invoker awaited PWDT but discarded its return value
   and returned `{}`. A real backend request therefore appeared ineffective to any
   panel needing a receipt. The sidebar now forwards the invoker unchanged.

## Verification

Final targeted checks on local macOS:

- Rust libraries and CLI unit tests: 422 passed, 1 ignored. The ignored test starts
  a full installed stack and is not part of this isolated qualification.
- Real Agent process suites: 37 passed, including shared fixture checks. They cover
  async/sync durable receipts, forced termination, restart, exact cancellation,
  lifecycle capacity and drain. Three new recovery scenarios verify discovery and
  original-owner cancellation, interrupted records versus a replacement execution,
  and disabled-store fail-closed behavior.
- Frontend unit/controller tests: 67 passed. These include invalid/foreign receipts,
  unsafe pagination, request pinning, no retry after uncertain cancellation,
  authority fencing and bounded PWDT results, plus existing run-controller tests.
- Browser interaction tests: 2 passed against an isolated current-source frontend
  and mocked HTTP gateway. They exercise actual rendered panel controls and the
  PWDT bridge, not just exported helper functions.
- Rust Clippy with warnings denied, frontend type checking, and the 800-line
  frontend guard passed.

Primary commands:

```text
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-cli -p kyuubiki-desktop-runtime --lib -p kyuubiki-protocol --quiet
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test agent_durable_recovery_live --test agent_durable_async_live --test agent_durable_execution_live --test agent_execution_cancel_live --test agent_lifecycle_live
workers/rust/target/debug/kyuubiki-script-runner frontend-unit-test durable-direct workbench-script-state-controller workbench-backend-service-composer workbench-run-controller workbench-study-run-backend
node --test tests/integration/workbench-ui-durable-direct-recovery.test.mjs
```

## Boundaries

Not qualified: physical power loss, installed Tauri GUI, remote Linux deployment,
or one end-to-end browser-to-real-Agent run in this round. Process-kill evidence is
not disk-controller power-loss evidence. Browser routing and native/Agent behavior
were verified in separate scoped suites.

Ordinary GUI/PWDT `job/run` and `job/cancel` are not silently rerouted. Durable
default submission still needs caller-handle persistence and explicit capability
selection. Ordinary synchronous solves have no admission in this ledger. No
automatic result-to-model restoration, full standalone workflow scheduler, TLS
parity or Installer default provisioning is claimed. The raw Agent listener still
requires a trusted network boundary. These additions do not qualify the overall
single-Agent production workflow as complete.

No version bump, package installation, Git commit or push was performed. Isolated
browser workspaces and Agent test stores are removed by their fixtures.
