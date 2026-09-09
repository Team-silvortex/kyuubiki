# Planar Result Construction: 2026-09-09

## Verdict and scope

**Installed planar postprocessing: PASS, 36/36 new fault scenarios.**
Qualification: `planar-postprocess-installed-thermal-recovery`. This extends the
[sparse scaling study](sparse-scaling-research-20260908.md) to temperature/
displacement restoration, physical result construction and summary passes in
triangle/quad planar heat and thermal-stress solvers. Qualification remains
`synthetic_reference_not_material_certification`.

| Gate | Executed result |
| --- | --- |
| Four public paths against old result builders | 0 pass / 4 expected failures for ignored cancellation |
| Complete final solver unit/integration run | 747 pass across 153 test targets, 0 failures, 5 explicit tests ignored |
| Final solver units in that total | 277 pass, including 12 new functional tests |
| Optimized release solver units | 277 pass, 0 failures; 5 opt-in tests selected separately |
| Retained old/debug/release physical output comparison | 12/12 records byte-identical |
| Optimized paired product/vector/scaling/result benchmarks | 4/4 explicit pass |
| Final CLI units | 151 pass |
| Cargo-built and Installer-installed cancellation / orphan / lifecycle / shutdown | 51 / 7 / 5 / 8 pass in each final run |
| Cargo-built TaskIR execution/tamper rejection/healthy next request | 1 pass |
| Installed independent mixed-workflow baseline | Pass against preceding verified physical output |
| Fresh official Rust SDK thermal/thermal-stress matrix | 302/302 pass, 0 failures |
| Original-case readback before and after idle restart | 302/302 pass at each stage |
| macOS ARM64 solver/CLI test compilation, Rust 1.88 | Pass |

The five ignored tests are four explicit performance tests and one retained
physical-output probe; all are executed separately. The 71 installed live
passes include shared startup ownership tests. Eight new test functions expand
to 28 cancellation and eight disconnect scenarios, not 36 material models.
CLI units use a Cargo test executable; live tests explicitly select the installed
Agent binary. Neither GUI packaging nor installed macOS/Windows is qualified.

## Control and numerical contract

Eight stages are appended without renumbering existing solver stage IDs:

| Stage | Actual progress unit |
| --- | --- |
| `result_prescribed` | 64 prescribed scalar DOFs |
| `result_free_dofs` | 64 reduced-to-full DOF assignments |
| `result_nodes` | 64 constructed node records |
| `result_elements` | 64 constructed element records |
| `result_node_summary` | 1,024 source node records per maximum scan |
| `result_element_summary` | 1,024 source element records per maximum scan |
| `result_totals` | 64 source elements per sequential sum |
| `result_rhs_norm` | 64 RHS values in the thermal solver's diagnostic norm |

Entry and final short blocks are checked, including entry for empty input.
Thermal displacement restoration has an empty prescribed list; it reaches that
stage's entry, not a positive prescribed count. Separate thermal summaries
reuse the node/element-summary stage but reset its counter per invocation.
Stage/count is not a unique field identifier, whole-job percentage, persisted
numerical state, or wall-clock latency bound. `resumable` remains false.

Cancellation propagates through borrowed, owned and profiled entry points.
Constructed prefixes are dropped rather than returned; full-field restoration
cannot expose a partial scatter. Late summary cancellation cannot publish an
otherwise complete result/profile. Summary work runs before cloning retained
input, avoiding that copy when a summary already fails. The copy itself is not
internally cancellable in this round.

Physical formulas, IDs, indices, record order, geometry and tolerances remain
unchanged. Maximum scans use slice-based folds with the original `f64::max`
behavior. Sums carry one sequential accumulator across blocks and retain the
old negative-zero identity. The diagnostic RHS norm retains its sum-of-squares
formula; this is not a change to a differently scaled norm algorithm. No
parallel reduction, regrouped physical sum or changed convergence criterion
is introduced.

The four red tests fail at their first requested `result_free_dofs` boundary:
the old raw solver returns success inside the observer scope. They do not rely
on the outer control scope filtering a successful return. Final tests traverse
all applicable result stages on all four public paths, then solve again and
compare the complete result. Owned/profile variants, zero-free-DOF heat, short
node/element arrays and the second thermal maximum-summary invocation are
tested explicitly.

