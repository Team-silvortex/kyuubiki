# Modal bounded lattice construction and recovery checks

October 5, 2026. Local macOS ARM64 research continues the three unresolved
ThreeLayers 128-member inputs without changing their roots, ordinary seeds,
internal residual gate, physical norm gate or independent physical JSON checks.
Package and documentation metadata remain daji 3.4.5; checkout HEAD is 3.4.6.
This work does not bump, install or publish a release.

This is test-only bounded lattice research, not production modal admission.

The retained supplementary-beam pipeline still completes 39 of 42 independent
physical readbacks. The new construction publishes two alternative directions
for the already recovered 100-member tiny input, not two new inputs. All three
128-member inputs still reject internally. The new modules have test-only
ancestry; production Solver selection and Engine/Agent ownership are unchanged.

## Construction and bounds

The previous experiment splits at most 64 beam branches among one to four
passes. This experiment changes the integer coordinate basis instead. A fixed
anchor, original floating-point grid and original radius remain frozen. Integer
column swaps and shears preserve the lattice before the original-radius filter;
there is no post-search amplitude scaling or recipe-specific dispatch.

The first policy reduces adjacent pairs with one or four preselected alternating
sweeps, at most eight pair steps per visit. A sole rounded QR fit then proposes
at most 64 directions. Every eligible direction is reconstructed from the
original seed and integer grid and certified with the actual sparse operator.

The second policy starts from rounded QR, applies bounded triangular size
reduction and adjacent swaps, and reconstructs the resulting columns from the
original Wide columns and exact integer transformation. A fresh rounded QR
factor supplies proposals; approximate triangular scores never certify results.
This is a bounded, possibly unfinished lattice reduction, not a claim that the
complete basis satisfies all lattice-reduction inequalities.

Both policies cap transform coefficients and shear quotients at `2^20`. The
triangular policy caps reduction at 4096 steps. Its strict diagnostic declined
all twelve difficult-input routes before any actual candidate receipt: eight
step limits and four integer limits. The bounded policy explicitly retains
the last whole, safe transformation at those limits. It does not apply a
partially overflowing shear or swallow arithmetic faults and cancellation.

`Complete`, `StepBudget` and `IntegerBudget` distinguish construction outcomes.
A stopped basis still needs reconstruction, a new factor and final actual
receipts. The twelve retained routes stop eight/four times at the step/integer
bounds, with zero completed reductions. A limit is not a numerical rejection,
an acceptance certificate or an impossibility proof.

For 256 coordinates and width 64, the pair plan reserves 7,356,416 numeric
payload bytes and 645,922,816 component visits. The triangular plan reserves
8,404,992 bytes and 3,061,841,920 visits, inside its explicitly separate 10 MiB
and 3.1 billion bounds. These conservative per-construction reservations do
not measure elapsed time, heap bookkeeping, caller-owned matrices or RSS and
do not qualify whole-pipeline production budgets. They are not the old beam
plan's cheaper bounds.

The beam width is frozen during preparation. Both policies permit at most
66 actual receipts: one seed, at most 64 proposals and one fresh final receipt.
Mapped integer offsets must remain inside the original `2^22` radius; floating
reconstruction must reproduce those same offsets exactly. Direction norm stays
inside `[0.25, 2]`. Root and ordinary assembled seed bits are never replaced.

## Difficult input results

Four inputs are rebuilt from current requests, using 100 members at scale
`1e-10` and 128 members at scales `1`, `1e14` and `1e-10`. Natural, Reverse and
GridNorm orders are separate cold experiments. There are four distinct inputs,
not 36 newly covered models.

The adjacent-pair policy has 24 routes: three orders and two sweep counts per
input. All reject numerically, after one factor and 66 actual receipts each.
It must not replace the retained pipeline, which already recovers the
100-member tiny input.

The bounded triangular policy has twelve routes, each with two factors and
66 receipts. Two directions on the 100-member tiny input pass both the original
physical hybrid stage and original-numbered independent JSON reconstruction.
Neither direction was seeded by a prior accepted output.

| Order on 100-member tiny input | Internal relative residual | Independent physical readback |
| --- | --- | --- |
| GridNorm | `7.350022359687814e-9` | `8.74036363526548e-9` |
| Reverse | `7.369594100117354e-9` | `8.566180921954366e-9` |

Both readbacks use the unchanged `1e-8` gate and strict physical unit-norm
validation. Natural order rejects this input internally. The two successful
routes are alternate constructions for one already recovered model, not an
increase from 39 to 41 distinct successes.

| 128-member scale | Best adjacent-pair residual | Best bounded triangular residual |
| --- | --- | --- |
| `1` | `1.1557142471390426e-8` | `1.151085440932885e-8` |
| `1e14` | `1.266016122206376e-8` | `1.2634743488791237e-8` |
| `1e-10` | `2.3156505249196856e-8` | `1.8402575614941376e-8` |

