# Composite Heat Projection: 2026-09-20

## Scope And Reproductions

Base revision: `96231757`. This local change repairs the Rust SDK study-level
bridge between the existing scalar electric operators and nodal heat loads.
The SDK remains a protocol/client library; real-solver integration tests live
in the CLI crate, which already depends on both SDK and solver. No runtime
solver dependency was added to the SDK.

Eight of the first nine regressions failed before the fix.

- On a unit split quad with potentials `[0, 1, 0, 1]`, the mean field is zero
  but both triangular subcells have `|E|^2 = 2`. Dielectric projection gave
  `0 W` instead of `3.7830101885227592e-6 W` at `1 MHz`, relative permittivity
  `3.4`, loss tangent `0.01`, and unit volume.
- An unequal-area quad similarly gave zero instead of
  `3.546572051740087e-6 W`. Its subcell areas are `1, 2`, squared fields
  `1.25, 0.3125`, and volume `3`; the area-mean squared field is `0.625`.
- Solved-current projection accepted a `NaN` seed load and a negative source
  power. Nonfinite balance errors could bypass `error > tolerance` checks.
- A real impedance-terminal solve dissipated `1 W` in its terminal while the
  projection returned success after mapping only `1 W` of bulk Joule loss.
- An unrelated `2^60 W` seed load caused both solved-current and prescribed-
  current projection to reject four correctly representable `0.25 W` additions.
  Subtracting whole-model sums had erased the added watt.
- Temperature feedback returned infinite conductivity for finite positive
  resistivity `1e-320` rather than rejecting the nonrepresentable reciprocal.

## Numerical And API Contract

Dielectric heating now recovers the spatial mean-square field from integrated
energy: `E_rms^2 = 2 * electric_energy_density / input.permittivity`. The solver
already integrates the two subcell energies before averaging. A mean vector
cannot recover this quantity when subcell directions cancel. Input voltages
are still interpreted as temporal RMS harmonic amplitudes; this is not a new
frequency-domain field solve. Nonfinite/negative energy, invalid permittivity,
an inconsistent mean-field bound, or unrepresentable derived loss fail visibly.
An old unit fixture was corrected to carry energy consistent with its field.

The three heat-load projectors share checked equal-quarter distribution.
Negative/nonfinite powers, duplicate/unknown target nodes, nonfinite or
unrepresentable nodal additions, and an unrepresentable positive-power sum are
rejected. Compensated non-negative summation bounds aggregation roundoff.
Global added power is the sum of actual per-node differences, not the difference
of two whole-model totals. Region evidence also measures actual additions.
Zero expected power is an exact-zero case, not an epsilon-sized exception that
could silently discard every sufficiently small heat source. The relative
conservation tolerance remains `1e-12`; unrepresentable increments are errors,
not silently rounded successes.

Finite negative seed loads (cooling) remain valid in additive Joule projections.
The existing dielectric distributor deliberately replaces seed loads; current
and prescribed-current Joule projectors add afterwards. That ordering is now
documented, not changed. Models are cloned before distribution, so a rejected
projection does not partially mutate the caller's seed. Spatial volume weights
use origin-relative quad area and normalized volume fractions to avoid common-
coordinate cancellation and unnecessary intermediate product overflow.

Contact and terminal results require explicit study-level heat mappings.
The automatic projection now rejects both, including declarations in the
source request, instead of silently dropping a terminal's dissipated power.
The electric solver's contact/terminal capabilities remain available.

## Validation And Limits

All 16 new cross-component integration tests pass, using real electrical and
thermal solves where applicable. They cover opposing/unequal-area subcell
fields, `+/-2^40 V` reference shifts, large coordinate translation, zero loss,
corrupt energy/power/summary values, reciprocal overflow, tiny swallowed power,
shared-node multi-region addition, finite cooling loads, and clean replay.
Analytic power comparisons use relative tolerance `1e-14`; the nodal thermal
response after dielectric projection is checked to `1e-12` absolute tolerance.
The SDK library's 270 unit tests also pass. All 39 existing material-exploration
runner tests pass, including the iterative composite electrothermal research
example. The 16 new regressions also pass in the optimized release build.

Both registered validation profiles were executed through the native runner:
11 electromagnetic-plane-patch commands and eight conduction-screening
commands passed. These overlap with the direct runs and are not summed into a
unique-test coverage percentage. Strict Clippy for the SDK library passes with
warnings denied and no lint exemptions. Formatting, validation-profile
structure, tensor structure, project organization, documentation inventory and
`git diff --check` pass. Source/document line limits remain 800/2,000 with no
tracked debt. The overall tensor daji status remains blocked: four maturity,
16 evidence-grade and 11 P0 gaps are not promoted by this local evidence.

Equal four-node lumping remains the study's spatial approximation. This is not
consistent triangular source quadrature, adaptive transfer, or a claim that a
single coarse element resolves a nonuniform thermal field. Selected conductor
regions remain explicit. No interface heat mapping or general material qualification is claimed.
No remote, installed-app, cross-platform or million-node run is claimed.

Existing development artifacts are not bulk migrated or backed up. If an old
nonuniform dielectric result will be reused, recompute it with the corrected
solver/projection; disposable development outputs may instead be discarded.

## Reproduction

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test composite_heat_projection
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-material-explore
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --release --test composite_heat_projection
./scripts/kyuubiki check-operator-validation --execute --profile electromagnetic-plane-patch --out tmp/composite-heat-electromagnetic-20260920.json
./scripts/kyuubiki check-operator-validation --execute --profile electric-conduction-plane-quad-screening --out tmp/composite-heat-current-20260920.json
```

The integration regression is registered in the electromagnetic-plane-patch and
electric-conduction-plane-quad-screening validation profiles. Tensor evidence
is bounded to local numerical validation, not a broader qualification promotion.
