# Offline Recovery Of Interrupted Modal Exports

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [record publication](headless-modal-record-publication-20261025.md).
This changes the official Rust SDK research example, not Solver numerics,
Engine/Agent execution policy, a generic headless CLI or the SDK wire contract.

## Gap And Reproduction

A complete original receipt could survive a killed exporter while formal
`verified` publication had not happened. Previously `inspect` required an Agent
to read that task again, and `verify` required a fully published export. There
was no explicit offline route from the intact retained receipt to a fresh export.

The existing real-Agent export interruption regression was extended: kill the
test exporter after staged result write, stop the Agent, then invoke the ordinary
example's offline recovery command. Before implementation it failed at the old
command dispatch. This is a missing workflow capability, not a new numerical
failure or evidence that incomplete files should be trusted.

## Explicit Offline Operation

```sh
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example durable_modal_research -- recover-export tmp/modal-study-001 tmp/modal-export-001 tmp/modal-export-recovered
```

The caller supplies the original submission directory, an unpublished export
directory containing a complete final `receipt.json`, and a new output directory.
The output parent must exist. Recovery uses the same input hash, typed intent,
optional-admission consistency, terminal state and independent physical checks
as the existing research path before creating output. It then retains the
checked receipt and uses the established staged/readback publication protocol.
The receipt copy is reserialized; source bytes are untouched and result values
remain bound to the typed completed receipt.

- No Agent connection, endpoint environment parsing, computation replay or retry.
- No source repair, deletion, pending-file reuse or promotion.
- No output inside either source tree; canonical parent checks include aliases.
- No replacement of an existing output. Final exclusive directory creation still
  decides ownership; the earlier path check is not a reservation.
- Any existing `verified` entry in the source blocks recovery, including an empty
  directory, damaged file or dangling link. Use `verify` on published evidence.
- Only a valid completed receipt can produce results. Missing/partial, failed,
  running, unknown, foreign and contradictory records stop before output creation.

No final receipt means no offline recovery; an explicit online inspection can
read the original task when available, but cannot authorize automatic replay.
Offline recovery does not authenticate caller-owned records or check the current
remote ledger. Existing single-writer and non-hostile-filesystem assumptions,
local hard-link requirement and lack of host-power-loss qualification remain.

## Evidence

The existing killed-exporter test now recovers into a fresh directory while the
Agent is still stopped, verifies that export offline, and checks unchanged raw
receipt and pending result bytes. The Agent is restarted only afterwards; the
original receipt remains identical and the durable store still has one task.

Three new opt-in live tests additionally cover:

1. Real submission and completed receipt, stopped Agent, malformed endpoint
   environment and a connection probe on the original port. Recovery and offline
   verification succeed with present or genuinely missing admission. No connection
   is received, source snapshots are identical, and an occupied output is preserved.
2. Failed/running/unknown/missing statuses, foreign receipt, invalid physical
   metadata, absent final receipt with pending alias, partial/duplicate-key/oversized
   JSON, corrupt admission, terminal conflict and changed input. No new output
   is created and source snapshots stay unchanged.
3. Existing empty/corrupt/file/dangling-link publication entries, occupied output,
   destination within submission or source, and a symlink alias to the source.
   None can bypass the source-preservation/publication gates.

The fixture captures store identity before dispatch; an Agent diagnostic snapshot
during active execution is not used as the source of retained recovery identity.
Actual forced termination remains in the existing test executable; no pause hook
is exposed by the shipping example. All test services are isolated development
processes, not installed applications or remote deployment.

## Validation

- `make test-sdk-modal-recovery`: passed, including 29 opt-in live tests.
- The `modal-sdk-durable-research` validation profile executed that target again
  successfully, with `executed=true` and no warnings.
- SDK all-target tests: 181 passed, zero failed, 29 ignored in that invocation;
  those 29 were executed separately above. Two of the 181 are inert child-fixture
  entries unless invoked with their test-only context.
- SDK all-target Clippy with `-D warnings`: passed.
- Documentation book/inventory, targeted Rust formatting, project organization
  and `git diff --check`: passed. Source/document limits remain 800/2000 lines.
- Tensor structure: passed, zero structural gaps. Overall daji status remains
  `blocked`, with 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

Runs overlap and must not be added as independent coverage. Tensor evidence
remains bounded `verified`, not `qualified`; unrelated acceptance gaps are not
closed by this example. No version bump, Git commit/push, installation or
deployment is included.
