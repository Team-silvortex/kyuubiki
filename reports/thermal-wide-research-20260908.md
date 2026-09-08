# Expanded Thermal Research: 2026-09-08

## Result and scope

**302/302 analytical reference cases passed on the Linux server's isolated
current-source Rust engine:** 14 layered-conduction cases plus 288 thermal patches.
This is source-engine execution, not a newly deployed Orchestra/Agent service
acceptance run. The old installed services and earlier failed evidence were not
replaced, restarted or overwritten.

The source suite uses the same graph/model/checker code as the public Rust
Headless SDK research example. `thermal-patch` and `all` are now explicit SDK
example suite options. Public graph validation and example compilation passed;
that does not claim those 288 jobs were submitted to an updated installed service.

## Bugs reproduced and corrected

1. Rust's default triangle element mapping requested `node_l`, which does not
   exist on triangles. Omitted index selections are now resolved per shape.
2. Truncated result-element arrays passed bridge validation. Element counts and
   source connectivity are now checked before reduction. Elixir also rejects
   mismatched node counts rather than silently truncating `Enum.zip`.
3. Malformed scale/default/reduction/index values could silently fall back,
   discard entries, or double-count nodes. Both contracts now reject malformed
   values and duplicate field selections; malformed Elixir sections return an
   error instead of raising through `Access.get`.
4. Floating-point reduction overflow in Rust could serialize a successful
   `null` temperature. Overflow now fails locally; native typed NaN values are
   rejected rather than defaulted. Elixir arithmetic overflow becomes an error
   result, and a subsequent valid request still succeeds.
5. Elixir accepted negative/out-of-range/missing element indexes and coerced bad
   physical values to zero. These inputs now fail before list indexing or math.

Before correction, the new Rust integrity suite failed 4/5 tests; the new
Elixir integrity suite also failed 4/5 tests. After correction, the expanded
Rust integrity suite passes 6/6 and the combined Elixir suites pass 7/7.
Contract parsing was separated from mapping so these checks do not push either
implementation toward the per-file source limit.

## Coverage and measurements

The patch matrix is 2 shapes x 3 Poisson ratios x 3 temperature rises x 2 origins
x 2 restraints x 2 mapping modes x 2 modulus patterns = **288 cases**.

- Shapes: 2D linear triangle and Q4; each mesh has nine nodes.
- Poisson ratios: 0, 0.25, 0.45; temperature rises: -20, 0, +20 K.
- Origins: 20 and 293.15; restraints: free expansion and fully clamped.
- Mapping: nodal copy and element mean with default shape indexes.
- Modulus: uniform 70 GPa or alternating 3/70 GPa; alpha is uniformly 12e-6 /K.

The prescribed uniform thermal field has exact free displacement
`u = alpha * rise * [x,y]`, or fully clamped plane stress
`sigma_x = sigma_y = -E * alpha * rise / (1-nu)`. Shear stress and flux are zero.
The stress-free-temperature formulation follows the
[MOOSE thermal-expansion definition](https://mooseframework.inl.gov/releases/moose/2021-05-18/source/materials/ComputeThermalExpansionEigenstrain.html).
No external solver was executed.

| Retained patch gate | Maximum error | Limit |
| --- | ---: | ---: |
| Displacement | 8.23994e-18 m | 1e-10 m |
| Stress normalized by max(E alpha abs(rise), 1 Pa) | 5.53960e-14 | 1e-8 |
| Temperature / mapped temperature rise | 0 K | 1e-8 K |
| Heat flux | 0 W/m2 | 1e-8 W/m2 |

All 14 layered cases also pass. Maximum nodal temperature error is 1.14256e-11 K,
and maximum axial displacement error is 1.10210e-16 m.

Additional verification includes 72 mapping combinations across unequal cell
areas, positive/negative/zero scale, two temperature origins and six reductions;
malformed-input and wrong-signed-stress counterexamples; and 24 existing Rust
bridge unit tests on macOS. The full Linux engine unit suite additionally passed
451 tests with zero failures; one dynamic-host test was ignored because it
requires a prebuilt operator-template cdylib. That ignored test is not counted
as verified. All 11 selected Linux integration tests passed, including the 302
reference cases, and all seven selected Elixir tests passed. Public SDK examples
compiled and their new suite/graph-contract test passed.
Linux runs used rustc 1.95.0; macOS used 1.88.0.
No broad Windows, complete Elixir application, restart or large-mesh qualification
is claimed by this round.

## Retained evidence and reproduction

Full requests/results remain in the server's Kyuubiki research state under
`research-runs/thermal-wide-20260908/evidence/{layered,thermal-patch}/`.
Reports identify `current_source_rust_engine_integration` and
`installed_service_verified: false`; each case has canonical JSON SHA-256 input
and output digests. Report file SHA-256 values:

```text
thermal-patch: db2a5879c3d0de7e99f15445b7a0816a5824c42b8899cca88c66823c3fd82dd1
layered:       3aca428a6cda275f996bc8d13cf02f9257f9ee6dce51dd6024a8dc1db1ab12e5
```

The worktree is based on `ee18a058d968c34b6401e88ebffc22fc75d61ad8` plus this
round's uncommitted corrections. Source digests were checked to agree locally
and remotely:

```text
Rust heat_bridge.rs: efc0b2749f6aafb6926338e6dd3c7d4f7da414e649dfea597171479b5ee5b46d
Rust heat_bridge_contract.rs: 7bc815ec1cbb594e42c7fcd2084413b0bc5d5b100f49e75d1efb597ae853a77e
Elixir workflow_operator_heat_bridge_runtime.ex: 9d576e9927336adcf360d8f70e5b30f11741b2bb50137c9ba3a7f9a72f63929f
Elixir workflow_heat_bridge_contract.ex: dc64ad925c978315a21417baf5d5b1aa6e8e19fa62040a73ae32bff81d8b6036
```

Native source verification from `workers/rust`:

```sh
cargo test -p kyuubiki-engine --lib
cargo test -p kyuubiki-engine --test layered_thermal_research \
  --test heat_bridge_integrity --test thermal_patch_research
```

To retain results, set `KYUUBIKI_RESEARCH_EVIDENCE_ROOT` to an existing parent
directory whose `layered` and `thermal-patch` subdirectories do not yet exist.
Without that variable the regression does not write evidence files. Reports
remain incomplete until a suite finishes; these files are not a transactional
database or proof of restart durability.

Next acceptance gate: Installer-managed deployment of corrected components and
a new public-SDK service round. Then cover nonuniform expansion coefficients,
material interfaces, realistic calibration, nonlinear/contact behavior and
independent numerical/experimental comparisons. This round is semantic and
constitutive verification, not proof that simulation results cannot be wrong.
