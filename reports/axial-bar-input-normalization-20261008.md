# Axial Bar Input Normalization Verification

## Defect And Repair

A GPa-only bar model passed small inline HTTP submission but failed when the
native SDK selected file transport: the Agent decoded native SI fields without
the HTTP unit conversion. The new shared accepted-input regression reproduced
the missing `youngs_modulus` failure before the adapter was added.

Rust protocol decoding and Elixir normalization now implement the same bounded
scalar contract. Both accept Pa-only, GPa-only and consistent dual-unit forms,
canonicalize to SI, and reject contradictory or invalid provided units. Both
reject fractional element counts instead of silently rounding. Native request
serialization and the physical solver stay unchanged. Agent reader decoding
skips unknown metadata instead of requiring full-model control-plane buffering.

## Actual Calculation Chain

A source-built macOS test owns a temporary Orchestra, isolated SQLite/artifact
storage and two Rust Agents configured for that control plane's download address.
Four input variants each run once inline and once through approximately 8 MB
file transport: GPa-only, Pa-only, dual units differing only by roundoff, and
strict numeric strings with negative loading. Full normalized solver results
match, not merely their status or selected metric. The uploaded descriptor binds
the original padded source bytes, not the normalized SI representation.

Actual file results are downloaded with a 1 MiB test-side cap and verified against
raw SHA-256 and declared length. Displacement `FL/(EA)`, stress `abs(F/A)`,
reaction `-F`, axial force `F` and energy `F*u/2` are checked independently.
Batch/report JSON survives serialization and reload; reading retained results
does not submit another calculation.

For each of conflicting Pa/GPa values and fractional counts, SDK and actual CLI
inline requests fail before job creation. The corresponding artifact requests
are accepted jobs that terminate on Agent decoding, have no result, and fail the
completion gate before a guarded project write. Attempted artifact execution is
counted by the watchdog, not falsely reported as zero admission. A separately
authorized healthy CLI upload then succeeds. The experiment observes exactly
13 Agent admissions/jobs: eight healthy comparison runs, four failed artifact
decodes and one healthy recovery. Project state remains unchanged.

The existing upload-acknowledgement substitution experiment also removes its
extra SI workaround; its healthy uploads are now GPa-only public models.

Stronger live assertions also exposed a diagnostic defect: the terminal-job
marker hid an explicit `invalid_params` Agent cause behind generic
`runtime_failure`. SDK failure classification now preserves `agent_decode` and
`invalid_solver_params` while remaining non-retryable. Unit checks include
misleading timeout/queue/connection hints and retain unknown-outcome/cancellation
precedence. Existing zero-area TaskIR tests now expect `decode_solver_input` and
`fix_solver_input_artifact`; actual modal engine failure/recovery coverage stays
at `dispatch_engine_solver`, not weakened to an input-only test.

## Verification

The selected regression suites pass **1600 tests**, with one existing Engine
test ignored. Counts below do not add focused reruns, zero-test binaries or
repeated qualification executions. This is suite breadth, not whole-repository
code coverage or scientific qualification of every solver.

| Suite | Passed |
| --- | ---: |
| Native Headless SDK | 514 unit and 25 integration |
| CLI | 16 unit and 95 selected integration, including 54 live/harness tests |
| Engine | 637 unit; one existing ignored |
| Protocol | 120 unit and seven integration |
| KCore | Six unit and 12 integration |
| Standalone Rust SDK | 106 integration |
| Solver accuracy/bar reliability | 32 integration |
| Elixir normalizer and HTTP/model/artifact APIs | 30 |

All-target Clippy for Protocol, native SDK, CLI and KCore passes with warnings
denied. Elixir compilation and Rust/Elixir format checks pass. The Protocol gate
requires and verifies all 120 unit tests, including six new normalization
anchors; it also checks 61 advertised RPC methods and five TaskIR examples. The
Headless gate requires 530 tests across its 514-unit SDK and 16-test CLI suites,
including the terminal decode classification anchor. Both local ignored reports
pass report verification.

Topology, module/function matrix, extension standard, runtime API surface,
documentation inventory/book and version checks pass. Source/document organization
passes the 800/2000-line limits with zero tracked debt. The coverage tensor
retains zero structural gaps, four maturity gaps, 19 evidence-grade gaps and
14 P0 gaps; Daji readiness remains blocked. New evidence is axial-bar validation
contract scope only, not operational or general numerical promotion.

## Scope And Remaining Work

Shared conformance has ten accepted and 26 rejected parsed-input cases. Extra
checks cover missing fields, atom-key internal HTTP input, enormous internal
integers, native SI roundtrips, raw Rust duplicate fields, streaming metadata and
early nested-scalar rejection. Duplicate-key Jason parity, every floating-point
decimal edge and unrelated solver normalization are not qualified here.

The padded four-element models test transport, not large-mesh performance.
Physical file readback is an explicit test helper, not an automatic SDK feature
or a new SDK research-metric dereference capability. Result ownership, byte
budgets, authenticated provenance, installed Apps and remote qualification remain
separate. No version bump, App rebuild, commit or push belongs to this round.