These are minima across separately preselected orders, not a deployed dynamic
selector. Compared with the earlier breadth/iteration diagnostics, the tiny
128-member minimum decreases from `2.130546151660412e-8`, but no unchanged
internal gate passes and no physical fit starts. Negative bounded searches
do not establish that no representable direction exists.

## Failure and recovery coverage

Small independent controls verify a determinant magnitude of one, integer
offset mapping, exhaustive two-coordinate objectives, sign and binary scaling,
and twelve signed/scaled known-null constructions. Strict and bounded reduction
limits are tested separately. Invalid sizes, widths, radii, tolerances, seeds,
retained arithmetic ranges and integer coefficients fail before publication.

Synthetic receipt controls inject twelve callback faults and sixty malformed
receipts across both basis policies. Two stale final gates reject. Six search,
substitution and final-validation cancellations retain eighteen fresh exact
replays. Width-64 controls reach all 66 actual receipts while retaining the
original anchor and radius; no stale candidate is returned after a failure.

The real GridNorm route is reconstructed under numbering keys 7 and 113. Root,
ordinary seed, order, factor/receipt counts, canonical physical metric and
geometry-aligned shape bits must agree. Each numbered result independently
passes the physical gate; independent readback residuals are not required to
be bitwise identical across assembly order. Within each fixed numbering,
healthy JSON and readback replays must be bitwise exact. The observed key-7/113
readbacks are `8.74036363526548e-9` and `8.740363635265486e-9`. The selected
GridNorm route records internal/physical factor counts `[2, 1]` and receipt
counts `[66, 23]`; fresh key-113 replays preserve JSON and readback bits.

Real cancellation targets include reduction step 64, first substitution,
the final internal receipt, internal validation, physical validation, last
original-numbered node restoration and final publication. The three failed
128-member requests are rebuilt under fresh numbering and must still stop
internally without a physical fit or partial output; each is followed by the
healthy case. Separate real final-product controls cover sign reversal,
four callback faults, five malformed final receipts, a lost final gate and
cancellation inside the final sparse product, with eleven fresh exact recovery
checks. Seven publication-chain cancellations and three failed requests retain
ten further fresh JSON replays, with no partial outputs.

## Reproduction and verification

Run from the repository root:

```sh
cd workers/rust
cargo test -p kyuubiki-solver --lib --release --locked --offline -- --nocapture modal_material_lattice modal_triangular_lattice modal_pair_lattice modal_material_triangular_lattice modal_material_pair_lattice
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo test -p kyuubiki-solver --lib --locked --offline -- --nocapture modal_material_lattice
cargo clippy -p kyuubiki-solver -p kyuubiki-cli -p kyuubiki-script-runner --all-targets --locked --offline -- -D warnings
cargo test -p kyuubiki-script-runner module_function_tensor --locked --offline
```

All final verification commands pass:

- Focused optimized lattice controls: nine passed, zero failed, 9.86 seconds.
- Complete optimized Solver library: 759 passed, zero failed and 34 existing
  ignored benchmarks, 347.08 seconds. This includes the retained 42-input
  supplementary-beam readback comparison with 39 accepted inputs; its old
  38-output exact-retention assertions are unchanged.
- Two real lattice controls with debug Wide arithmetic assertions: two passed,
  zero failed, 307.22 seconds.
- All-target Solver, CLI and script-runner Clippy passes with warnings denied.
- Coverage tensor tests: 21 passed, zero failed, 2.21 seconds.
- Eight native checks/self-tests pass for coverage tensor, operator validation,
  documentation inventory/book and project organization. Source/document limits
  remain 800/2000 lines with zero tracked debt; formatting and diff checks pass.

The tensor retains 13 modules, eleven paradigms, zero structural gaps, four
maturity gaps, nineteen evidence-grade gaps and fourteen P0 gaps; Daji
qualification remains blocked. Operator inventory validates 59 profiles with
`executed=false`, not numerical execution of all 59 operators. Book checks cover
26 HTML files at unchanged development/shipping metadata 3.4.5.

Local test processes overlap. Elapsed suite times are not comparable isolated
numerical benchmarks and imply no speedup, remote performance or measured RSS.

## Remaining obligations

Keep the positive 39-input retained pipeline unchanged. The bounded lattice
experiment improves one diagnostic margin and supplies alternative checked
directions on an already accepted input, but does not resolve the three
128-member internal failures or justify production default replacement.

General topology, multiple modes, complete cumulative production budgets,
public Solver admission, current Engine/Agent and Headless journeys, remote
performance and installed-app qualification remain separate open scopes.
Further construction work should retain exact integer transformations and
independent final receipts, not increase coverage by counting alternative
routes or relax physical acceptance conditions.
