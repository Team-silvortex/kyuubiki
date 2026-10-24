# Harmonic RHS And Back Substitution Updates

Date: 2026-10-24

Source: daji 3.7.0 after `60115afa`, retaining the uncommitted arithmetic and
[ordinary division](harmonic-fast-division-20261024.md) follow-ups. RHS elimination
and back-substitution numerators now use fused scalar updates with a local
rounding-risk replay rule. This fixes selected weak-response errors that a
correct final division could not repair. Standalone products, matrix updates,
Engine production code, Agent, TaskIR and operator contracts are unchanged.
No version bump, publication or desktop rebuild is included.

## Failures Before Repair

Two private tests and one public operator test failed before implementation.
For `d=2^-27`, the exact product `(1+d)*(1-d)=1-d*d` rounds to one when stored
as binary64. With `w=2^-20`, evaluating `1+w-product` then produces `w` rather
than the representable `w+d*d`. The numerator is larger than the existing
`2^-26` relative replay trigger, so the ordinary path accepts it. Correctly
rounding the subsequent division by three cannot restore that missing tail.
The independent reference is `(w+d*d)/3`, not the program's residual.

The public example is a grounded chain with two springs of stiffness `1-d`
and `1+d`. The middle and tip loads are `2*(1-d)` and `-1+w`; frequency is zero.
The free stiffness matrix is `[[2,-(1+d)],[-(1+d),1+d]]`. Its infinity-norm
condition number is `(3+d)^2/(1-d*d)`, below ten. This is not an almost singular
matrix, although the small tip response is componentwise sensitive to cancellation.

The exact tip displacement is `2*(w-d*d)/(1-d*d)`. At `w=2^-20`, its rounded
reference is approximately `1.9073486327014777e-6`; the old successful result
was `1.9073486328125e-6`, about `5.8e-11` relative error. The absolute error
remains small and passes the existing original-equation tolerance. Independent
component references expose what a global backward-residual gate does not.

The final public test uses rationally derived reference bits for `w=2^-18`,
`2^-20` and `2^-24`, with a four-epsilon relative comparison. This allows
remaining factorization rounding; it does not assert exact whole-solve bits.
Both signs of all loads, reversed element orientation and borrowed/owned
execution are covered.

## Fused Updates And Replay Routing

The new private row helper replaces separate complex-product rounding followed
by subtraction only for finite RHS updates and back-substitution numerators.
Each complex term uses two scalar FMAs per component. A single FMA retains
the product tail through subtraction in the first example, so the represented
weak numerator is preserved before final division.

FMA alone is insufficient. For example, first subtracting `d*d` from `1+w`
rounds that partial result back to `1+w`; subtracting one afterward again loses
the tail. The helper therefore accumulates a conservative local rounding
estimate per component. Each nontrivial FMA contributes an upward-padded
`epsilon*abs(result)` amount; both multiplication and positive accumulation
are padded upward with `next_up`. A rounded-zero FMA still contributes a
minimum quantum. Known zero products leave the value and estimate unchanged.

If the estimate exceeds eight epsilons of the final represented component,
the entire expression is replayed through the existing aligned integer sum.
The existing strong-cancellation routing remains, along with finite/product
range guards and rejection of nonzero subnormal ordinary intermediates.
This eight-epsilon policy selects arithmetic replay; it is not a physical
acceptance tolerance, an editable regularization setting or a whole-solve
forward-error certificate.

All later operands are still validated even after an earlier prefix becomes
unsafe. Long ordinary scans check cancellation every 1,024 terms; complete
replay retains its existing bounded checkpoints. State is private until success,
and fresh inputs can be reused after cancellation or failure.

The helper is a 70-line private numerical module with fixed per-row scalars,
no new row vector, matrix snapshot, heap allocation on success or dependency.
Existing RHS storage promotion is unchanged. Additional FMA, rounding checks
and replay decisions have costs; no performance neutrality or speedup is claimed.
RHS update bits may intentionally change. Standalone product/matrix-update bit
controls, pivot constants and original-equation admission remain unchanged.

