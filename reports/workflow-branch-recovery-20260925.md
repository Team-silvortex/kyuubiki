# Local Workflow Branch Recovery Contract

Date: 2026-09-25

This is bounded local branch-recovery evidence, not distributed execution or industrial physics qualification.

## Scope

The prior reporting parity round left Elixir graph execution fail-fast-only.
This follow-up aligns explicit branch-local recovery with the existing Rust
engine policy while keeping numerical failure distinct from a valid blocked
guard decision. No version bump, release rebuild or deployment is part of this
change. Earlier uncommitted reporting and diagnostic integrity work is retained.

## Changes

- Validate both declared `config.on_error` and `config.recovery.on_error` before
  execution. Only exact `skip`/`fail` strings are valid. Top-level policy takes
  precedence; an invalid nested value cannot hide behind a valid top-level one.
- Handle returned node errors only when `skip` is enabled. Record a failed node
  and its reason, publish no failed artifacts, and resolve blocked descendants
  as skipped. Independent work continues. Default behavior remains fail-fast.
- Preserve declared incoming edge order. A partial-input fallback waits for
  upstream completion/failure/skip before selecting its first surviving input.
  This matches the existing Rust topological execution policy, not a race.
- Preserve `failed_nodes` and `node_failures` in full, compact, custom and
  automatic-compact responses. Include failed timing in performance summaries.
- Emit terminal progress for successful, failed and skipped nodes. Async
  persistence uses the resolved total without mislabeling failed work as
  successful. Completion messages warn when failures remain.
- Keep exceptions, process exits and workflow cancellation outside Elixir's
  branch-skip handler. This is not a promise of exception-boundary parity with
  the Rust panic boundary.

## Reproduction

The profile runs the storage-independent Elixir tests plus shared Rust graph
tests and existing Rust recovery regressions:

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-branch-recovery-contract --out tmp/workflow-branch-recovery-contract.json
```

Run API and lifecycle regressions from `apps/web` with `SQLITE_DATABASE_PATH`
pointing to a newly created, isolated test database, never an existing developer
database:

```text
mix test test/kyuubiki_web/api/workflow_branch_recovery_api_test.exs test/kyuubiki_web/api/workflow_reporting_integrity_api_test.exs test/kyuubiki_web/orchestra/workflow_restart_recovery_test.exs
```

The shared policy fixture is `tests/fixtures/workflow-recovery-policies.json`;
the common reporting graph is `tests/fixtures/workflow-report-contract.json`.
Both engines exercise 20 policies against six reporting faults in two node
orders (240 scenarios each). Assertions compare failure identity/path, output
absence and surviving evidence, not byte-identical diagnostic text or trace
ordering. Fallback, exhausted-input and corrected-rerun controls are additional.

## Validation

The initial 24-test Elixir reproduction produced nine failures against the old
runner, including opt-in recovery, fallback ordering, compact receipts and
terminal progress. Tests were expanded during implementation to include
preflight rejection, automatic compact mode and async persistence.

Final validation results:

| Check | Result |
| --- | --- |
| Focused native validation profile | Passed: 33 Elixir tests, 3 shared Rust graph tests, 7 existing Rust recovery tests |
| Shared recovery policy scenarios | Passed: 240 executions per runtime, in addition to fallback and replay controls |
| Broad Elixir workflow suite | 349 tests passed across 57 files, excluding benchmark tests |
| Focused API and existing application-restart suite | 12 tests passed; also included in the broad run |
| Rust release shared branch-recovery suite | 3 tests passed |
| Elixir test-environment compilation | Passed with warnings treated as errors |
| Rust engine all-target Clippy | Passed with warnings treated as errors |
| Operator profile structure | 55 profiles passed validation; only the focused profile was executed in this round |
| Coverage tensor and checker self-tests | Passed structurally; 13 modules, 11 paradigms, zero structural gaps |
| Repository organization and changed-file formatting | Passed; source limit 800, docs limit 2000, tracked organization debt zero |

Counts overlap across the focused and broad suites and must not be added as
unique tests. Two older guarded-workflow assertions expected progress only for
seven successful nodes; they now explicitly verify seven successful events,
the skipped-node identities and full terminal progress instead of hiding skips.
No physics assertion or failure acceptance criterion was weakened.

The tensor remains blocked with four maturity gaps, 16 evidence-grade gaps and
11 P0 gaps. The new claim is scoped local `verified` evidence, not an operational
promotion. Focused machine-readable results are disposable under `tmp/`;
the isolated API/broad-test databases were removed after the test VMs exited.

## Boundaries

- A completed job means the run reached a terminal scheduling state. Callers
  must inspect `failed_nodes`, `node_failures` and scientific guard decisions;
  completion does not certify a usable report.
- No artifacts are published for a rejected node in the tested in-memory graph
  execution. This does not roll back arbitrary external side effects of custom
  operators and does not authorize automatic retries.
- API tests use the local Plug router and an isolated SQLite result store, not
  a remote server. Existing restart regressions restart the local application;
  they are not machine-loss or distributed recovery qualifications.
- The async and compact failure receipts are Elixir response additions. Rust
  already retains failure IDs and per-node errors; its full response shape,
  memory accounting and panic semantics are not declared equivalent.
- Installed GUI presentation, remote Agent recovery, full graph-security
  preflight parity and dedicated domain-diagnostic reducers remain separate
  audits. Do not promote broad tensor maturity from this bounded claim.
- No performance benchmark, release packaging, commit or push is included.
  Temporary test databases are disposable; no developer data is migrated.
