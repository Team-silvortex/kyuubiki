# Headless Saved Model Reference Gates

Date: 2026-10-07. Base checkout: `0712b88d`, current daji 3.5.0 source overlay.
Prior uncommitted repairs, version alignment and historical report fingerprints
are preserved. No solver algorithm moves into Engine/Agent.

## Repairs

Saved model/version GETs previously required an object but did not establish
that its identity matched the requested reference. The direct mesh source
selection could also prefer caller inline input over a saved reference. A native
FEM fallback omitted reference context from its submitted body. This follow-up
adds a shared loader for three actions:

- `solve_from_model_version`.
- `solve_and_wait_from_model_version`.
- Reference-based `direct_mesh_solve`, including native FEM fallback.

Selected records require usable unmodified canonical IDs, matching requested
identity, usable parent project/model IDs, consistent repeated IDs and version
aliases, a usable nonblank kind and an object payload. Any supplied project/model
parent hint must agree. Reference aliases cannot conflict. A saved reference
cannot coexist with caller `input` or `model_payload`, including null fields.
The loaded version is resolved only once before fallback.

The identity/source gate emits `model_reference_invalid:` under the existing
`contract_failure` category at `validation`, without retry, bindings, successful
steps or downstream calculation submission. This is a known pre-submit failure,
not an uncertain write acknowledgement. Rejected identifier values are not
echoed. Transport/read failures and complete 4xx diagnostics retain their
existing classifications; this change does not make every GET error nonretryable.

Native FEM fallback now transmits the verified project and, when explicitly
selected, saved version context. Mutable model references retain project context
but do not fabricate a version association from `latest_version_id`. Native
version route selection still uses the saved record kind. Existing caller
study-kind/material overrides in mesh routing are not removed or validated for
physical equivalence by this identity gate.

## Tests

Five new SDK boundary tests iterate over eight action/source/route cases:
version solve/wait with native or explicit mesh fallback, and direct mesh with
model/version sources with and without explicit endpoints. They cover absent,
non-object, missing, wrong, null/numeric/boolean, blank/whitespace, control,
traversal, encoded-slash and Unicode identities; parent hints and repeated
identities; kind/payload shape; mixed input sources; and valid body context.
Invalid cases run a batch with an independent later project write, which is
never called. Mixed sources and invalid primary aliases stop before network I/O.
Healthy fixtures are corrected to include real parent IDs rather than weakening
the loader. Explicit mesh behavior is fixture evidence only.

Two new actual service tests own temporary SQLite/Orchestra and two Rust Agents.
The bounded test-only relay forwards a selected GET, receives its actual reply,
removes exactly one nested identity and emits complete HTTP 200 with correct
byte length. This is an injected test fault, not a normal backend fault observed
in production. Every original/mutated pair is compared exactly; direct reads
prove the underlying record remains intact.

Corrupted version solve, version solve/wait, direct version and direct model
reads stop. An additional CLI experiment repeats only the corrupted read and
checks nonzero exit, non-retryable JSON stderr and stdout/report parity. The
five read attempts create no job, later project, model/version or Agent
calculation. Test-captured original identities are independent oracles only.

The normal test creates a project/model at 1000 N and a version at 2000 N, then
runs two separate chains without explicit mesh endpoints:

```text
direct_mesh_solve(saved model or version) -> job_wait -> result_fetch
```

Both chains complete using the actual Orchestra native FEM route. Jobs retain
the verified project; only the explicit version reference retains a version ID.
The mutable model load is read independently rather than assumed after version
creation. Displacements match `FL/EA` for their selected loads within relative
`1e-12`. Two distinct jobs produce exactly two Agent calculations. This small
analytical example is not general material or solver qualification.

## Verification

Final macOS ARM64 source checks:

- Headless SDK: 357 unit tests in 32.92 seconds plus 25 integrations, all passed.
- Protocol: 114 unit tests in 0.45 seconds plus seven integrations, all passed.
- Complete actual live suite: 35/35 in 30.99 seconds.
- CLI binaries: 182 and 16 unit tests. CLI surface, execution posture, parameter
  patch, research round, task completion and wait recovery: 1, 16, 2, 2, 5 and 1
  integration tests respectively, all passed.
- CLI/SDK/Protocol test-target Clippy passes with warnings denied.

Full targets have zero ignored or filtered tests. Focused development runs
filter unrelated tests. An initial positive batch fixture used executor aliases
where the registered batch contract requires canonical keys; the fixture was
corrected without changing the preflight contract. Durations are not benchmarks.

Remote Linux verification of this follow-up is not yet established. The
temporary test directory was created, but source transfer was blocked by the
security reviewer pending explicit authorization; no source or private config
was transferred by that rejected command. Earlier Linux reports are retained
as historical evidence, not relabeled as verification of these changes.
The newly created empty remote test directory was removed with absence verified.
No remote container, installed service or deployed data was changed in this run.

Runtime API, topology, module matrix, extension standard, tensor self-test and
validation, documentation book/inventory, organization and version-line checks
pass. The matrix retains 13 modules and 11 paradigms with zero structural gaps;
the tensor retains four maturity, 19 evidence-grade and 14 P0 gaps, with Daji
readiness blocked. Source/document limits remain 800/2000 with zero tracked
debt. The version audit checks 224 exact contracts with zero mismatches.
Formatting and whitespace checks pass. Local live suites remove owned scratch
resources on exit; no installed service restart or persistent data migration
was performed.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

This is selected identity/source validation, not full JSON Schema, exact
request-bound payload identity, immutable revision validation, authorization,
durability, atomic batch, exactly-once or automatic recovery. Kind/material
overrides and generic mesh deployments remain separate qualification work.
Empty object payloads reach existing physical validation; they are not certified
numerical models. Preflight canonical required keys remain unchanged.

The tensor claim is native source `verified`, not installed APP, Windows,
Python/Elixir parity, scientific, scale or long-run qualification. No version
bump, commit, push, installed service restart or APP rebuild is performed.
