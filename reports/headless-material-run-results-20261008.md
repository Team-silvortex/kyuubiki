# Headless Material Run Results

October 8, 2026. macOS ARM64 source overlay on commit `fe0f025d`
(`daji 3.5.2`); source/package metadata remains 3.5.0 and the last locally
installed baseline remains 3.4.0. No version bump, App packaging, remote test,
public release or qualification promotion is part of this follow-up.

Fixed material reports now require internally consistent retained execution
records and owned candidate solve/wait/read chains. SDK and standalone native
report generation share this policy. Raw result arrays remain caller-owned;
explicit mock runs remain previews, not scientific evidence.

## Reproduced Defect

The former extractor excluded only failed/blocked result-fetch steps. It could
take successful-looking result objects from an overall failed run, accept unknown
or dry-run step states, and fall back to a whole preview when the explicit result
was missing. Fixed builders then associated results with candidates by position
without checking the retained candidate/job chain.

The new `failed_or_unfinished_runs_cannot_publish_partial_material_results` test
first failed against the original code: an overall failed nine-step run with
three retained results was accepted. This was a synthetic receipt mutation,
not an observed deployed server fault.

## Contract Repair

`material_run_results.rs` provides one borrowed retained-run view for typed SDK
reports and JSON inputs. It checks the current schema, a named `execute:` mode,
overall `ok` status, valid issue-free validation, no confirmation/failure stop,
matching executed-step count, ordered one-based indices and only `executed`
steps. A failed report retains its failed-step diagnostic in the rejection.
Unknown, cancelled, blocked or unfinished records cannot supply partial results.

Every fetch must declare a resolved requested job with noncontradictory aliases,
carry the same returned job identity and retain an explicit object result.
The existing native result-envelope validator checks optional job/status fields.
Missing/null/array results and compacted root summaries are not object results.
Non-mock runs reject explicit preview markers or simulated-result receipts;
the known `execute:mock` mode remains explicitly usable for preview fixtures.

Fixed report construction additionally requires all three unique candidates,
their registered solver action and matching study metadata, unique submitted
job identities, preceding completed owned waits and canonical readback order.
Explicit root `result.research` candidate/study claims cannot contradict the
owned candidate. The existing factory candidate catalogs are reused rather than
introducing a second candidate registry. Interleaved/grouped schedules, study
aliases and custom workflow IDs remain supported.

`build_material_report_from_input(study, payload, optimization)` is the shared
JSON/raw-input dispatcher used by the standalone native material report command.
Both it and `build_material_report_from_run` enforce the fixed-study chain. The
lower-level extraction helpers check general run/result consistency without
assigning a study. Objects declaring a run schema or `steps` cannot bypass run
validation by adding a fallback `results` array. Ordinary raw arrays and explicit
`results`/`result_payloads` objects retain the existing caller-owned path and
optimization profiles.

Validation borrows payload/result trees and retains only step references and
small identity maps. It clones admitted result objects for the existing owned
report API, not the whole run. It issues no service requests, creates no files
and cannot replay computation. General solve execution, Solver numerical
ownership and Engine/Agent execution authority are unchanged.

## Regression Scope

Nine SDK tests cover overall/step-state rejection, contradictory headers/counts,
job aliases and returned identities, explicit object-result requirements, all
five fixed-study ownership chains, grouped schedules and aliases, raw inputs,
optimization preservation, no caller mutation and contradictory returned
research metadata. Synthetic receipts are clearly fixture execution, not a
service/physics acceptance claim.

The controlled CLI tests reject nine mutated records per study (45 cases),
leave source bytes and prior material reports unchanged, emit no successful
stdout and create no extra files. A separate wrong-study test fails at an
unowned candidate read. Its initial assertion expected a later completeness
diagnostic; the assertion was corrected to the actual earlier rejection, not
by relaxing production policy. The old positive standalone test now uses an
explicit mock SDK receipt rather than incomplete JSON labeled as service.