Kernel tests cover lengths 0/1/63/64/65/127/128/129/1023/1024/1025, exact
projection counts, entry/interior/final cancellation, dropping every constructed
prefix object, prescribed/free assignment order, sequential rounding, signed
zero and NaN/infinity classification. The opt-in physical probe retains all
four complete outputs for grids 1/7/12 before numerical changes. Its 12 named
records match final debug and release outputs byte-for-byte, independently of
the observer tests. This proves equivalence on those inputs, not equivalence
for every possible material or mesh.

## Installed research

Each new live scenario first computes and retains a healthy baseline. It then
holds the same job at a real result stage, cancels or disconnects it, verifies
released capacity while the marker still exists, removes only that marker,
and replays the same job. The entire returned result must equal the pre-fault
baseline, in addition to the analytical field checks.

| Installed fixture | Physical nodes | Elements | Cancellation / disconnect |
| --- | ---: | ---: | ---: |
| Heat quad, 40 by 40 grid | 1,681 | 1,600 | 7 / 2 |
| Heat triangle, same grid split into triangles | 1,681 | 3,200 | 7 / 2 |
| Thermal-stress quad, 12 by 12 grid | 169 | 144 | 7 / 2 |
| Thermal-stress triangle, same grid split into triangles | 169 | 288 | 7 / 2 |

Heat has 1,599 free temperature DOFs; thermal stress has 312 free displacement
DOFs. Construction/restoration/totals/norm holds occur at 64 steps. Heat maxima
hold at 1,024; thermal node maxima at the final 169 and thermal element maxima
at the final 144/288. Thus installed cases cover interior and terminal maxima,
not only a large-array interior. Thermal zero-length prescribed restoration is
covered natively, not claimed as an installed positive-step fault.

All 28 cancellation receipts contain code `cancelled`, the selected exact
stage/count, no result and `resumable=false`. Eight transport-loss cases occur
at element construction or totals; watchdog failures identify that exact stage
and 64 steps. Capacity is zero before marker removal. Heat replays check every
temperature at 1e-6 K and each flux component at 1e-3 W/m2 against the linear
field. Thermal replays check free uniform expansion at 1e-10 m and maximum
stress below 1e-4 Pa. These explicit synthetic tolerances are not relaxed to
make cancellation tests pass. Normal research has all fault controls unset.

The official Rust Headless SDK then submits fresh work through an isolated
Installer-managed distributed runtime with SQLite and transitional worker
adapters disabled. Baseline job `8196838cef863ab4` completes its seven-node
graph and matches the preceding study. This separate mixed fixture has 153
physical nodes and 128 Q4 cells. Temperature/displacement/flux errors are
6.310e-12 K, 6.806e-17 m and 7.687e-9 W/m2, under the unchanged 1e-7 K,
1e-9 m and 7.921e-5 W/m2 gates. All 14 layered and 288 triangle/quad variations
pass physics, output-manifest verification and readback.

Before idle restart, Agent `agent-instance-7-1788916697957-1` records 606
started/completed executions and zero failed/active executions. Both owned
services stop with exit 0 and no OOM. After restart, all 302 original results
pass exact readback and the independent baseline file is byte-identical.
New Agent `agent-instance-7-1788917105802-1` records zero started/completed/
failed/active executions throughout readback: no hidden recomputation is used.
The baseline verifier also checks 600 ms terminal stability. This is idle
persistence qualification, not an in-flight Orchestra process-loss or HA test.

## Performance and provenance

The paired release benchmark exercises actual result helpers: 100,000 thermal
node records with 200,000 displacement values, one-million-value maximum and
sum scans, and restoration of one million scalar DOFs split evenly between
prescribed/free lists. These are kernels, not million-node FEM solves. Three
warmups precede nine rotated old/unscoped/controlled samples with eight passes
each. Inputs are black-boxed and outputs checked; setup and output comparisons
are outside timing. Result allocation and replacement of previous pass results
are inside timing where applicable.

Final repeat, median milliseconds/pass:

| Kernel | Old loop | Unscoped | Controlled |
| --- | ---: | ---: | ---: |
| Thermal node records | 3.265748 | 3.325888 | 3.472270 |
| Maximum, 1M values | 0.242019 | 0.273303 | 0.268658 |
| Sum, 1M values | 0.678025 | 0.720636 | 0.763040 |
| Restore, 1M DOFs | 2.087081 | 2.258665 | 2.250311 |

The first generic iterator maximum regressed from 0.252083 to 0.904655 ms,
about 3.59 times its paired reference. Sliced 64-value scans reduced this to
0.258860/0.508700/0.504308 ms but were not accepted as the final implementation.
Separating cheap maxima into 1,024-record blocks gave a preceding final-code
repeat of 0.260410/0.271962/0.284600 ms while retaining all special-value and
terminal-boundary checks. Expensive record construction remains at 64.

