# Headless Native CLI Output Publication

Date: 2026-10-07. Base checkout: `7276aa6c` (subject `daji 3.5.1`).
Actual source/package metadata remains daji 3.5.0; installed local baseline is
3.4.0. This source follow-up preserves the preceding uncommitted report-compaction
work. No version relabeling, App rebuild, install, commit or push is performed.

## Repair

The native Headless example CLI previously created a complete encoded JSON byte
vector/string before file/stdout output. File writes directly truncated existing
reports. A subsequent write failure could destroy the old report and return before
printing the new run receipt, even though calculation had already completed.

A scoped output module now streams pretty JSON through a 64 KiB buffer. File
publication exclusively creates a temporary file in the selected directory,
encodes, flushes, syncs, closes and renames it over the destination. It never
deletes the previous report first and retains no old-report backup. Normal errors
before publication leave committed output intact; owned staging is cleaned on a
best-effort basis after its file handle closes. Unix output uses `0600` mode.
Symlink, special-file, directory and read-only destinations are rejected, including
a second destination check before replacement. These checks do not make an
untrusted parent directory into a sandbox.

This applies to Headless `init`, `render`, `plan`, run/preflight and material
reports, parameter-patch receipts and research-round evidence. Other example
CLIs and independent SDK consumers are not silently changed. File formatting
retains the previous pretty JSON bytes; only stdout adds its existing newline.
Typed-report comparisons use the same original value, avoiding false failures
from re-created HashMap iteration order after deserialization.

Run/material/round file failures retain the available run receipt on writable
stdout and exit nonzero with a separate `report_output_failure` diagnostic,
stage `artifact_output`, and `retryable: false`. The actual run state is not
changed from successful calculation to failed calculation. Preflight report-file
failure likewise retains the original validation receipt. Parameter-receipt
failure stops before dispatch with an invalid preflight report and explicit
output recovery advice. The public failure receipt schema additively declares
`artifact_output`; its schema version and existing enum values stay unchanged.
Output-prefix classification takes precedence over path/label text that could
otherwise look like a parameter, research or timeout error.

Closed JSON stdout returns an output error through the existing stderr envelope
instead of a printing panic. This cannot guarantee complete stdout when the
consumer closes its pipe. No output error authorizes automatic step/batch replay.
No numerical algorithm or execution authority moves into the output layer.

## Tests

Twenty new tests verify eleven writer cases, one diagnostic-priority case, six
CLI publication cases, one round-evidence failure and one actual-service recovery:

- Pretty JSON equality for large escaped multilingual values and typed run
  reports, replacement without sidecars and relative Unicode output paths.
- Serialization failure after a large partial output, injected failure after
  bytes reach staging, failed first publication and changed destination cleanup.
- Four concurrent writers publish complete JSON documents through distinct
  staging files. Publication order is last-successful-write, not compare-and-swap.
- Private Unix permissions, read-only-file protection and rejection of symlink,
  special-file, directory or invalid-parent destinations without modifying data.
- Dry-run/preflight receipts survive report-file failure. Material-output failure
  preserves the completed mock run and its already published run report; this
  mock case checks output plumbing, not physical qualification.
- Parameter-receipt symlink rejection stops before service connection and retains
  the original file. Round-evidence failure follows one controlled service call
  and preserves its metric/run report instead of retrying the request.
- Closed stdout yields a non-retryable structured output diagnostic, not a panic.

The actual-service fixture starts temporary Orchestra/SQLite and two Rust Agents.
One explicitly submitted saved-version bar calculation completes with 513 nodes
and 512 elements; tip displacement independently satisfies `FL/EA` to relative
`1e-12`. Report publication is then rejected by a directory destination.
JSON stdout still contains the completed job identity and one successful step.

An explicitly requested two-step `job_fetch`/`result_fetch` workflow reads the
same job and publishes a recovery report. Stored job/result values remain
unchanged, the recovered file equals JSON stdout, the database contains one job,
and the combined Agent calculation count remains one. Both Agents return to
accepting/idle. Owned service processes/data are cleaned up. This is read-only
calculation recovery, not exactly-once execution or a restart experiment.

## Verification

Final macOS ARM64 source regression:

- Headless SDK: 445 unit tests in 33.96 s plus 25 integrations, all passed.
- Protocol: 114 unit tests plus seven integrations, all passed.
- Complete actual-service suite: 44/44 in 23.43 s, all passed.
- CLI binaries: 182 and 28 unit tests; selected integrations: surface 1,
  execution posture 16, output publication 6, parameter patch 2, research round 3,
  task completion 5 and wait recovery 1, all passed.
- CLI/SDK/Protocol test-target Clippy passes with warnings denied; Rust formatting
  and diff whitespace checks pass.

Runtime API, topology, matrix, extension-standard, tensor self-test/validation,
documentation book/inventory, organization and version-line checks pass. The
tensor has zero structural gaps, four maturity gaps, 19 evidence-grade gaps and
14 P0 gaps; Daji qualification remains blocked. No evidence grade is promoted.
Source/document limits remain 800/2000 with zero tracked organization debt.
All 224 exact version contracts match actual source metadata 3.5.0, independently
of the base commit's 3.5.1 subject.

Full selected targets have zero ignored/filtered tests. Test durations do not
measure throughput improvement. Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery --test headless_output_publication -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

This removes the additional complete encoded document from ordinary report output,
not caller values, parsed responses, report construction, required mirrors,
concurrent branches or the compact stderr diagnostic. It is not a total report
byte/RSS cap or an OOM/CPU guarantee. File syncing adds I/O cost; no deployed RSS
or throughput number, aggregate memory bound or scale qualification is claimed.

Single-file replacement is not a multi-artifact transaction, durable checkpoint,
directory durability, power-loss guarantee or hostile-directory sandbox. A killed
process can leave staging; cleanup can fail and newly created parent directories
can remain. No global cleanup, crash journal or hidden backup scheme is added.
Readers must treat missing independently published artifacts as missing evidence,
not as permission to replay computation. If stdout also fails, it can be partial;
use the owning service's existing task/result records for explicit inspection.

Linux, Windows, installed GUI/Agent, independent Python/Elixir clients, authenticated
durable deployments and general scientific/scale qualification remain separate.
Evidence is native `verified` execution/contract/recovery only, not `qualified`.
