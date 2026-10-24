# Harmonic Complex RHS Range Recovery

Date: 2026-10-24

Source: daji 3.7.0 after `60115afa`, with this uncommitted numerical overlay.
This extends the [complex arithmetic range repair](harmonic-complex-arithmetic-range-20261023.md).
Solver owns the change. Engine, Agent, TaskIR, payloads and result formats are
unchanged; no desktop rebuild or installation is included.

## Reproduced Failures

Two solver unit controls and one public solve failed before implementation:

- Both complex factorization paths divided RHS `2` by row scale `1e-308`,
  materializing infinity before division by the normalized complex pivot.
  The original one-row equation `(1e-308 + i 1e-308) x = 2` has finite
  components approximately `1e308 - i 1e308` and finite amplitude.
- A triangular two-row control produced temporary RHS `1.95e308` after
  elimination. Its pivot `1 + i` gives finite solution components
  `9.75e307 - i 9.75e307`; restoring the RHS first incorrectly rejected it.
- A public grounded damped spring with tiny positive mass, load `2`,
  stiffness/damping `1e-308` and angular frequency one failed before original
  equation admission. Its displacement amplitude is approximately `sqrt(2)*1e308`
  and force amplitude approximately `2`, both representable.

The matrix controls are generic complex arithmetic tests, not independently
assembled physical spring networks. Public path and weakly linked cyclic
fixtures separately exercise original-equation admission and force publication.

## Representation And Ownership

The existing RHS vector remains ordinary complex binary64 storage until a value
cannot restore reversibly. Only then does the private adapter promote that vector
to separate real/imaginary mantissa-exponent records. Row normalization retains
nonmaterializable quotients; forward updates accumulate complete scaled products.
Pivot swaps move the whole RHS identity, and back substitution divides before
restoring the final unknown. A temporary quarter-quantum RHS can therefore become
one representable quantum after division by one quarter.

Matrix row scaling, coefficient storage, pivot selection/floors and original
physical-equation admission are unchanged. Final unknown components must be finite
and nonzero when required; rounded subnormal unknowns remain subject to the same
original-equation gate. Final displacement/extension amplitudes and force products
still have their independent publication checks. An overflowing displacement norm
cannot be excused by finite components or finite force.

Normal-path normalization, subtraction and division keep the previous arithmetic
order. A pointer/bit control checks that the ordinary adapter neither promotes
nor copies its original RHS allocation. Exceptional conversion uses one sized
O(n) allocation, with bounded cancellation while converting and collecting the
final solution. Old/new allocations coexist transiently during conversion; this
is not a process-wide RSS cap or zero-allocation claim for exceptional systems.
No persistent cache, checkpoint format, new execution service or doubled-real
matrix is added. Runtime predicates add work; no throughput improvement is claimed.

## Regression And Recovery Scope

Thirteen new tests comprise eight Solver units, two public Solver integrations
and three Engine integrations:

- One-row normalization, overflowing elimination RHS, adjacent and dense pivot
  swaps, and complete-row back substitution retain finite solutions.
- Independent quantum-sized rows survive promotion and swaps unchanged.
  Quarter-quantum normalized/update temporaries restore after final division.
- Invalid scales/nonfinite RHS values and genuinely overflowing final unknowns
  still fail. Ordinary allocation and arithmetic bits are checked separately.
- Signed loads, reversed elements and borrowed/owned public calls preserve
  analytic displacement components and grounded force amplitude on path and
  cyclic topology. The cyclic links are deliberately weak; this is not a
  conditioning certificate for strongly coupled near-limit systems.
- The public output gate rejects finite components with an overflowing norm,
  including a later frequency after an earlier one completed privately. The
  complete sweep returns an error, not the earlier successful frequency.
- Serialized direct dispatch and workflow execution agree on finite results.
  Late output failure prevents successful publication; explicit corrected calls
  recover without reused mutable solver state.
- A 2,050-row adapter test cancels conversion/collection at interior and final
  checkpoints. Engine tests cancel during a later frequency's preparation,
  factorization, substitution and element-result collection, followed by clean
  replay. Existing SolverStage identities are reused rather than renumbered.

Cancellation/replay is in-process source evidence, not process-kill, durable
continuation, automatic retry or host-power-loss qualification.

## Validation

Pinned Rust 1.88.0, offline locked dependencies and existing build directories.
The remote Linux x86_64 workspace is a reused source overlay, not a clean Git
checkout. Its five preexisting changed source files matched the committed base
before transfer; all seven updated/new source files and two Cargo metadata files
matched local SHA-256 digests after transfer. No deployment configuration or
runtime installation was changed.

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions |
| --- | ---: | ---: | --- |
| Solver library | 794 | 794 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded per platform |
| Engine library | 711 | 711 | 1 ignored per platform |
| Engine harmonic integration target | 13 | 13 | No installed/Agent transport qualification |
| Public complex-force integration target | 5 | 5 | Includes both new public tests |
| Existing harmonic frequency range, balance, reliability and dynamic closed forms | 30 | 30 | Selected targets only |
| Total | 1,553 | 1,553 | Not full workspace or all-physics coverage |

macOS Solver/Engine all-target Clippy with warnings denied also passes. Native
validation checks pass for 89 profiles without execution; the new profile's
three executed commands pass all 13 new tests. That execution is a subset of
this table, not additional tests to sum again. Rust formatting, diff whitespace,
26-file HTML book, document inventory and source/document size checks pass.
The tensor remains at zero structural, two maturity, 23 evidence-grade and 18
P0 gaps with release qualification blocked; adding this bounded claim does not
promote platform, physical or operational grades.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_complex_rhs_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test harmonic_complex_force_range
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-engine --test harmonic_constraint_admission
```

The validation profile registers these boundaries under
`harmonic-complex-rhs-range-local-reliability`. The tensor binds scoped verified
numerical/recovery evidence, not an operational or physical grade promotion.

## Limits

This is bounded harmonic complex RHS range recovery, not general complex-solver qualification, arbitrary precision, independent physical assembly or durable restart.

Exceptional coefficient-matrix normalization, unresolved large-exponent alignment,
conditioning, physical reference correlation, modal families, other complex
operators, sustained-scale performance, installed acceptance and cross-language
SDK studies remain separate work. Existing reports and release snapshots retain
their original versions and test scopes.
