# Harmonic Coefficient Assembly

Date: 2026-10-24

Source base: `70340d07`, plus this working-tree follow-up. Package metadata and
the installed runtime remain 3.7.1. Production changes are private Solver
arithmetic; Engine, Agent, TaskIR and operator protocols are unchanged. No
version bump, commit, deployment or desktop rebuild is included.

## Reproduced Failure

A two-node oscillator has unit angular frequency, free-node mass `2^55`, a
grounded spring of stiffness one and damping two, and a second grounded spring
of stiffness `2^55`. Its represented dynamic diagonal is exactly `1 + 2i`.
For a unit real load, the separately rounded reference components are `0.2`
and `-0.4`; their norm is approximately 0.447213595499958. Negative loads reverse
the phase without changing the amplitude.

The old assembly starts at `-2^55`, adds one, then adds `2^55`. The first addition
loses one, producing a zero real diagonal. Public solving succeeds with amplitude
0.5, about 11.8 percent too large, and a phase error of about 26.565 degrees.
Removing damping makes the same path falsely singular. Element order can change
the outcome. One direct coefficient test and two public tests fail before repair.
An initial malformed test fixture was corrected before capturing these numerical
failures; input-decoding errors are not counted as the solver defect.

The existing original-system backward-error check does not catch this example:
its scale includes the separately large spring and inertial terms. This result
does not establish a forward-error certificate for ill-conditioned physics.
The repair changes assembly rather than weakening that check or the pivot floor.

## Retain And Round Addition Tails

Dense and path assembly use a shared private correction collector. Each finite
scalar addition retains its exact TwoSum tail when that tail is nonzero. A
partial sum or TwoSum intermediate outside binary64 defers the original finite
operands instead of storing infinity. Exact additions retain the ordinary sum
and allocate no correction buffer.

The collector groups corrections by scalar coefficient ID, combines each final
partial sum with its tails and rounds the exact integer sum once. Dense matrix
cells and path diagonals/lower/upper bands keep their existing allocations.
Both real and imaginary parts, including symmetric off-diagonals, are covered.
Final true overflow is still an error; exact zero still reaches the normal
singularity policy. Invalid operands poison the collector rather than allowing
a later finish to publish a prefix.

The existing fixed-integer accumulator is parameterized without relaxing its
normalized 18-limb flavor or its public-to-parent admission bound of four.
Assembly uses a separate 34-limb wrapper over the same checked carry, borrow,
sticky-bit and ties-to-even rounding implementation. This spans finite binary64
coefficients down to the minimum quantum, plus a usize-bounded accumulation
carry. The type occupies at most 576 bytes and is reused per corrected scalar;
this is not a whole-solve stack measurement.

Scratch storage is a flat vector of actual tails/deferred operands, not a matrix
snapshot or a wide accumulator for every cell. With K retained parts, grouping
uses comparison sorting and costs O(K log K); storage is O(K). Tests establish
zero correction-buffer capacity for 100,000 exact additions, not zero total
solver allocations or a measured speedup. Heterogeneous million-node overhead
needs separate benchmarking.

Cancellation is checked before/after grouping, during replay and scanning, and
before final handoff. The standard sort itself is not cooperatively interrupted.
Cancelled assembly discards private working state; it does not restore an
in-place matrix, survive process termination or resume from durable checkpoints.

## Verification

Seventeen new test functions cover nine private units, five public Solver tests
and three Engine integrations. Their controlled cases include:

- Independent damped/undamped oscillator references; dense and path layouts,
  element-order/orientation changes, signed loads and borrowed/owned equality.
- Common binary model scales from `2^-900` through `2^900`, displacement,
  velocity, acceleration and member-force readback.
- Parallel-member symmetric off-diagonals, a deterministic independent i128
  lattice oracle, full-range tails, exact zeros, midpoint/sticky/carry rules,
  and finite results after a partial overflow.
- True singularity, true overflow, invalid operands, forced accumulator capacity
  poisoning and impossible correction-cell IDs.
- Start/end grouping, mid-replay and final-handoff cancellation; unchanged
  borrowed requests, fresh complete reuse and late-frequency failure.
- Serialized Engine dispatch and workflow output readback; cancellation and
  failure cannot return a successful frequency prefix through these APIs.

On macOS AArch64 with pinned Rust 1.88.0 and locked offline dependencies:

| Completed scope | Passed | Exclusions |
| --- | ---: | --- |
| Solver library | 899 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded |
| Engine library | 711 | 1 ignored |
| Harmonic integrations and dynamic physical controls | 97 | Selected integration targets |
| Total | 1,707 | Not full workspace, installed or all-physics acceptance |

All-target Solver/Engine Clippy with warnings denied passes. The nine private
assembly tests and eight new public/Engine tests are subsets of this table.
The registered local assembly profile also checks ten existing normalized and
expression-accumulation tests; its 27 tests are not additional to the broad total.

The source evidence is registered at `runtime-engine-solver` with numerical
validation and recovery dimensions. Catalog inspection validates 102 profiles
without executing them all; the new profile executes its three commands.
Tensor checks retain 13 modules, 11 paradigms, zero structural gaps, two maturity
gaps, 23 evidence-grade gaps and 18 P0 gaps; overall qualification stays blocked.
Book, documentation inventory, formatting and 800/2,000-line organization checks
pass, with zero tracked line-limit debt. No release qualification is promoted.
Linux, Windows, installed-runtime, scale, forced-kill and physical-material
validation are not performed in this follow-up.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_assembly
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_assembly
kyuubiki-script-runner check-operator-validation --profile harmonic-assembly-local-reliability --execute --out tmp/harmonic-assembly-profile.json
```

## Scope Boundary

This is bounded final rounding of sums of represented harmonic coefficients, not exact frequency products, exact factorization, a whole-solver forward-error certificate, independent physical qualification or durable restart.

Frequency/coefficient products already rounded before assembly are not made
exact. Existing row normalization, elimination, output-norm rounding and
original-equation acceptance retain their own boundaries. The analytic reference
above follows norms of separately rounded response components, not a correctly
rounded certificate of the continuous complex norm.
