# Headless Material Plan Preflight

Date: 2026-10-07. Base `7276aa6c` has subject `daji 3.5.1`; source/package
metadata remains 3.5.0 and installed baseline remains 3.4.0. Earlier uncommitted
receipt, publication, output-path and research-plan repairs are retained. No
version bump, App install, commit or push is performed by this follow-up.

Later follow-up: [fixed input profile checks](headless-material-input-profiles-20261007.md)
now reject physical input edits that would reuse original report constants. This
report's 955-test measurements describe the earlier candidate-chain repair;
the new profile cache and expanded checks have separate evidence below that link.

## Candidate Contract Repair

The previous `validate_material_report_compatibility` checked template identity
and dielectric unit consistency, but accepted an empty heat-spreader execution
batch. The new empty-batch regression reproduced that acceptance before the fix.
All five built-in builders interpret result payloads positionally against fixed
candidate factories. Counting readbacks alone cannot distinguish a missing,
repeated, reordered or mislabeled candidate source.

The same public SDK API now validates a complete fixed-study candidate chain
without executing or mutating it. A separate private module uses the existing
candidate factories for IDs and the existing batch/binding contract for syntax,
actions, indices and risk. It does not regenerate full solver models. Every
candidate has exactly one matching solver and unique known research identity;
each declared study resolves to the selected canonical study. The report needs
exactly one ordered `result_fetch` per candidate.

A forward-only source ledger ties each readback to an earlier owned solver's
`job_id`, directly or through owned wait forwarding. That source must have a
preceding `job_wait`. Literal unrelated job IDs, repeated or swapped readbacks,
unknown candidates, contradictory aliases and missing/extra candidate solves or reads
are rejected. Canonical interleaved and grouped solve/wait/read chains pass;
this is not an exact nine-step layout requirement. Utilities may coexist with
the complete candidate chain under the ordinary batch contract.

Batch payloads retain canonical `job_id`. Executor alias-only `jobId` support
does not extend batch required keys; consistent dual aliases and whitespace
accepted by the shared whole-value binding parser are supported. The initial
alias-only acceptance fixture failed the existing required-key check, and was
corrected rather than weakening that check. Dielectric validation also rejects
empty element lists and unrepresented/nonpositive expected SI permittivity.

The native CLI already invokes this SDK policy before executor construction.
Invalid non-empty plans retain a non-retryable
`material_report_input_contract_mismatch` at `material_report_validation`, even
if diagnostic metadata contains timeout or artifact-limit wording. Empty workflow
documents still fail at earlier normalization with `headless_command_failed` at
`command_validation`; a directly constructed empty batch fails material preflight.
Requested failure run files may be published, but original workflow bytes and
prior material files are retained. No workflow service request is made for these
static failures, including a write preceding the first candidate solve.

## Regression Coverage

Thirteen new tests comprise seven SDK policy tests, two CLI/private-boundary
unit tests, three controlled CLI integration tests and one actual-service test.
Loops cover all five built-in studies and their aliases, truncations to zero,
two, three, six and eight steps, extra reads, wrong/missing solver or identity,
duplicate candidates, absent/unrelated waits, swapped/literal/repeated sources,
wait-forwarded bindings, contradictory aliases and empty dielectric elements.
SDK input snapshots remain unchanged.

The controlled HTTP spy records zero requests for invalid material plans. Each
failure has zero executed steps/jobs, a saved failure run matching stdout,
the exact expected stage, unchanged source/prior material bytes and no staging
residue. Full nine-step mock previews for all five studies still produce their
three-candidate reports with no service requests. Those previews qualify adapter
plumbing only, not simulated physical results.

A temporary Orchestra/SQLite and two actual Rust Agents receive invalid heat
candidate batches that would first create a project. Missing final readback,
repeated first-candidate readback and unknown candidate identity are rejected
before execution. Projects/jobs remain JSON-equal to their baseline, the job list
is empty, both Agents stay accepting idle and total calculation count is zero.
Requested failure run files match stdout and source/prior material files survive.
Fixture-owned processes and temporary data are cleaned after testing.

Two earlier shortened-material CLI tests now assert early preflight instead of
post-run generation failure. A private post-run unit test independently retains
the builder defense: an explicit mock report with a corrupted partial retained
result set is refused without changing its run receipt. Earlier real retained
bar-result recovery and research generation-failure tests remain in the selected
suite. Historical measurements in the artifact-generation report remain dated
to that earlier implementation, with this policy change linked explicitly.

## Verification

Final selected source regression passes 955 tests with zero failures, ignored
tests or filtered tests in the full selected runs:

- Headless SDK: 458 unit and 25 integration tests; the unit suite takes 33.99 s.
- Protocol: 114 unit and 7 integration tests.
- CLI: 182 ordinary CLI and 45 Headless unit tests plus 107 selected integration
  tests. All 48 actual Orchestra/Agent tests pass in 21.74 s, including the new
  no-effect material preflight test. All three controlled material tests pass.
- KCore: all 17 consumer tests pass, including retained research-series exchange
  validation. This does not qualify arbitrary external archives.

Clippy for CLI/SDK/Protocol tests passes with warnings denied; workspace formatting
and `git diff --check` pass. Runtime API-surface, module topology, module/function
matrix, extension-standard, coverage-tensor, book/inventory, organization and
version-line checks pass, including applicable self-tests. The matrix remains
13 modules by 11 paradigms. Tensor status retains zero structural, four maturity
and 19 evidence-grade gaps, 14 P0 gaps and blocked Daji status, without promotion.
Documentation checks cover 26 HTML files. All 224 exact source version contracts
match 3.5.0; organization has zero tracked debt under the 800-line source and
2000-line document limits. New source/test modules are also below 800 lines.

Reproduce from `workers/rust`, allowing temporary local service fixtures:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_artifact_generation --test headless_material_preflight --test headless_research_preflight --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery --test headless_output_publication --test headless_output_paths -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-kcore -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Qualification Limits

This is macOS ARM64 source execution/contract/recovery evidence at `verified`,
not numerical/scientific, producer-authenticity, installed GUI/Agent, durable,
Linux/Windows, Python/Elixir parity or scale qualification. Tensor registration
adds this scoped evidence without promoting existing qualification grades.
Solver retains numerical algorithms; Engine/Agent retain execution authority.
No protocol/schema version, worker implementation or automatic replay changes.

Declared candidate IDs and study labels are not authenticated result provenance
or proof that an edited model matches a factory's material constants or geometry.
The SI consistency check is not model fidelity. Static acceptance cannot predict
runtime result fields, finite metrics, valid physical results or service behavior.
Actual job/result admission, report completeness, research lineage and physical
validation remain required. This work is not a fresh successful real heat solve
or a scientific study qualification.

The optional built-in policy does not restrict arbitrary Headless workflows,
worker operator graphs or independently provided payloads to the public material
report builders. Retained external/literal-job results can be read separately;
callers must validate their association and ordering before rebuilding reports,
not replay solves to satisfy this static plan gate. General result provenance,
new-candidate materialization and durable multi-file recovery remain separate
work. No filesystem sandbox, aggregate RSS bound or throughput claim is added.
