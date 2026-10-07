# Headless Contract Risk Gate

Date: 2026-10-07. Base checkout: `76aaf10d`, preserving the preceding native
binding, cancellation and original-receipt repairs. Imported native execution
batches can no longer override action risks to bypass confirmation. All risk
metadata is validated before any batch step, and plans display contract-owned
approval requirements even when the imported metadata is invalid.

## Contract Repair

Test-first regression reproduced three failures: alternate risk labels passed
validation, an earlier write ran before a malformed later step, and a plan
accepted imported labels as its confirmation policy. The positive independent
confirmation-flag matrix already passed. After the repair all four tests pass.

`validate_batch` now requires each supported action's `risk` to equal its
registered contract. This includes rejecting stricter labels and swapping
`sensitive` with `destructive`; those two permissions are independent, not a
risk ranking. Workflow normalization already derives risk from the contract.
Callers wanting stricter approval should apply a separate application policy,
not rewrite serialized contract metadata.

Whole-batch validation happens before the executor or dry-run begins. A mismatch
returns `invalid`, zero executed steps, no per-step receipts, and a non-retryable
`kyuubiki.headless.document_validation` failure at `batch_validation`. Allowing
both risk flags cannot authorize malformed metadata. `build_execution_plan`
uses registered risks for its step and confirmation lists, consistently with
the contract-derived policy summary, while retaining `ok: false` and the actual
validation issues. No engine, solver or service-authentication behavior changed.

`risk_contract_tests.rs` exercises the risk invariant for all 74 current action
contracts, including each of the two alternative labels. Payload-specific
validation remains separate in that enumeration. Representative valid batches
also cover JSON import, all four flag combinations, whole-batch side-effect
isolation, dry-run, diagnostic plan/policy agreement, and independent sensitive
and destructive confirmation. Browser snapshot is covered at native batch/unit
level, not through an actual browser session.

## Real Service Chain

`imported_delete_risk_cannot_bypass_confirmation_or_run_earlier_work` owns an
isolated Orchestra, SQLite database, source Agent, peer Agent and scratch root.
Its only deletion target is a project created by that test in temporary storage.
The batch contains a real bar TaskIR, a project write, and a project deletion.
Changing the deletion risk to either `normal` or `sensitive` makes the entire
batch invalid for every confirmation-flag combination before any executor call.

The actual CLI repeats both malformed JSON imports with both allow flags.
Each exits nonzero, emits the same invalid report written to `--report-out`, and
retains a non-retryable `batch_validation` failure. The project listing is
unchanged, both Agent execution counters remain zero, and no task-dispatch
journal is created. No fake service reply or production fault switch is used.

The caller then explicitly builds a corrected one-step deletion batch, without
the unused source calculation or earlier write. Both no approval and sensitive
approval remain blocked through the SDK, and sensitive-only CLI execution also
exits nonzero without changing storage. Explicit destructive SDK approval
performs exactly one deletion. The final listing equals the original listing
minus that owned target; both Agents still have zero computations. This is not
an automatic batch replay, recovery decision, or authorization grant.

## Verification

Final macOS ARM64 source checks:

- Complete `headless_live`: 26/26 passed in 49.21 seconds.
- Complete Headless SDK: 321 unit tests in 31.35 seconds and 25 integrations passed.
- Complete Protocol: 114 unit tests in 0.43 seconds and seven integrations passed.
- Native CLI binary: 182 units in 0.92 seconds and `headless_task_completion`
  five integrations in 0.85 seconds passed.
- CLI/SDK/Protocol test-target Clippy passed with warnings denied; formatting
  and whitespace checks passed.

Remote Ubuntu x86_64 source checks used an unprivileged disposable container
with Rust 1.88.0, Elixir 1.19.5/OTP 28, 8 CPUs, 8 GiB memory and 512 PIDs, four
Rust test threads and two Erlang schedulers. No installed service was restarted.

- Complete `headless_live`: 26/26 passed in 39.72 seconds.
- Complete Headless SDK: 321 unit tests in 31.29 seconds and 25 integrations passed.
- Complete Protocol: 114 unit tests in 0.39 seconds and seven integrations passed.
- SHA-256 matched 17 selected final source/support/schema/lock files. Cargo and
  Mix lockfiles were unchanged. The test container was removed, its 452 MiB
  temporary directory deleted, and directory absence verified.

All complete targets listed above had zero ignored or filtered tests. Timings
are correctness-run durations, not throughput or benchmark claims. Documentation
and the coverage evidence register only verified native source-chain behavior.

Runtime API contracts, module topology/matrix, tensor self-test and actual tensor
check, documentation book/inventory and project organization audit passed. Source
and documentation limits remain 800 and 2000 lines, with zero tracked debt. The
tensor retains four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps; release
status is still blocked. This repair does not erase outstanding qualification.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol
cargo test --quiet --locked --offline -p kyuubiki-cli --test headless_live -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --test headless_task_completion
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Scope

The confirmation guarantee is for native batch/CLI execution, not direct
low-level `execute_step` calls or server authentication. It does not qualify
standalone Python/Elixir/Rust SDK clients, browser sessions, installed App
behavior, durable recovery or replay deduplication. Matching risks alone cannot
prove a payload or solver result valid; all existing binding, model, TaskIR and
completion gates remain necessary. This is verified evidence, not installed or
operational qualification. No release metadata, commit or App bundle changed.
