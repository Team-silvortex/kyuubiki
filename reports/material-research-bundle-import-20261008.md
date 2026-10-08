# Material Research Bundle Import

October 8, 2026. macOS ARM64 source overlay on commit `fe0f025d`
(`daji 3.5.2`); source/package metadata remains 3.5.0. The last installed
App/runtime baseline remains 3.4.0. This follow-up adds strict retained JSON
import to both Rust Headless SDKs, without changing numerical solvers, Engine
execution, schema versions, package versions or deployment policy.

## Reproduced Gap

`validate_material_research_bundle` checked that all four declared checksums
were 64-character hexadecimal values, but did not compare them with embedded
artifact content. A structurally valid artifact edit could therefore pass the
SDK path while the retained-file checker rejected it.

The initial `verified_import_rejects_each_stale_artifact_digest` regression
failed against a parse-and-structure-only importer: a modified initial
exploration with its original well-shaped digest was admitted. The later
expanded test changes one original raw artifact at a time, so each rejection
must identify that artifact's own checksum rather than a re-encoding change
in an unrelated artifact.

## Strict Import Contract

Both Rust SDKs now expose:

```rust
let bundle = MaterialResearchBundle::from_json_verified(&original_json)?;
```

This checks the existing structure/metadata policy, then verifies the four
embedded original JSON artifacts: `initial_exploration`,
`next_round_execution_plan`, `next_exploration` and `chain`. Failure returns no
bundle. The standalone Rust validation example and retained-bundle model
validation test use this entry point instead of plain deserialization.

v1 hashes the artifact's original UTF-8 JSON bytes with JSON whitespace outside
strings removed, followed by one LF byte. It is not sorted-key canonical JSON
and not a hash of a parsed/re-encoded value. The importer borrows each raw
artifact and streams non-whitespace slices into SHA-256 without constructing a
second compact artifact buffer. String bytes, quoted whitespace, escape
spelling, numeric tokens and object/array order remain unchanged.

Indentation, outer JSON whitespace, LF/CRLF and formatting tabs may change.
Changing `1.0` to `1`, `-0.0` to `0`, an exponent's spelling, object order or a
Unicode escape to a literal character changes the v1 artifact digest even if
a consumer considers the decoded value equivalent. Read the original document
before interpreting its retained checksums. A new producer encoding may be
considered separately; this change does not silently relabel the v1 contract.

The small digest implementation is mirrored in the independently distributable
Rust SDK rather than adding a dependency on the runtime workspace. A native
source regression requires both implementations to remain byte-for-byte equal;
both SDKs also run the same boundary cases against their own packaged fixture.

## Validation Boundaries

The existing in-memory `validate_material_research_bundle` function and Rust
`validate()` method remain structure/declared-metadata checks. Their API docs
now say this explicitly. A parsed object has already lost some lexical source
information, so that API cannot prove the original v1 file digest. Python and
Elixir currently retain their structure-only helpers; this follow-up does not
claim their file-integrity parity.

The four digests do not cover every top-level metadata field and are not a
signature or authentication mechanism. A caller can edit an artifact and
recompute its hash; the import correctly accepts matching content if the
structure policy still holds. Likewise, an unprotected timestamp edit can
remain admissible. Both boundaries are explicit regression cases, not a
producer-authenticity claim. Editing the returned mutable bundle does not
retain a verification guarantee or create a persistent verified badge.

Digest agreement does not establish original physical inputs, immutable
execution history, parameter-to-result binding, external calibration or
scientific correctness. Retained labels such as `real_solver` remain declared
metadata rather than independently authenticated producer identity. Existing
screening and quality-gate limitations stay in effect.

## Regression Scope

The native SDK adds nine tests: eight import behaviors plus the implementation
mirror check. The standalone SDK adds the eight import behaviors. They cover
all four single-artifact mutations and all four digest-shaped wrong hashes,
original shared fixtures, harmless formatting changes, literal Unicode,
quoted whitespace, escaped quotes/backslash parity, numeric tokens above the
JavaScript exact-integer range, key ordering, malformed JSON, duplicate known
top-level fields, matching hashes with invalid structures, explicit
non-authenticity limits and immutable caller text.

