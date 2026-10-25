# Recovery Local Preflight

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [modal result metadata validation](headless-modal-metadata-validation-20261025.md).
This changes validation order in the Rust headless SDK and research example,
not Solver numerics, execution/replay policy or installed services.

## Reproduced Gaps

- History-aware SDK fetch checked terminal payload depth only after remote
  observation. A locally unusable retained receipt could first produce an I/O
  timeout, concealing the actionable local error.
- Online example inspection fetched before reading saved admission. Invalid
  JSON or an admission already inconsistent with the prepared intent could
  therefore contact the remote endpoint and leave diagnostic output first.
- Submission and inspection contacted the remote service before rejecting
  occupied output paths or impossible parents.

One ordinary SDK regression and two isolated native-process regressions failed
before the repair. The SDK test received a timeout instead of its depth error;
both example tests recorded an unwanted loopback connection. A real Agent
produced the original task and receipt; fault injection changed only test-owned
local evidence and probe endpoint configuration, never the real Agent ledger.

## Repair

The SDK computes the retained terminal commitment before connecting and reuses
it after the single fetch. Invalid identity or excessive retained result/error
depth fails locally. Valid observations retain their existing identity, terminal
immutability and no-replay checks. Each compared terminal payload is hashed
once; no new persistent field, global cache or performance benchmark is claimed.

The example reads and validates present admission against the prepared intent
before remote endpoint configuration/fetch and before offline receipt reads.
Errors identify `admission.json`. Truly absent admission retains the original
intent-based recovery path. Local validity does not establish agreement with a
future remote observation: later conflicts are still rejected after fetching,
and may leave a raw diagnostic receipt but never a verified result.

A small read-only destination preflight rejects existing files/directories,
dangling links and absent/non-directory parents before networking. Relative
single-component destinations resolve their parent as the current directory.
No directory is created by preflight, and parents are not created automatically.
Final exclusive directory creation and publication checks remain authoritative.
This does not reserve paths, test all write permissions or defend against hostile
filesystem races; a later filesystem change may still produce an error after
a remote read, without overwriting existing evidence.

## Regression Scope

- SDK completed and failed retained records with excessive content depth fail
  before the probe listener sees a connection; original failure semantics stay intact.
- Invalid/duplicate JSON, null admission, foreign store and inner-request mismatch
  fail before contacting the probe, before a deliberately invalid endpoint setting,
  and before offline receipt consumption. Admission files are unchanged.
- A directory masquerading as admission is rejected without connection/export.
- Existing file/directory, missing parent, file parent and dangling destination
  link are rejected before networking for both `submit` and `inspect`.
- A truly absent admission reaches exactly one probe read; a fresh real-Agent
  inspection then succeeds without resubmission, using absolute and relative
  output paths. The original receipt and single stored execution are preserved.

No real application runtime is stopped. Probes use isolated loopback ports,
and tests clean their own temporary directories on success. This is bounded
local source-built evidence, not remote/installed or host-power-loss qualification.

## Verification

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

The focused target passed all 24 opt-in live tests plus regular SDK/protocol
checks. The full SDK suite passed 173 tests, zero failures, with 24 live tests
ignored by default and executed separately. Counts include the existing no-op
child fixture; overlapping runs are not additive. All-target Clippy passed with
warnings denied. The final native `modal-sdk-durable-research` profile passed
with `executed=true`, including the relative-path positive control.

Documentation book/inventory, targeted Rust formatting, project organization
and `git diff --check` passed. Source/document limits remain 800/2000 lines with
zero tracked organization debt. Tensor structure passed with zero structural
gaps; two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps remain. Its evidence
grade stays bounded `verified`, not `qualified`. No package bump, install, Git
commit or push is included.
