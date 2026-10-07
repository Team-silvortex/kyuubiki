# Headless Native CLI Artifact Path Protection

Date: 2026-10-07. Base checkout: `7276aa6c` (subject `daji 3.5.1`).
Actual source/package metadata remains daji 3.5.0; installed baseline is 3.4.0.
This follow-up preserves preceding uncommitted report compaction/publication
work. No version relabeling, App rebuild, installation, commit or push is done.

## Repair

Two new regression cases first failed against the earlier CLI: a report could
replace its source workflow, and differently spelled report/material output
paths could target the same file. The additional file-as-parent output case also
failed before its repair. All experiments used disposable local fixtures.

The native example CLI now reserves command-scoped artifact paths before input
decoding, patch application/receipt output or execution. Each `run` output
(run report, material report, round evidence, parameter receipt) is checked
against the source workflow, patch, round specification and previous evidence,
then against all earlier outputs. Duplicate outputs and output-file ancestors
are rejected in either order. Every declared output in that command scope is
reserved even when a later option validation would make it inactive.

`render`/`plan` include their `--out` and parameter receipt; `inspect`/`validate`
include their receipt. The read inputs can alias each other; only writes are
restricted. Ignored options outside a command's scope are not globally reserved.
Distinct regular writable reports remain replaceable. `init`, without consuming
inputs or multiple output artifacts, retains explicit replacement behavior.
Independent SDK consumers choose their own storage policy.

Relative/absolute paths and dot segments resolve through the nearest existing
canonical ancestor without creating directories. Existing symlink parents are
resolved before missing-tail normalization. Existing Unix device/inode identity
detects hard-link aliases. macOS/Windows additionally reserve ASCII case-only
variants conservatively, including output-file ancestor relationships. Unresolved
symlinks, non-directory parents and other identity-resolution errors fail closed.

Initial rejection reports `output_path_conflict`, `command_validation`, and
`retryable: false`. JSON `run` stdout retains an invalid zero-step report with
workflow identity `unresolved`, because decode has not occurred. No file output
is written, including a distinct failure-report destination. Error classification
prioritizes this prefix over research/patch/timeout words in file names. Recovery
advice never authorizes automatic replay.

Every guarded file writer revalidates the complete command-scoped path set before
staging and after encoding/flushing/syncing/closing, immediately before replacement.
A newly observed alias rejects publication and uses the existing best-effort
owned-staging cleanup. When calculation already completed, the existing output
failure path retains the available actual run receipt on writable JSON stdout.
No algorithm or execution authority moves from Solver/Engine/Agent into this layer.

## Tests

Twenty-seven new tests cover twelve path unit cases, one error-priority case,
twelve CLI path cases, one publication-fixture isolation case and one
actual-service case. Loops additionally exercise:

- Every combination of four run outputs and four protected read inputs, with
  byte-for-byte checks of all original files and an existing safe report.
- All six pairs of run outputs, before any parameter receipt or directory creation.
- Both output-file ancestor orders, without rejecting sibling name prefixes.
- Workflow/patch/output/receipt conflicts across render and plan; receipt-input
  conflicts across inspect, validate, render, plan and run.
- A malformed workflow cannot be replaced by its own validation failure report.
- Successful distinct transform/report/receipt replacement without input mutation.
- Relative/absolute spellings, missing-tail dot normalization, symlink-parent
  traversal, Unix hard links, dangling parents and conservative ASCII case aliases.
- A test-only serializer creates an input hard-link alias during encoding.
  The second publication guard retains both original files and cleans its staging.
- A controlled HTTP service creates a hard-link alias while processing the one
  explicit health request. The CLI rejects report output but retains successful
  one-step stdout and original source/alias bytes, without replaying the request.
  This is a service-output boundary check, not scientific solver qualification.

A repeated full run also exposed a test-fixture race: time-only scratch names
collided between concurrent publication tests. An initialization failed at rename
with `ENOENT`; changing fixture creation to exclusive creation reproduced an
`AlreadyExists` error for the same generated name. Both new CLI fixture suites
now include atomic sequence IDs and exclusive directory creation. A fixed-time
regression proves independent files and cleanup even with identical timestamps.
This repairs test isolation, not a material algorithm or output-publication bug.

The actual-service fixture starts temporary Orchestra/SQLite and two Rust Agents,
creates one baseline project and saved model through the SDK, then invokes a CLI
workflow containing a project mutation followed by a saved-version solve. Source
overwrite and duplicate output configurations both fail before the first step.
Original workflow bytes and public project/job records remain unchanged; no output
directory is created, no job is stored, both Agents remain accepting/idle, and
their combined calculation count is zero. Owned processes and data are cleaned.

## Verification

Final macOS ARM64 source regression after fixture isolation repair:

- Headless SDK: 445 unit tests in 34.19 s plus 25 integrations, all passed.
- Protocol: 114 unit tests plus seven integrations, all passed.
- CLI binaries: 182 and 41 unit tests, all passed.
- Complete actual-service suite: 45/45 in 20.39 s, all passed.
- Selected CLI integrations: surface 1, execution posture 16, output paths 12,
  output publication 7, parameter patch 2, research round 3, task completion 5
  and wait recovery 1, all passed.
- Publication integration suite additionally passed 50 consecutive runs with
  four test threads, seven tests per run. This is repeated regression, not a
  throughput benchmark or proof that unrelated fixtures cannot collide.
- CLI/SDK/Protocol test-target Clippy passes with warnings denied; Rust formatting
  and diff whitespace checks pass. Full selected targets have no ignored/filtered
  tests; earlier isolated diagnostic runs intentionally filtered other tests.

Runtime API, topology, matrix, extension-standard, tensor self-test/validation,
book/inventory, organization and version-line checks pass. The tensor retains
zero structural gaps, four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps;
Daji qualification remains blocked. No grade is promoted. Source/document limits
remain 800/2000 with zero tracked organization debt. All 224 exact version
contracts match actual source metadata 3.5.0, independent of the commit subject.

The earlier full rerun with the fixture collision failed and was not counted as
a successful acceptance run; the final command below completed after its repair.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery --test headless_output_publication --test headless_output_paths -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

These are local macOS ARM64 source regressions, not Linux/Windows, installed App,
authenticated/durable deployment, Python/Elixir storage parity or scale acceptance.
No new public SDK API or protocol schema version is added. Evidence is `verified`
execution/contract/recovery only; maturity and qualification are not promoted.

Reservations are path checks, not file locks, content hashes, input immutability
or a hostile-directory sandbox. Files and symlinks can change after the last
check; the deterministic second-check test does not eliminate TOCTOU races.
Prospective Unicode normalization/case folding, Windows hard-link identity and
special/network filesystem equivalence are not qualified. Conservative ASCII
case checks can reject distinct names on a case-sensitive macOS/Windows volume.

Multi-file publication is not transactional, durable across restart/power loss
or globally cleaned after forced termination. Distinct artifacts can publish
before later failures. Available stdout may be incomplete if its consumer fails;
inspect owning service records without automatic recomputation. No backups,
directory scans, destructive cleanup, full input reads for path comparison,
new dependency, throughput claim or total RSS/CPU guarantee is introduced.