The duplicate-key test is specifically for known top-level artifact/checksum
fields. It is not a claim that every nested or unmodeled key receives duplicate
rejection. Digest construction adds no filesystem writes, service calls or
automatic calculation replay. The owned parsed bundle and caller's original
text still consume memory; streaming the hash is not a total RSS bound.

## Fresh Export Round Trips

The current native research-bundle builder explicitly ran the initial
exploration, next-round planning, next run and chained research for both
supported retained profiles. The generated original documents were accepted by
the updated standalone Rust example:

- Heat spreader: winner `pyrolytic_graphite_in_plane`.
- Composite thermo-electric panel: winner `copper_ptfe_glass_epoxy`.

Both retain `screening_research_bundle`, reliability
`blocked_by_quality_gates`, next-round decision `mitigate_design_risk`,
next iteration 2, two runnable next steps and chain stop
`risk_mitigation_required`. These are small local linked-Rust-solver research
examples, not distributed Orchestra/Agent, remote scale or numerical
qualification tests. Importing the already generated files performs no solve.
The existing native retained-file checker also accepts both generated
documents. The two disposable output files and four intermediate exploration
files were removed after verification, together with their empty work
directories; existing artifacts were not cleared.

## Verification

The final selected full-suite regression passes 739 tests:

- Native Headless SDK: 487 unit tests in 33.38 s and 25 integration tests.
- Protocol: 114 unit tests and seven integration tests.
- Standalone Rust SDK: all 106 integration tests, including the eight new
  integrity tests and the strict-import model validation fixture.

There are no failures, ignored or filtered tests in those selected full runs.
Earlier focused and failed diagnostic runs are not added to this total.
Native SDK/Protocol and standalone SDK all-target Clippy pass with warnings
denied. No remote, GUI, installed-App or benchmark run is claimed here.

The digest contract is documented in the schema, SDK guides, research example,
HTML tutorial and current-line entry points. The SDK guide is 1991 lines,
within the 2000-line limit; each new source file remains below 800 lines.
The active Headless workflow qualification requires the stale-digest,
numeric-token and duplicate-field regression anchors. Tensor evidence is
scoped to verified source contract coverage, without qualification promotion.

The active workflow gate passes both suites and all required anchors. Its
existing largest-test-binary-per-suite rule reports 503 tests (487 SDK unit
tests and 16 CLI boundaries), with suite durations 55.273 s and 25.454 s.
This repeated execution is not added to the separately summed 739 full-suite
tests. Its report overwrites the existing development temporary path, rather
than adding versioned artifacts or backup copies.

Formatting and `git diff --check` pass, including targeted formatting of the
modified standalone Rust files without reformatting unrelated baseline tests.
Research-bundle schema/example, runtime API surface, topology, matrix,
extension standard, tensor, book/inventory, organization and version-line
checks pass with their applicable self-tests. The API surface covers four
families; the matrix covers 13 modules and 11 paradigms. The book check covers
26 HTML files, organization retains zero tracked debt, and all 224 exact
version contracts match 3.5.0. Tensor gaps remain zero structural, four
maturity and 19 evidence-grade gaps, with 14 P0 gaps and Daji status blocked.

## Reproduce

```text
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk -p kyuubiki-protocol
cargo test --offline --manifest-path sdks/rust/Cargo.toml
cargo run --offline --manifest-path sdks/rust/Cargo.toml --example validate_material_research_bundle
CARGO_NET_OFFLINE=true make check-material-research-bundle-contract
CARGO_NET_OFFLINE=true make qualify-headless-workflow
```

Full suites use temporary local fixture ports and need the local-service test
permission. Generated validation data is disposable, not retained release
evidence. Source-to-execution binding, authenticated provenance and scientific
validation remain distinct follow-up work.
