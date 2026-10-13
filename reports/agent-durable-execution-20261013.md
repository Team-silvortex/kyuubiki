# Standalone Agent Durable Execution

Date: 2026-10-13. Version: daji 3.6.0. Scope: local macOS source-build qualification.

## Contract and Implementation

- New framed RPCs: `run_durable_agent_task` and `fetch_durable_agent_execution`.
- Callers choose and retain a 32-character lowercase hex execution ID before
  submission, together with the selected Agent identity/endpoint and store lifetime.
- The wrapper accepts solver RPCs and execute-mode TaskIR, not control methods,
  nested durable wrappers or TaskIR preflight. Engine/operator code is unchanged.
- Input identity uses the existing typed binary64 digest encoder with a distinct
  execution-domain prefix. It does not round tiny or adjacent floating-point values
  to a fixed decimal precision and does not duplicate the input model for hashing.
- Admission is synced before dispatch. A checksummed immutable receipt is synced
  before transport publication. Files use atomic temporary-file rename and Unix
  directory sync. An exclusive process lock prevents concurrent store ownership.
- Same-ID/same-input retries return existing status/receipt. Same-ID/different-input
  retries fail closed. An interrupted admission without a receipt returns
  `outcome_unknown`, never automatic replay permission. `failed` preserves the
  original solver error, not a claim that no external side effects occurred.
- Native HTTP routes `/api/direct-mesh/tasks` and `/api/direct-mesh/tasks/inspect`
  forward to one pinned, allowlisted endpoint. They preserve gateway authentication,
  bypass the old result cache, and do not fail over after an ambiguous dispatch.

## Storage and Resource Policy

`KYUUBIKI_AGENT_EXECUTION_STORE` explicitly selects a dedicated absolute directory.
The parent must exist; the final directory is created with private Unix permissions.
The resolved directory and policy are visible under `describe_agent.durable_execution`.
An unconfigured store rejects durable requests rather than falling back to memory.

Limits: 1,024 admissions, 8 MiB per encoded receipt, 128 MiB total retained record
bytes. Active work reserves room for its bounded receipt. No model-input copies,
backup generations, automatic eviction or background archive are introduced.
Capacity rejects new work. A write failure blocks further durable admission in that
process; an unacknowledged receipt cannot be exposed as durable success. Read-side
corruption blocks the affected execution rather than replaying it. Readers validate
bounded length, checksum and identity. Writers reject receipts that the restart
parser cannot decode, including excessive nesting.

Only owned `.next` files are reclaimed under the startup lock. Committed identities
remain until an explicit development reset with the Agent stopped and callers
retired. Deleting or replacing the store destroys its deduplication history; a
caller must not treat a new empty store as proof an earlier task never ran.

## Validation

The live suite launches real Rust Agent child processes on private dynamic ports,
with no Orchestra or Node runtime, using temporary directories cleaned by the fixture.
`Child::kill` uses SIGKILL on the tested Unix host, bypassing shutdown hooks.

- Completed solve, then kill/restart twice: the original result is retained, and
  duplicate submissions do not enter a solver hold or acquire an execution slot.
- Kill an admitted, held execution before computation: repeated restart/retry
  remains `outcome_unknown`, with no second computation.
- Hold after receipt sync but before final reply, then kill: the request receives
  no terminal response, yet the original completed receipt survives two restarts.
- Duplicate submission while running preserves the single active execution.
- Different inputs cannot rebind an ID. Failed solves are retained across restart.
- Real native solver TaskIR executes and its result survives restart; axial-bar
  displacement and stress are independently checked against FL/EA and F/A.
- Exact process/request/generation/job cancellation still works through the wrapper;
  its original cancelled receipt survives restart.
- Missing configuration, corrupt receipts and forbidden nested/control calls fail
  closed. An unrelated execution can proceed after read-side receipt corruption.
- Store tests cover exclusive ownership, quotas, checksum failures, abandoned
  temporary writes, symlink rejection, oversized/deep receipts and write failures.
- Native gateway tests cover authorization, endpoint pinning, forwarding, cache
  independence, report identity/state validation and preserving the recovery handle
  when an Agent response is lost. The gateway explicitly enables lossless floating-
  point JSON parsing, independently of features enabled by other workspace crates.

Results: protocol 142 passed; Agent binary unit tests 199 passed; durable Agent
live suite 11 passed (including two shared-fixture checks); existing exact-cancel
live suite 4 passed; existing lifecycle live suite 6 passed; desktop-runtime unit
tests 62 passed and 1 pre-existing ignored test. Total: 424 passed, 0 failed,
1 ignored. The ignored test would start the workstation's real local stack and
was intentionally not enabled. Clippy for all three changed crates and all their
targets passes with warnings denied. Rust formatting, documentation book/inventory,
the organization audit and tensor self/full checks pass. The overall tensor still
has 4 maturity gaps, 20 evidence-grade gaps and 15 P0 gaps; its release status stays
blocked. This scoped recovery evidence does not erase unrelated gaps.

Reproduce from repository root:

```text
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol --lib
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-cli
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test agent_durable_execution_live --test agent_execution_cancel_live --test agent_lifecycle_live
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-desktop-runtime --lib
```

## Deliberate Boundaries

Not qualified: physical power loss, storage hardware caches, Windows directory
durability, remote Linux, or packaged/installed GUI acceptance in this round.
File and Unix directory sync strengthen persistence but do not prove those cases.

This is retained execution evidence, not a mid-solver numerical checkpoint or
automatic resume/replay mechanism. This initial round used synchronous calls;
[the follow-up round](agent-durable-async-20261013.md) adds asynchronous admission
and frontend recovery interfaces. Reconnect UI remains follow-up work. The gateway suite uses controlled
TCP peers, while the Agent suite uses the actual solver executable; this is not
a new full GUI-to-Agent installed acceptance run.

The ordinary GUI/PWDT solve route, legacy in-memory Agent TaskIR cache and Orchestra
recovery store are unchanged. Installer does not yet provision this opt-in directory
by default. Full graph execution, transport security parity, SDK convenience
wrappers, and disk-backed large-result chunking remain separate work. Execution
IDs/checksums are not authentication; keep the raw Agent RPC behind trusted access.