## Regression Coverage

Twelve new test functions cover seven private units, two public Solver tests
and three Engine integration tests:

- Signed real/imaginary weak numerators at three common binary scales, plus
  direct RHS subtraction before final division.
- Complex partial-sum cancellation and a two-term ordering control that sends
  earlier lost tails to replay while retaining a safe fused-order path.
- Dense and path triangular systems with independent weak-response references.
- Invalid later operands, true range failure, a 2,050-term replay cancellation,
  cancellation during a long ordinary scan and clean explicit reuse.
- The public two-spring closed form at three weak-load levels, both load signs,
  reversed element orientations and borrowed/owned equality.
- Byte roundtrips, direct Engine dispatch and input/solve/output workflows,
  retaining the independently checked weak tip response.
- Factor/substitution/result-boundary cancellation and a later frequency range
  failure: no successful result prefix, reproducible failure and fresh recovery.

Cancellation checks are cooperative in-process tests, not forced termination
or persisted power-loss recovery evidence. The public case is a small analytical
spring model, not independent material or large-scale physical qualification.

## Platform Verification

Pinned Rust 1.88.0 and locked offline dependencies reuse existing build directories
on macOS AArch64 and Linux x86_64. Linux uses the preceding source overlay, not
a clean Git checkout or installed runtime. The existing arithmetic source
matched its preceding digest before transfer, and the four new source/test
files were absent. All five transferred files matched local SHA-256 digests.
No deployment configuration or credentials changed.

| Completed final-source scope | macOS AArch64 passed | Linux x86_64 passed | Exclusions |
| --- | ---: | ---: | --- |
| Solver library | 864 | 864 | 9 ignored; 292 `modal_frame_spectrum::` tests excluded per platform |
| Engine library | 711 | 711 | 1 ignored per platform |
| Existing Engine constraint integration | 16 | 16 | Selected harmonic scope |
| Previous quotient and division integrations | 15 | 15 | 9 Engine and 6 public Solver tests |
| New row-update integrations | 5 | 5 | 3 Engine and 2 public Solver tests |
| Existing public complex-force integration | 7 | 7 | Selected force scope |
| Frequency range, balance, reliability and dynamic closed forms | 30 | 30 | Selected integration targets |
| Total | 1,648 | 1,648 | Not full workspace, installed or all-physics qualification |

macOS all-target Solver/Engine Clippy with warnings denied, Rust formatting and
diff whitespace checks pass. Historical reports retain their earlier source
scope, arithmetic and counts.

The native catalog validates 98 profiles without execution. The new profile
separately executes its three commands and all 21 selected tests successfully.
The 26-file HTML book, documentation inventory and 800/2,000-line organization
audit pass with zero tracked line-limit debt. Tensor structure and command
checks pass for 13 modules and 11 paradigms; qualification remains blocked with
zero structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

## Reproduction

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_row_update
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib harmonic_fast_division
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver -p kyuubiki-engine --test harmonic_row_update
```

Profile: `harmonic-row-update-local-reliability`, in the registered
`harmonic-cancellation.json` shard. Its three commands select seven new row units,
nine existing division units and five new integration tests. These 21 tests are
a subset of the table, not additional tests to sum.

## Remaining Boundary

This is bounded harmonic RHS and back-substitution numerator hardening, not exact matrix updates, exact unrounded-expression division, a whole-solver forward-error certificate, independent physical qualification or durable restart.

Assembly, matrix Schur updates, normalization fallback and previously rounded
RHS values can still lose information. Replay rounds its numerator before the
final divide; correct rounding of that represented numerator's quotient is not
the same as rounding the whole unrounded row expression once. Complex rotation
at division may amplify earlier component error. Independent physical references,
all-model forward-error budgets, scale performance, installed/SDK journeys and
persisted recovery require separate evidence. No qualification threshold or
release grade is promoted by these local controls.
