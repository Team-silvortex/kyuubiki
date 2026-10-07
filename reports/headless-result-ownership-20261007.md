# Headless Native Result Envelope Ownership

Date: 2026-10-07. Base checkout: `0712b88d`, daji 3.5.0 source overlay.
Existing uncommitted changes and historical evidence are preserved.

## Repair

Native normalization used `json!` with already-owned `serde_json::Value` data.
The locked serde_json macro passes expressions through `to_value(&expression)`,
which reconstructs their values rather than transferring ownership. Job-state
and submission normalization also explicitly cloned projected fields before
that reconstruction. Large original envelopes and their temporary copies could
coexist unnecessarily while preparing an otherwise unchanged public result.

Job observation/submission normalization now directly assembles a JSON object,
moving the original envelope into `raw`. Each projected public field is cloned
once where independent public values require it. The `job_fetch` `raw`, `job`
and `result` diagnostics are retained unchanged; this does not make its full
result/raw mirrors share storage or remove their necessary final duplication.

Preferred result reading validates completion/context/object-result first,
then moves the original result and job into the same existing public output.
Separate result reading also moves the result after its existing identity and
association gates. Combined saved-version solve/wait moves its owned `solve`,
`wait` and `result` envelopes into the unchanged nested output. Small public
metadata aliases still have independent values and existing optional defaults.

No result/report schema, binding contract, receipt/risk/completion gate, retry
rule, request route or server action changes. Query success is still distinct
from completed-result admission. Unknown writes remain unknown without replay.
No numerical implementation is added; Solver retains numerical ownership and
Engine/Agent retain admitted-task execution authority.

## Tests

Eleven new SDK tests verify:

- Original job observation/submission raw arrays and 1-MiB strings retain their
  allocation pointers, while public job/result mirrors remain independent.
- Mutation of an exposed result or job does not affect `raw`, and raw mutation
  does not affect the exposed result. Template-looking strings stay literal.
- Observations match the previous shape across all eight public states and
  96 optional-progress/result combinations, including absent/null results and
  diagnostic arrays.
- Non-job fallback retains its existing value/ownership behavior; submission
  terminal markers and contradictory/missing receipt rejection remain unchanged.
- Preferred reads move the original job diagnostics and complete result; separate
  reads move complete results and retain their previous fallback/null shapes.
- Combined solve/wait/result assembly moves original arrays and strings from all
  three envelopes, matching the former nested output and optional metadata defaults.

These use allocation-pointer checks and test-only previous-shape references,
not source-text assertions alone. Payload sizes are bounded and data remains in
memory; no retained large benchmark files or global allocation hooks are added.
The 1-MiB string/4096-value fixtures are ownership regressions, not an RSS benchmark.
Existing identity, association, response-budget, unknown-write, incomplete-result,
confirmation and failure-isolation tests remain in the full suite.

The existing real result-fanout regression now also checks original combined
result equality and independently mutable full `job_fetch` mirrors. Its source
513-node/512-element calculation still happens once, despite SDK/CLI read chains.

One new real-service test owns temporary Orchestra/SQLite and two Rust Agents.
It executes a four-step combined saved-version solve, full-result model-version
write, separate result read and job observation through SDK, then a distinct
explicit CLI experiment. Each experiment creates exactly one job/calculation.
Both complete 513-node/512-element results are equal, independently satisfy
`FL/EA` at relative `1e-12`, and are saved exactly in research metadata rather
than as their compact report summaries. Job IDs and nested solve/wait/result
identities match; full original job/result readback agrees with the stored receipt.
CLI JSON stdout equals its report file and stderr is empty. Exactly two jobs,
two Agent calculations and three versions (initial plus two requested) exist.
The second solve is caller-requested, not a replay or recovery attempt. Reading
and normalization add no calculation. This is not a coupled feedback study.
Owned test services and temporary files are cleaned up.

## Verification

macOS ARM64 source regression:

- Headless SDK: 417 unit tests in 33.49 s plus 25 integrations, all passed.
- Protocol: 114 unit tests plus seven integrations, all passed.
- Complete real-service suite: 43/43 in 25.36 s, all passed.
- CLI binaries: 182 and 16 unit tests; selected integrations: surface 1,
  execution posture 16, parameter patch 2, research round 2, task completion 5,
  wait recovery 1, all passed.
- CLI/SDK/Protocol test-target Clippy passes with warnings denied; Rust formatting
  and diff whitespace checks pass.

Full selected targets have zero ignored/filtered tests; focused runs filter
unrelated tests. Durations describe test execution, not throughput improvements.
The live tests exercise the backend; no new full Elixir or Linux suite is claimed.

Runtime API, topology, matrix, extension standard, tensor self-test/validation,
documentation book/inventory, organization and version-line checks pass. The
tensor includes scoped native `verified` evidence without qualification promotion:
zero structural gaps, four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps;
Daji readiness remains blocked. Source/document limits remain 800/2000 lines,
with zero tracked organization debt. All 224 exact version contracts match 3.5.0.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

This removes redundant native normalization copies, not all memory overhead.
Independent public raw/result/job mirrors remain, including overlapping fields.
Wire buffers, JSON parsing and request serialization, original payloads, reports,
branch fanout, concurrency and caller-retained results still allocate. There is
no total byte/RSS cap, OOM guarantee, streaming result reader or disk spooling.
The prior per-response transport limits and binding-cache lifetimes remain separate.

No quantitative peak-RSS/throughput or 1M-scale result is claimed. Independent
clients in `sdks/`, installed GUI/Agent, authenticated/durable deployments,
cross-language parity, scientific provenance and general qualification remain
separate. Evidence is native `verified` only, without promotion. Installed local
baseline remains 3.4.0. No App rebuild/install, version bump, commit, push or
remote run was requested or performed in this turn.
