# Headless Native Report Compaction Ownership

Date: 2026-10-07. Base checkout: `7276aa6c` (subject `daji 3.5.1`).
Actual source/package metadata remains daji 3.5.0; installed local baseline is
3.4.0. This run started from a clean worktree and preserves the committed repairs.
No version relabeling or update of historical evidence is performed.

## Repair

The existing borrowed compactor already sampled oversized arrays and long strings
without copying their entire tails. The remaining report construction copied
object keys, short strings and small arrays even when the resolved payload or
unreferenced result was owned and no longer needed elsewhere.

Compaction is now a dedicated internal module with shared summary constructors
and thresholds for borrowed and owned values. Borrowed source inputs retain the
previous independent-copy behavior. Owned values transfer eligible short strings,
object fields/keys and small array buffers. Large arrays use a fresh three-item
sample vector, avoiding iterator allocation reuse that could retain the entire
source capacity. Small arrays/strings with unusually large reserved capacity
shrink before entering reports. Necessary shared previews are still independent.

Completed-result cache insertion returns the report preview while retaining full
original values only for later-referenced top-level outputs. Referenced outputs
receive independent compacted previews; unused fields move into the report.
The last binding use still moves its complete original into the dependent action.
Owned resolved payloads can then move into the report after dispatch; unbound
borrowed payloads are never mutated. Dry-run/local preparation follow the same
payload ownership rule, without changing their existing preview retention.

Failed, blocked, cancelled and unknown-status outcomes do not create successful
bindings or permit subsequent actions. Public report/result schemas, confirmation,
binding syntax, opacity, classification, retry/replay policy and summary values
are unchanged. Arrays remain complete up to 128 items and otherwise retain three
recursive samples plus counts. Strings remain complete up to 4096 UTF-8 bytes
and otherwise retain byte count and the first 256 characters, not bytes.

This is native SDK/report work, not a numerical implementation or engine protocol
change. Solver retains algorithms, and Engine/Agent execute admitted tasks.

## Tests

Thirteen new native SDK tests verify:

- Borrowed, owned and Cow payload paths match a test-only previous compactor
  across scalar, nested, array 127/128/129 and string 4095/4096/4097 boundaries,
  including Unicode byte-boundary cases and literal template-like strings.
- Eligible short-string, object-key and small-array allocations retain their
  original pointers; large-array samples preserve selected leaves without
  keeping the original 4096-item vector capacity.
- Overreserved small arrays and Unicode strings release excess capacity without
  losing values. Long multilingual summaries retain their exact byte count and
  a valid 256-character prefix.
- Borrowed inputs remain independent under report mutation; owned payloads move
  their eligible leaves. Existing summaries stay idempotent and literal markers
  are not treated as executable instructions.
- The public batch path moves unused diagnostics, preserves separate report/cache
  copies of referenced fields, dispatches original final-use data, and transfers
  eligible bound payload leaves into the downstream report after execution.
- Noncompleted/unknown outcomes preserve diagnostics and halt with zero successful
  steps; unknown completion classification remains the existing runtime failure.
- Public dry-run bound-payload reports match prior values and leave the source
  batch unchanged. Cache insertion previews remain mutable without changing
  complete 4096-item cached arrays or their final ownership transfer.

The existing two real SDK/CLI persistence chains now compare the first three
node/element report samples against the original complete results. Long Chinese/
Arabic research-note summaries must preserve exact byte counts and Unicode
prefixes, while each stored version retains the entire note and result.

Each result still has 513 nodes/512 elements and independently satisfies `FL/EA`
at relative `1e-12`. The fanout chain performs one calculation with seven versions;
the combined chain performs exactly two explicitly requested calculations with
three versions. No reread, report construction or binding repeats computation.
CLI JSON stdout equals the report file, stderr is empty, and owned temporary
Orchestra/SQLite, Agents and data are cleaned up. This is not a coupled study.

The bounded pointer/capacity fixtures and previous-value oracle test behavior,
not just source strings. No global allocation hooks or retained large benchmark
files are added. They do not establish deployed peak RSS or throughput.

## Verification

macOS ARM64 source regression:

- Headless SDK: 445 unit tests in 34.02 s plus 25 integrations, all passed.
- Protocol: 114 unit tests plus seven integrations, all passed.
- Complete real-service suite: 43/43 in 22.18 s, all passed.
- CLI binaries: 182 and 16 unit tests; selected integrations: surface 1,
  execution posture 16, parameter patch 2, research round 2, task completion 5,
  wait recovery 1, all passed.
- CLI/SDK/Protocol test-target Clippy passes with warnings denied; Rust formatting
  and diff whitespace checks pass.

Full selected targets have zero ignored/filtered tests. Focused runs exclude
unrelated tests. Durations describe test execution, not throughput improvement.
Backend execution is exercised, but no new full Elixir or Linux suite is claimed.

Runtime API, topology, matrix, extension standard, tensor self-test/validation,
documentation book/inventory, organization and version-line checks pass. Native
`verified` evidence is included without qualification promotion: zero structural
gaps, four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps; Daji readiness
remains blocked. Source/document limits remain 800/2000 lines with zero tracked
organization debt. All 224 exact version contracts match actual source metadata
3.5.0, independently of the base commit's 3.5.1 subject.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

Ownership transfer reduces repeated report allocations, not required copies of
shared previews, full binding values, caller-retained payloads, parser results,
result/raw mirrors or concurrent branches. Reports still contain every object
field/key and retained step. There is no aggregate report-byte/process-RSS cap,
depth limit, OOM guarantee, CPU preemption, checkpoint or durable recovery claim.
Shrinking excess capacity may allocate/copy; allocator overhead and reuse are
outside the value-level policy. Existing borrowed compaction remains necessary.

Independent clients in `sdks/`, installed GUI/Agent, authenticated/durable
deployments, cross-language parity, scientific correlation and scale qualification
remain separate. Evidence is native `verified` only, with no grade promotion.
No App rebuild/install, version bump, commit, push or remote run was requested
or performed in this turn.