**No global overhead bound** is claimed. Final maximum overhead is about 11.0%,
node construction 6.3%, sum 12.5% and restoration 7.8%. The shared host and code
generation affect these paired measurements. The retained matrix-product,
PCG-vector and sparse-scaling suites also pass; no end-to-end speedup is inferred.
Installed functional tests use a debug-profile Agent, not the benchmark binary.

Base revision is `82fb5cb18fe0e728b99596eb4b3b5b38ed7d7965` plus the 14-file
retained Rust source/test overlay. All overlay digests and all 1,251 tracked
native-workspace file digests match the remote tree. The five new untracked
helper/test files are covered by the overlay, giving 1,256 unique verified files.
SDK study definition remains
`245beec6daf0c4647ecaba7f4e5606a9fa17fa325a481e12e34773d7bffe218c`.

Installer seals the final candidate under label 3.1.6 in a fresh managed root;
actual Cargo, reused Orchestra and SDK metadata remain 3.0.0. An initial
orchestration-order mistake started installation before sealing completed:
the digest check rejected it and no installed candidate existed. The premature
live-test launch likewise failed before qualification. Those attempt artifacts
are excluded from pass counts; successful installation occurs after sealing,
and installed/source payload manifests compare equal. Integrity checks are
not disabled. No version bump, commit, GUI packaging or production promotion occurs.

Native compilation uses Rust 1.95.0 on Linux; SDK research uses the reused
Elixir 1.19.5 / OTP 28.5.0.2 release, not source-tree Mix qualification. Owned
SDK containers run uid 1001, read-only root/payload, isolated writable state,
init, no capabilities, no-new-privileges, four CPUs each and 4 GiB / 2 GiB
memory limits. HTTP/TCP bind loopback 6500/6501. Both containers are stopped
and removed with exit 0 and no OOM. Original services and unrelated containers
are not restarted or modified.

Evidence stays under managed server state `research-runs/postprocess-research-20260909/`:
old/new physical records, source overlays/digests, rejected/successful install
attempts, kernel measurements, installed checkpoint/receipt matrices, fresh
research arrays, SQLite state, readback and process exits. Server configuration,
credentials, payloads and bulk results are not added to Git.

| Artifact | SHA-256 |
| --- | --- |
| Installed Agent | `fe55fc571261e3892295917fde10696dbb9f3df4389e10a0e48cee73d257bae7` |
| Payload manifest | `fffa38254b6436176c6cae39b77a49fdc2d91bb8074c28ed8a542a15011bbe70` |
| Final source/test overlay | `4f0451fa17c3e6e79927b7fa968c1da8a265b270e761efda0e42da194b60d097` |
| Twelve old/final physical records | `90eecdfc8367528c133e06ed32a268103cbdf654e61ded4040ea37332c89d9ee` |
| Fresh 302-case report | `cb0b1616e87a0ea1efaef6e0c88982579f139c86d5160f1a648a9015b220846b` |

Digests identify bytes, not signatures. Reproduce with
`cargo test -p kyuubiki-solver --tests`,
`cargo test -p kyuubiki-solver --release --lib paired_ -- --ignored --nocapture --test-threads=1`
and the explicit `retained_postprocess_physical_reference` probe. Select the
installed Agent with `KYUUBIKI_TEST_AGENT_BINARY` and retain its owned evidence
with `KYUUBIKI_TEST_AGENT_EVIDENCE_DIR`. See the
[HTML tutorial](../docs/research-layered-thermal.html#interrupted-research) and
[control contract](../docs/agent-orchestrator-boundary.md#cooperative-numerical-cancellation).

Remaining gaps include retained-input cloning, serialization/output writes,
individual allocations and long string copies, other operators' postprocessing,
dense/preconditioner row interiors, other assembly paths, nonlinear outer loops,
analytical shortcuts, custom constraints, external workers and child threads.
Power loss, network blackholes, database corruption, external-solver validation
and real-material calibration remain separate qualifications. Scoped tensor
evidence does not override overall release maturity.

The 25-file HTML book check, documentation inventory, Rust formatting, tensor
structure/self-test and organization audit/self-test pass. Source/document limits
remain 800/2000 with zero tracked debt. The new scoped evidence is registered
through `runtime-postprocess.json`; the global tensor still reports
`daji status=blocked` and is not promoted by these local qualifications.
