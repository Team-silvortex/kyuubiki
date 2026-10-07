# Headless Native Binding Result Lifetimes

Date: 2026-10-07. Base checkout: `0712b88d`, daji 3.5.0 source overlay.
Existing uncommitted changes and historical evidence are preserved.

## Repair

Native batches previously retained every successful executor result until batch
end, even when no downstream step referenced it. Whole-value bindings cloned
their source on every use, including the final use. Dry-run and local TaskIR
preparation also copied their complete previews into this result map.

The private `BindingResults` cache now counts references in the original payloads
after whole-batch validation. It shares the existing binding parser and collects
only whole strings in object values/arrays, not object keys, inline interpolation
or strings later returned by an executor. Actual successful top-level outputs
are moved into the cache only when later steps reference them. Unused fields,
including unused raw/result mirrors, and unreferenced terminal results are not
retained there. Outputs from one source have independent lifetimes.

Earlier uses clone an independent value; the final use removes and moves the
original value into the dependent payload. Empty per-source cache and reference
entries are released. Preview paths preserve their existing report-owned value
and copy only referenced outputs into the cache. No HTTP call, server write,
computation, result publication or automatic replay is added.

The public SDK API, result and report shapes and binding grammar are unchanged.
Actual results remain distinct from compact report previews. Missing or non-object
source outputs keep their existing failure; present optional null stays present.
Required payload checks still follow resolution. Whole-batch validation and risk
confirmation still precede dependent execution. A partial resolution failure
stops the batch; consumed cache values do not authorize retry or continuation.

No numerical algorithm or alternate runtime is introduced. Solver retains
numerical ownership; Engine/Agent retain admitted-task execution authority.

## Tests

Twelve new SDK tests cover unused-result/mirror rejection, a 1-MiB string's
final-use pointer identity, independent nonfinal array copies and original
final-use array ownership, repeated nested bindings, whitespace and leading-zero
step spellings, independent output lifetimes, optional null versus absent output,
opaque returned templates, literal fragments/object keys and preview ownership.
A 20-layer chain transfers the same 4096-value array allocation across handoffs;
only its live frontier remains in the binding cache, not all completed layers.
This is pointer/retention verification, not a timing or RSS benchmark.

Nine depth/fanout combinations (depth 2/8/20, fanout 1/2/4) compare every resolved
payload against a test-only reference of the previous whole-value resolver.
The public batch dispatcher separately checks original allocation transfer and
unchanged compacted result/payload reports. Existing confirmation, missing-output,
opaque-value and dry-run regressions remain in the full suite. The older unbound
large-payload borrowing test now plans a later reference so its source actually
remains available while the unrelated payload is borrowed.

A new real-service test owns temporary Orchestra/SQLite and two Rust Agents.
An actual saved-version axial bar computes once at 512 elements, with 513 nodes;
the tip displacement agrees with independent `FL/EA` at relative `1e-12`.
It then runs a nine-step job-read/wait/result-read/model-version chain through
the public SDK and a distinct CLI invocation. Nonadjacent source IDs, repeated
whole-result references and both preferred-job and separate-result reads are used.
Six explicitly requested new versions contain exact complete source results;
node/element arrays are not replaced by their compact run-report summaries.
CLI JSON stdout matches its report file and stderr is empty.

The source job/result remain exactly equal JSON values on HTTP readback; the single
job and total Agent execution count stay one. Only the initial model version
and six requested new versions exist. This checks result propagation into model
research metadata, not a coupled numerical feedback solve or six new calculations.
The owned services and temporary data are cleaned up after the test.

## Verification

macOS ARM64 source regression:

- Headless SDK: 406 unit tests in 33.55 s plus 25 integrations, all passed.
- Protocol: 114 unit tests plus seven integrations, all passed.
- Complete real-service suite: 42/42 in 27.43 s, all passed.
- CLI binaries: 182 and 16 unit tests; selected integrations: surface 1,
  execution posture 16, parameter patch 2, research round 2, task completion 5,
  wait recovery 1, all passed.
- CLI/SDK/Protocol test-target Clippy passes with warnings denied; Rust formatting
  and diff whitespace checks pass.

Full selected targets have zero ignored/filtered tests. Focused runs filter
unrelated tests. Durations are test execution times, not throughput measurements.
The real-service suite exercises the backend; no new full Elixir or Linux suite
is claimed for this native cache follow-up.

Runtime API, topology, matrix, extension standard, tensor self-test/validation,
documentation book/inventory, organization and version-line checks pass. The
tensor includes the scoped native `verified` evidence without promotion: zero
structural gaps, four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps;
Daji readiness remains blocked. Source/document limits remain 800/2000 lines,
with zero tracked organization debt. All 224 exact version contracts match 3.5.0.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

The plan adds one payload traversal and per-reference/output counters. Retained
storage still grows with the live dependency frontier and fanout. Overlapping
requested public fields remain separate values. Original batch payloads, reports,
dry/local previews, parsing, serialization and public `job_fetch` raw/result
normalization still allocate. This is not a process-wide byte/RSS cap, an OOM
guarantee, or complete result streaming. Transport limits remain separate.

No public diagnostic/result field is removed to reduce memory. The cache is
ephemeral, not a durable checkpoint, resume mechanism, provenance proof or
exactly-once guarantee. Recovery remains caller-authored and explicitly authorized.
Independent clients in `sdks/`, installed GUI/Agent acceptance, authenticated or
durable deployments, 1M-scale throughput/RSS and scientific qualification remain
separate. Evidence is native `verified` only; no qualification grade is promoted.
Installed local baseline remains 3.4.0. No App rebuild/install, version bump,
commit, push or remote run was requested in this turn.
