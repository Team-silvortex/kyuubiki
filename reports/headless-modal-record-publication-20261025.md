# Research Record Publication And Process Interruption

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [local recovery preflight](headless-modal-local-preflight-20261025.md).
This changes the example's record writer, not Solver numerics, Engine/Agent
execution policy, SDK wire contracts or installed services.

## Reproduced Window

The previous writer created a final filename before writing its JSON bytes.
An interrupted admission write after real dispatch could leave a partial final
`admission.json`. Strict recovery correctly rejected that corrupt record, but
could not use the intended missing-acknowledgement reconciliation path.

The old writer was extracted without changing its behavior, and a deterministic
partial-write failure regression demonstrated that a final partial file remained.
The test failed before the publication repair. It does not imply that existing
corrupt final records can safely be ignored; those remain rejected.

## Record Protocol

Only `input.json`, `submission-intent.json`, `admission.json` and `receipt.json`
use this example-local writer. Existing final paths, stale pending paths and
symlink containers are not overwritten. The encoded 8 MiB record limit matches
the existing reader policy; this is not a total serialization-memory limit.

1. Create a new same-directory `.NAME.pending` file exclusively.
2. Write and synchronize its complete encoded bytes, then close the file.
3. Read through the bounded evidence reader and require exact byte equality.
4. Publish the final name with a no-replace hard link, then remove the pending alias.

Ordinary failures clean only newly owned pending files. A publication collision
preserves the competing final record. A pre-existing pending record is never
reused or deleted by a failed attempt. Unsupported hard-link operations fail
closed; there is no fallback to an overwriting rename. A cleanup failure after
publication is reported explicitly as published-but-alias-cleanup-failed.

Process termination can leave a partial/full pending record, or a complete final
record with a pending alias. Only the final record is evidence. No automatic
promotion, repair, cleanup of old pending data or retry is authorized. The
successful path leaves no alias or extra backup; a killed attempt retains one
bounded diagnostic pending file. General disk cleanup policy is unchanged.

This does not synchronize parent directories or provide host-power-loss,
adversarial-filesystem or general multi-writer transactional guarantees. Local
hard-link support is required and this run verifies only the macOS filesystem.

## Forced Termination Evidence

A test executable calls the same record writer and public Rust SDK as the
example. Its test-only writer pauses after a partial or complete file write;
the parent kills it with `Child::kill`, not a graceful shutdown. No pause
environment variable is added to the shipping example. Fresh recovery then
uses the ordinary example executable and real native Agent.

- Partial intent write: only a pending intent exists; no task was dispatched in
  this controlled run. Inspection rejects missing final intent without export.
- Complete intent before dispatch: the original Agent returns `not_found`;
  inspection retains only diagnostic receipt, not a verified result or new task.
- Partial admission after dispatch: no final admission exists. The original
  committed task survives Agent kill/restart and fresh inspection verifies it.
- Complete staged admission before publication: recovery likewise reads the
  original task, not the pending file. It reports admission unavailable and
  leaves pending bytes unchanged rather than promoting them.

Both post-dispatch cases also pass offline verification. Store counts and
original receipt equality show no resubmission; submission-directory snapshots
remain unchanged. Missing local evidence alone does not prove that an arbitrary
task was never sent: zero dispatch is established only by the controlled test
ordering and real Agent observation, not inferred as a general recovery rule.

Seven ordinary writer tests cover exact bytes, partial write cleanup, staged
corruption, publication collision, stale pending preservation, size/name limits
and Unix link handling. They include a complete final file with a leftover
pending alias. Existing corrupt-final-admission and original-store tests remain.

## Verification

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

The focused target passed 26 opt-in live tests plus regular SDK/protocol checks.
The formal `modal-sdk-durable-research` validation profile executed this target
again successfully (`executed=true`), without warnings. The full SDK all-target
run passed 181 tests, with zero failures and 26 opt-in tests ignored in that run;
those 26 were executed separately by the focused target. The 181 includes two
child-fixture entries that are inert without their test-only context. Overlapping
runs are not additive. All-target Clippy passed with warnings denied.

Documentation book and inventory checks, targeted Rust formatting and
`git diff --check` passed. The project organization audit passed with the
800-line source / 2000-line documentation limits and zero tracked debt.
The tensor structure check passed with zero structural gaps; it still reports
2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps, with overall daji status
`blocked`. This does not imply those unrelated gaps are resolved. This coordinate
remains bounded `verified`, not `qualified`. No package bump, install, Git commit
or push is included.
