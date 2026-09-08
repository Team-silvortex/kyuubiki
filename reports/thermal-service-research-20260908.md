# Installed Thermal Research and Readback: 2026-09-08

## Verdict and scope

**Installed candidate numerical verdict: PASS, 302/302 cases.** The official
Rust Headless SDK submitted real distributed workflows to an isolated,
Installer-installed Orchestra/Agent pair on Linux. No mock or local-solver
fallback was requested. All seven workflow nodes, output contracts, analytical
gates and immediate result readbacks passed for every case.

| Stage | Result | Meaning |
| --- | --- | --- |
| Fresh installed-service study | 302/302 pass | 14 layered cases and 288 thermal patches |
| Read-only check before restart | 302/302 pass | Original job ids, contracts, exact JSON values and numerical gates |
| Agent unavailable | 14/14 expected execution failures | No result fabricated; execution causes preserved |
| Read-only check after both process restarts | 302/302 pass | Original results, not resubmitted replacements |
| New layered jobs after restart | 14/14 pass | Recovered service can execute new valid requests |

The unique physical matrix remains **302 cases**, not the sum of repeated
readbacks and recovery rounds. The qualification remains
`synthetic_reference_not_material_certification`. Previous source-only and
old-installed-service reports remain unchanged historical evidence:
[source matrix](thermal-wide-research-20260908.md) and
[initial 0/14 finding](layered-thermal-research-20260908.md).

## Defects exposed and corrected

1. **Compute-only installation lacked an explicit inventory contract.** A
   `headless` service profile now requires Agent and Orchestra without inventing
   a frontend. Omitted/desktop profiles still require all three services;
   unknown profiles and contradictory frontend entries fail. Installer sealing,
   activation and runtime resolution validate declared entrypoints. Unit tests
   also exercise desktop/headless rollback inventory in both directions.
2. **The research runner hid execution causes behind numerical-validation
   errors.** Failed/cancelled jobs now retain their execution stage, message,
   status detail and full terminal response. The live negative round retained
   `agent_process_unavailable`, `econnrefused`, and the failing heat node for all
   14 cases. Those are terminal failed jobs; automatic resumption is not claimed.
3. **SDK JSON save/reload could change floating-point values by one ULP.** A
   successful 14-case round failed exact retained-result readback in 13 cases.
   Decimal decoding without `serde_json/float_roundtrip` reproduced mismatched
   IEEE-754 bits, including `1.8181818181817597`. The SDK now enables correct
   round-trip decoding. Tests cover measured values, ten successive save/reload
   cycles, signed zero, subnormals, finite extremes and finite samples from
   8,192 deterministic bit patterns. No physical or equality tolerance was
   relaxed and no historical result file was rewritten to conceal the drift.

The first candidate smoke failure was a **test-deployment mistake**: its Agent
cache pointed at the parent of the required managed `packages` directory. Agent
startup correctly rejected that layout. The retained 14 failures exposed the
runner's diagnostic masking, not a new physical-solver failure. Fixing the cache
configuration allowed all 14 numerical cases to pass before the readback defect
was diagnosed. An older wide run was then intentionally interrupted, retaining
191 rows with `complete: false`; it is not counted as a completed matrix. The
final 302-case round uses the corrected SDK and a new evidence directory.

## Physical coverage and measurements

The same reviewed model/checkers serve the source-engine tests and public SDK
example. Expected values come from closed-form heat conduction, integrated
thermal expansion, and plane-stress equations, not another execution of the
solver under test. No external solver or experiment was run.

- The 14 layered cases cover two conductivity contrasts, four aligned Q4 mesh
  sizes, two temperature origins, zero heating and doubled heating; at most
  153 mesh nodes. The layered axial reference uses zero Poisson ratio.
- The 288 patches cover triangle/Q4, Poisson ratios 0/0.25/0.45, cooling/zero/
  heating, two temperature origins, free/clamped restraints, node/element
  mapping, and uniform/alternating modulus. Each patch has nine mesh nodes and
  prescribed uniform temperature, so this is not heat-convergence qualification.

| Installed-service metric | Maximum error | Acceptance limit |
| --- | ---: | ---: |
| Layered temperature | 1.14255e-11 K | 1e-7 K |
| Layered axial displacement | 1.10236e-16 m | 1e-9 m |
| Layered heat flux | 7.68658e-9 W/m2 | 1e-6 max(abs(q), 1) W/m2, per case |
| Patch displacement | 8.23994e-18 m | 1e-10 m |
| Patch normalized stress | 5.53960e-14 | 1e-8 |
| Patch temperature and heat flux | 0 | 1e-8 K / 1e-8 W/m2 |

Stress normalization uses `max(E * alpha * abs(rise), 1 Pa)`. Every readback also
rechecks the output graph contract and these gates, separately from exact JSON
value equality. Small errors under these assumptions do not imply comparable
accuracy for arbitrary geometries or experimental materials.

## Deployment and lifecycle evidence

The candidate payload contains a real native Agent and complete production Mix
release. Native Installer sealed it and installed activation generation 1 into
an isolated store. Docker supervised those installed files read-only; it did
not replace Installer payload validation. The database, cache and release state
were separately writable. Ports 6410/6411 were host-loopback only, with no public
plaintext credential exposure. Containers used an unprivileged user, dropped
capabilities, no-new-privileges, CPU/memory limits and an immutable root.

