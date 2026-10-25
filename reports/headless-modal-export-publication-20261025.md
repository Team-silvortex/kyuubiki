# Modal Research Export Publication Boundary

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [failure-context recovery](headless-failure-context-recovery-20261025.md).
This turn changes the official Rust SDK research example and tests, not Solver,
Engine, Agent execution policy, package versions or installed services.

This is bounded source-built SDK/Agent verification, not installed or general modal qualification.

## Reproduced Defect

The example previously wrote `result.json` before checking the result's physical
validity. A regression obtained a real native Agent's four-mode result, altered
the first eigenvalue in a test-owned receipt copy, and passed it through the
actual example publication code. Physical verification rejected it, but the
nominal result file remained. The test failed on that file's existence. The
Agent's original ledger and result were not changed by the fault injection.

This was an export/publication ordering defect, not evidence of an erroneous
solver result. Successful RPC or completed execution alone is insufficient to
qualify the numerical result against this example's independent reference.

## Publication Contract

The output root retains `receipt.json` as diagnostics. Formal results now live
under `verified/result.json` and `verified/verification.json`; consumers must
update paths from the old output-root layout. The example publication helper:

1. Validates the original completed execution handle and independently checks
   the physical result before creating staging data.
2. Writes the result into a new `.verification-pending` directory with file
   synchronization, then checks exact serialized bytes after disk readback.
3. Independently verifies the readback again and writes the verification report
   with original execution identity, result SHA-256 and scoped physical checks.
4. Checks verification-file readback and publishes both files with one
   same-parent directory rename. Existing formal exports are not replaced.

Consumers require both formal files, `passed: true`, original execution identity
and the result hash. A raw receipt, pending data or a directory name alone does
not qualify a result. This sample is not a new filesystem requirement for all
headless SDK consumers; embedding systems retain their own persistence policy.

Ordinary failures remove only the staging directory created by this call.
Pre-existing staging or exports are retained and block overwrite. A killed
export process may leave pending files, never a partially published verified
directory. The caller can inspect the original execution into a new directory
without solving again. No staging auto-repair, implicit retry or backup chain
is introduced.

## Fault Coverage

- Real result copies with a changed eigenvalue, missing modes or wrong total
  mass fail independent validation without publishing formal files.
- Staged-byte corruption, missing staged results and a blocked verification
  path fail without publishing. Owned staging is cleaned and raw receipts stay.
- An existing publication target introduced before commit blocks replacement;
  its sentinel is preserved. A pre-existing staging directory is not deleted.
- A healthy export is not overwritten by another call, including changed
  admission-availability metadata.
- A native test child pauses after writing its pending result. The harness
  forcibly kills it, confirms no verified output exists, kills/restarts the
  actual Agent, and verifies the original committed receipt is unchanged.
  A fresh process running the actual SDK example performs read-only inspection,
  publishes and independently validates the same result. The store still has
  one execution; the abandoned pending directory remains non-authoritative.

Fault hooks are local test/example module helpers. The production example has
no environment-triggered pause or corruption mechanism. The child-process
fixture does not run in normal application execution.

## Verification

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

The initial publication negative control failed on the old ordering. After the
repair, all 14 opt-in real-Agent tests passed: six admission-fault, five modal
research and three publication tests. The shared digest contract, 12 receipt,
three intent and 15 transport regressions also passed. The all-target SDK run
passed 149 tests with zero failures and 14 opt-in tests ignored by default.
The no-op child-fixture entry is included in 149, not a separate live journey.
Do not add overlapping focused and full-suite counts.

The native validation runner also executed `modal-sdk-durable-research` with
`executed: true` and `ok: true`. SDK all-target Clippy with warnings denied,
targeted formatting, diff whitespace, documentation book/inventory and project
organization checks passed. The organization audit found no tracked size debt
against the 800-line source and 2,000-line documentation limits.

The refreshed coverage tensor has zero structural gaps, two maturity gaps and
23 evidence-grade gaps. Overall release readiness remains blocked with 18 P0
gaps; this bounded publication repair does not close unrelated qualifications.

## Limits

The publication boundary assumes a caller-owned single-writer local filesystem
supporting same-parent directory rename. It is not an authenticated artifact,
hostile/concurrent-filesystem defense, network-filesystem transaction or
host-power-loss durability claim. File synchronization and process-kill tests
do not establish directory durability across a power failure. A process killed
after publication may already have completed the export; inspect it rather
than assuming a missing process acknowledgement means rollback.

The physical checker remains scoped to the two explicit synthetic model
orderings. No production tolerance or numerical algorithm changes, new broad
modal qualification, remote/installed acceptance or Python/Elixir parity are
claimed. Tensor evidence remains `verified`, not `qualified`.