The actual-service test starts temporary Orchestra/SQLite and two Rust Agents,
executes the five built-in studies, and builds three-candidate reports from
each real service run. The standalone CLI reconstructs each report from saved
JSON identically to the SDK. Five mutations per retained run (25 cases) are
rejected: overall failure, wrong result job, unknown source candidate, unfinished
wait and contradictory returned candidate. Every rejection preserves the saved
source and existing material output. Project/job list snapshots and combined
Agent calculation counters remain unchanged throughout regeneration/rejection.
Fixture state and its two overwriteable artifact files are removed on completion.

These are small factory models and receipt-recovery checks, not independent
five-domain numerical certification, throughput benchmarks or large-mesh proof.

## Shared Test Request Reader

The expanded CLI regression exposed another inherited-nonblocking health fixture:
`alias_created_during_service_execution_preserves_completed_stdout_and_source`
failed at its first request read with `WouldBlock`, then its completion channel
reported a secondary send error. The prior artifact fixture's repaired reader is
now shared in `tests/support/health_request.rs`. Both fixtures switch accepted
sockets to blocking mode and wait for complete headers under one two-second
deadline and a 4 KiB cap. The existing delayed three-fragment/three-request test
exercises that common reader. Production transport behavior is unchanged.

## Verification

Final selected full-suite source regression passes 994 tests, with zero failures,
ignored or filtered tests in those selected runs:

- Headless SDK: 478 unit tests in 34.08 s and 25 integration tests.
- Protocol: 114 unit tests and seven integration tests.
- CLI: 182 ordinary and 45 Headless unit tests, plus 126 selected integration
  tests. All 50 actual-service tests pass in 29.56 s. The selected integration
  count includes the 12 standalone material-report cases and both new retained
  result tests, plus output paths/publication and research recovery regressions.
- KCore: all 17 consumer tests pass, without new archive qualification.

Repeated focused runs and earlier failed diagnostic runs are not added to this
independently summed full-suite total. CLI/SDK/Protocol test Clippy passes with
warnings denied. The new source modules remain below 800 lines and the SDK guide
is 1979 lines, within the 2000-line document limit.

Workspace formatting and `git diff --check` pass. Runtime API-surface (four
families), module topology, the 13-module/11-paradigm matrix, extension standard,
coverage tensor, book/inventory, organization and version-line checks pass with
their applicable self-tests. The book check covers 26 HTML files and all 224
exact version contracts match source/package 3.5.0. Organization retains zero
tracked debt. Tensor gaps remain zero structural, four maturity and 19
evidence-grade gaps, including 14 P0 gaps; Daji status stays blocked. The added
evidence remains scoped verified source consistency/recovery, not a grade
promotion.

The active native Headless workflow qualification also passes both suites with
all required anchors present, including the new unfinished-run and fixed-chain
tests. Its existing largest-test-binary-per-suite counting rule reports 494
tests (478 SDK unit tests and 16 CLI boundaries); the SDK and CLI suites take
64.271 s and 8.744 s respectively. This separate repeated execution is not added
to the independently summed 994 selected tests. Its overwriteable report stays
at `tmp/headless-workflow-qualification-report.json`, not in versioned evidence.

Reproduce the active gate from the repository root with local service fixture
permissions:

```text
CARGO_NET_OFFLINE=true make qualify-headless-workflow
```

## Qualification Limits

This is source-level macOS execution/contract/recovery evidence. Receipt labels,
job IDs and declared candidate metadata are not authenticated provenance. The
retained record cannot independently establish the original physical inputs,
producer identity, immutable server state, scientific accuracy or completeness
against a separately authenticated original batch. Use fixed-input preflight
before solving and independent numerical/lineage checks after solving.

Literal/raw result builders retain caller responsibility for candidate identity,
physical applicability and result quality. Root returned identity checks do not
inspect arbitrary nested subcomponent metadata. Explicit mock reports are
previews. Installed GUI/runtime, Linux/Windows, Python/Elixir parity, durable
recovery, power-loss atomicity and scale remain separate qualification. Historical
evidence is not relabeled and tensor grades are not promoted by this repair.