The base source revision is `c0c59c95543ec51839c42bbb6cbcc888eba92ef6` plus this
round's changes. Candidate package/Mix release is labeled **3.1.0**, whereas the
Rust manifests and public SDK still identify **3.0.0**. This records the tested
provenance, not a completed repository-wide release/version alignment.
Linux Rust is 1.95.0; the offline Mix build used the already-present Elixir
1.19.5 / OTP 28 image rather than the host's older Elixir. The reference study
definition, including the SDK dependency manifest, has SHA-256:

```text
245beec6daf0c4647ecaba7f4e5606a9fa17fa325a481e12e34773d7bffe218c
```

Before the controlled restart, no active jobs, execution leases or queued
requests remained. Agent PID changed from 1464806 to 1498639; Orchestra PID
changed from 1458194 to 1498714. Both container identities and the mounted
installed payload remained unchanged. Orchestra's fencing token advanced from
1 to 2 and its session id changed. The same 302 persisted jobs were then fetched
without submissions. Before/after verifier reports have identical hashes because
they contain the same job ids and results, not timestamps; the separately retained
process lifecycle records establish that the restarts actually occurred.

**Remaining shutdown boundary:** an unsupervised Agent running directly as
container PID 1 did not exit within the 15-second stop window and ended with
exit 137, `OOMKilled: false`. A separate idle probe of the same installed binary
with Docker `--init` exited 143 on termination, with the stop command completing
in under one second. That verifies signal delivery, not graceful task draining.
This is not being reported as a passing in-flight shutdown/recovery test.

After retaining results and logs, both candidate containers and the idle probe
were stopped and removed. Their installed payload, database and all evidence
remain in isolated server state. Existing Agent, Orchestra and deployment
services stayed active, with their original PIDs 3463, 3462 and 3461. They were
not upgraded or restarted. Docker-based lifecycle supervision here is not a
complete remote Installer promotion or rollback qualification.

## Evidence and reproduction

All raw requests, terminal responses, results and build logs remain under the
server's managed `research-runs/thermal-service-20260908/` state, not in Git.
Final evidence directories are `evidence/all-ieee-service`,
`evidence/all-readback-before-restart`, `evidence/agent-unavailable-negative`,
`evidence/all-readback-after-restart`, and `evidence/recovery-layered`.
Each submitted case records request/result/terminal SHA-256 values. The verifier
rejects failed/incomplete baselines, duplicate job identities, mismatched case
inventory, altered requests/results, symlink files and oversized evidence.
Checksums establish retained-file identity, not externally signed attestation.

| Evidence | SHA-256 |
| --- | --- |
| Installed runtime-payload manifest | `788e16f8d0874e72d0d0457f5f5def250617236862dc2090e8d8b4425377dd00` |
| Installed native Agent executable | `77d983d460927585542ce2fb08f58642681095f2f6a80a2976f522460d0fbdbb` |
| Fresh 302-case report | `0965c0d8cae3f21fc68e854b1ac35f58b0594df20840b3559dd2f12eaa745649` |
| Before/after readback report, each | `251b67464c861e9214a912e35d9f3ebbf439e4fe038ab189b35f118db0b40a49` |
| Agent-unavailable negative report | `c526d89b7c9803eb43d7685bc848cfa0063baf7c84fe80cbbb6ac9b9d5bbde8c` |
| Recovery fresh-job report | `e6eb3756bde85f4ea7f202f4a818030baebfbf69d7399b929d18f3964e6a11fc` |
| Lifecycle record before restart | `3339f45987e5da7d8479491716c190513756f5381a1797fa3e40a6cfe8d8bcfa` |
| Lifecycle record after restart | `d6873cfa11892dc0a630fe0b572125606e9470c0848dc31092a1a7a8e44dcebf` |

With a reviewed service pair running, configure the local endpoint through
`KYUUBIKI_BASE_URL` and run from the repository root:

```sh
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example layered_thermal_research -- research-round-003 all
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example verify_layered_thermal_research -- research-round-003 research-readback-001
```

Each output directory must be new. Preserve lifecycle evidence and wait for all
jobs to be terminal before externally restarting services, then use a second
new readback directory. Neither example performs a service restart. See the
[HTML research tutorial](../docs/research-layered-thermal.html) and
[headless payload instructions](../docs/packaging-and-deployment.md).

## Regression checks and unqualified boundaries

Linux checks passed: 127 Installer tests, 45 desktop-runtime tests, one shared
profile test, 87 public SDK integration tests, and seven research-example tests.
One desktop-runtime test was explicitly ignored because it starts the real
local stack; it is not counted as passed. macOS ARM64 with minimum-supported
Rust 1.88 passed all ten selected floating-point/example tests. Offline SDK
package verification also passed. Three-platform payload metadata tests are
not physical Windows execution evidence. Early fixture-only failures in the
selectively staged remote checkout were resolved by supplying the repository
fixtures before the final full runs.

The coverage tensor now links the numerical service report, headless inventory,
exact-float regressions and retained-result verifier. These additional anchors
do not promote global maturity, usability, release or power-loss gates.

Still unqualified by this round: real material calibration; independent solver
or experimental agreement; general distorted-mesh convergence; nonlinear,
contact and transient coupling; large meshes or multi-Agent concurrency;
in-flight task draining/recovery; physical power loss and database corruption;
and end-to-end remote Installer promotion. These are the next research and
reliability boundaries, not hidden inside the 302/302 result.
