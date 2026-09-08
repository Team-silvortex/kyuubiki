# Layered Thermal Research: 2026-09-08

## Scope and verdict

The public Rust Headless SDK submitted a two-material heat-to-structure graph
to the existing Linux laboratory Orchestra service. The retained workflow has
seven nodes, including real heat and structural solves and three output nodes.
The installed service was not replaced or restarted during this investigation.

**Installed-runtime numerical verdict: FAIL, 0/14 cases.** All 14 jobs completed,
all 14 SDK output manifests validated, and all 14 second result fetches matched.
Successful orchestration did not imply physically correct thermal expansion.

**Current-source correction: PASS in the bounded Rust engine regression.** This
is not an installed-service retest of the correction. Installer-managed rollout
and another full service round remain open.

The synthetic inputs, analytical formulas and exact thresholds are in the
[HTML tutorial](../docs/research-layered-thermal.html). There is no claim of
material certification, external-solver execution or experimental validation.

## Findings

1. The heat-to-thermo bridge ignored `transform.reference_temperature` and copied
   absolute temperature into `temperature_delta`. With a reference of 20, this
   added 0.0003 m to tip displacement, including the zero-rise case. With the
   equivalent Kelvin origin of 293.15, displacement error was 0.00439725 m.
   Refining the mesh did not remove this systematic semantic error.
2. The Rust Headless SDK library embedded a schema example outside its package
   directory. A clean standalone SDK copy could not compile. Library/example
   fixtures are now bundled; contract tests guard parity with repository schemas.
   Bootstrap-document integration tests still explicitly require repository
   documents and the other official SDK surfaces.

The bridge correction is `(temperature - reference_temperature) * scale`,
before node reduction, in both Rust and Elixir. Default reference zero preserves
existing temperature-rise workflows. Nonnumeric references and nonzero
references attached to heat-load/flux sources are rejected.

## Evidence

The final retained baseline is the authoritative round below. Two earlier
14-case exploratory/readback rounds also remain on the server; they are not
counted as additional independent physical coverage.

| Gate | Passing cases | Maximum observed absolute error |
| --- | ---: | ---: |
| Heat temperature | 14/14 | 8.6402e-12 K |
| Heat flux continuity | 14/14 | 8.6871e-9 W/m2 |
| Bridge temperature reference | 0/14 | 293.15 K |
| Axial displacement | 0/14 | 4.39725e-3 m |
| Completed job | 14/14 | Not a numerical gate |
| SDK output manifest | 14/14 | Structural contract only |
| Same-job result readback | 14/14 | Restart durability not tested |

Bounded verification performed:

- Rust engine: 3 integration tests passed, including all 14 analytical cases,
  triangle/quad and node/element conversion, malformed-reference rejection,
  post-rejection reuse, and negative tests of the research validator itself.
- Rust engine: 24 existing bridge-filtered unit tests passed.
- Remote Linux Elixir: 2 standalone contract tests passed, covering both shapes,
  two temperature origins, zero/nonzero rise and two mapping distributions.
  These are bridge unit tests, not a full Elixir application test run.
- Remote Linux public Rust SDK: 84 tests passed with the shared repository
  bootstrap resources present. Standalone `cargo package --locked --offline`
  packaging and verification passed without those repository resources.
- The extracted package also compiled the research example independently.
  Reusing the baseline directory was rejected before submission, and the
  retained report checksum was unchanged after that negative test.

Research definition SHA-256:
`90666f904e71ec87da1dd887f6c8685de4ac66892a95dcfa8561cdf911d50ad3`.
Final baseline report SHA-256:
`67e45689188aca739eb1163acfc437c5e4a641e34a0d50a31875152457383238`.

Current-source tests used worktree changes based on commit
`66326adb08acb763eef6abb20ca0c637200c54ef`; this is not a claim that the commit
alone contains the correction. Tested Rust bridge source SHA-256:
`d52eb18407bf92456a13d6fff50f004f74c0d8f22fca12e5da117722b0d326f3`.
Tested Elixir bridge source SHA-256:
`50f7959b8eeadb1d7400b780d8293a42add3a878d2748e1a396da9ec0f572575`.

The remote installed Orchestra release selector still identifies
`2.15.0-bandit-20260822T123552Z`; the source/SDK line is daji 3.0.0. The runner's
optional source revision was not supplied; no server Git revision is inferred
from the SDK version.
Full requests and results remain in the server's Kyuubiki research state under
`research-runs/layered-thermal-20260908/retained-baseline/`. They are not added to
Git. No endpoint address, host credentials or private service configuration is
part of this report.

Final baseline job ids, in the runner's case order:

```text
52b66c19cef716aa f787b1ce638d4fa9 5ced61f29573d141 2c90dab24071db0a
8d9504f1e1657744 cf66e61ae49287b2 36d3b573058b7640 9c2d4913c4005d81
fceb5e66bc75d7e6 35e834fe51ed2025 21f1da7cfd63681a 3360db5f9d7ffec6
842e84dc504d6384 fd2551b8cc16f157
```

## Next acceptance gate

Deploy the corrected components through Installer and rerun the public SDK
example into a new round directory. Require 14/14 numerical passes, matching
result retrieval and explicit installed component provenance before calling
this service path repaired. Then add realistic material cards and transverse
mechanics, followed by independent numerical/experimental comparisons.

Tensor relevance: `runtime-engine-solver / validation`,
`sdk-headless / workflow_composition`, `orchestra-control-plane / workflow_composition`,
and `verification-evidence / validation`. This bounded regression is linked as
contract evidence; it does not promote general physical qualification or erase
the installed-runtime failure.
