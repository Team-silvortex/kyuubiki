# Modal Canonical Grid And Candidate Cost

Date: 2026-10-04
Source line: daji 3.4.5 working-tree overlay based on `431b2ac7`.
Platform: local macOS ARM64, Rust 1.88; optimized native libtest.

## Scope And Decision

This is test-only canonical grid validation and bounded local candidate benchmarking, not runtime admission or general modal qualification.

The earlier [portfolio report](modal-grid-portfolio-reliability-20261004.md)
retains eighteen successful beam transformations and aggregate failure controls.
This follow-on expands the retained numbering boundary, including actual failures,
and measures the alternative candidate entries. All new candidate code is behind
`cfg(test)`. Public Solver, Engine, Agent, Headless, formats, dependencies,
tolerances and installed applications are unchanged.

## Expanded Raw Boundary

Three uniform straight-beam discretizations (80, 100, 128 elements), three segment
lengths (`1`, `1e14`, `1e-10`), four fixed Fisher-Yates permutations (seeds
7, 19, 41, 113) and both signs give 72 raw-portfolio cases. The matrix rows and
columns, masses and seed follow a joint permutation of the same assembled input.
These are transformations of nine fixtures, not 72 independent geometries.

| Elements | Cases | Accepted | Rejected | Interpretation |
| --- | --- | --- | --- | --- |
| 80 | 24 | 24 | 0 | All original seeds already pass; retention, not recovery |
| 100 | 24 | 24 | 0 | All original seeds already pass; retention, not recovery |
| 128 | 24 | 8 | 16 | All original seeds require correction |
| Total | 72 | 56 | 16 | Raw three-order portfolio is not numbering stable |

At 128 elements, unit scale accepts only shuffle 113; large scale accepts only
shuffle 7; tiny scale accepts shuffles 7 and 41. Both signs have identical
classifications. Accepted shuffled candidates use the third (grid-norm) order
and ten actual certificates. All sixteen failures exhaust the three policies
after ten certificates, with no operator error misclassified as rejection.
Per-case expected outcomes are asserted, not just aggregate pass counts.

## Alternative Canonical Entry

The new entry orders coordinates by the binary exponent of
`abs(direction[i][i]) / sqrt(mass[i])`, then exact absolute seed bits.
It has no caller-supplied translation/rotation partition. Equal signatures are
explicitly unsupported and fail before any physical certificate; no original
index tie-break is used to pretend general numbering invariance.

Rows, columns, seed and mass are copied into this order. Candidate shapes are
restored to the caller's original coordinates for every actual operator check,
and the true residual is remapped back. Final restoration checks unit norm
again because floating-point reduction order changes, retains the original
anchor bits, and has a final cancellation checkpoint. There is no post-check
renormalization or eigenvalue replacement.

This entry is an alternative to the raw portfolio, not an extra fallback after
its three failures. It shares the same three sequential-factor limit, nineteen
actual-certificate cap, four correction passes per fit and radius 4,194,304.
A separate plan accounts for the canonical copy, extra strict preflight and
bounded callback/result mappings. At 256 coordinates the conservative payload
model is 6,914,048 bytes and proposal-work model is 1,009,090,560 visits, still
under 8 MiB and 1,050,000,000 visits. These models exclude caller physical
assembly, actual certificate work and diagnostics; they are not measured heap
or RSS. Reservations are charged to the returned monotonic usage counter.

The canonical test includes identity plus the four shuffles, three element
counts, three scales and both signs: 90 cases. All pass independent physical
reassembly and constructed result JSON readback. After sign restoration, shape
bits match the identity candidate exactly across all five numberings.

| Elements | Cases | Factors/certificates per case | Residual at lengths 1 / 1e14 / 1e-10 |
| --- | --- | --- | --- |
| 80 | 30 | 1 / 3 | 2.864669e-9 / 3.191792e-9 / 3.555850e-9 |
| 100 | 30 | 1 / 3 | 6.888546e-9 / 8.181898e-9 / 7.745823e-9 |
| 128 | 30 | 1 / 4 | 2.845895e-9 / 3.405395e-9 / 3.272357e-9 |

For 80/100 elements these are already-passing seed retention checks.
For 128 elements all thirty cases need correction. The result is scoped to
joint permutations of fixed assembled inputs with unique signatures. It is
not proof that independently reassembled/recomputed seeds have identical bits,
nor arbitrary geometry, repeated signatures or multimode invariance.

Controls additionally cover invalid plan dimensions, signature ambiguity,
operator faults, malformed original receipts, final certificate failure,
cancellation during original-residual mapping and before returning the restored
shape, and fresh replay. Errors return no candidate and never reset the budget.
The original raw-portfolio controls still test eighteen rejection certificates
and the successful nineteen-certificate boundary using injected receipts;
those controls do not constitute physical certificates.

## Isolated Optimized Cost Comparison

Each mode ran in its own process, sequentially after other builds/tests finished.
Each of the nine cases uses one warmup and five timed samples. Fixture/eigenvalue/
direction construction precedes the timer; independent physical/JSON reassembly
follows it. Timers include entry-specific anchor/order/signature preparation,
factorization, correction and actual operator calls, but exclude final output
conversion back to the physical fixture in the benchmark harness. The canonical
entry's own mandatory restoration is inside its timer.

The single-order baseline performs one reverse-order fit and its built-in
certificate, without the portfolio's extra publication certificate or order
preflight. It is a lower-bound entry comparison, not identical publication
semantics. Every accepted timed or warmup candidate still receives an independent
physical/JSON recheck outside the timer. Rejected runs are timed too.

Each cell below is median milliseconds / outcome / actual certificate count.

| Segment length | Numbering | Single reverse fit | Raw portfolio | Canonical portfolio |
| --- | --- | --- | --- | --- |
| 1 | identity | 12.986 / pass / 3 | 13.193 / pass / 4 | 13.684 / pass / 4 |
| 1 | reversed | 12.991 / reject / 3 | 26.390 / pass / 7 | 13.773 / pass / 4 |
| 1 | shuffle-7 | 13.031 / reject / 3 | 40.151 / reject / 10 | 13.570 / pass / 4 |
| 1e14 | identity | 13.071 / pass / 3 | 13.313 / pass / 4 | 13.673 / pass / 4 |
| 1e14 | reversed | 12.983 / reject / 3 | 26.339 / pass / 7 | 13.536 / pass / 4 |
| 1e14 | shuffle-7 | 12.941 / reject / 3 | 39.937 / pass / 10 | 13.487 / pass / 4 |
| 1e-10 | identity | 13.057 / pass / 3 | 13.445 / pass / 4 | 13.614 / pass / 4 |
| 1e-10 | reversed | 13.017 / reject / 3 | 26.572 / pass / 7 | 13.626 / pass / 4 |
| 1e-10 | shuffle-7 | 13.043 / reject / 3 | 44.686 / pass / 10 | 13.708 / pass / 4 |

All three isolated processes pass their benchmark test. Canonical median cost
is 13.487-13.773 ms across these cases, and all nine classifications pass.
Raw reverse numbering requires two factors; shuffle 7 requires three and still
rejects at unit scale. These measurements are not production throughput, an
end-to-end solve, hard latency limits, statistical significance or scale testing.

Timing spread and native `getrusage` process high-water records follow.
RSS includes the caller fixture, independent rechecks, runtime and allocator
state. Each before/after value is a cumulative process peak, not per-case peak
allocation or incremental candidate heap. No before/after subtraction is used
as a solver memory metric.

| Mode | Length | Numbering | Min ms | Median ms | Max ms | Peak before KiB | Peak after KiB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| single | 1 | identity | 12.956 | 12.986 | 13.060 | 9024 | 11120 |
| single | 1 | reversed | 12.862 | 12.991 | 13.187 | 11120 | 11184 |
| single | 1 | shuffle-7 | 12.798 | 13.031 | 13.177 | 11184 | 11216 |
| single | 1e14 | identity | 12.733 | 13.071 | 13.101 | 11312 | 11376 |
| single | 1e14 | reversed | 12.835 | 12.983 | 13.182 | 11376 | 11376 |
| single | 1e14 | shuffle-7 | 12.871 | 12.941 | 13.174 | 11376 | 11376 |
| single | 1e-10 | identity | 12.906 | 13.057 | 13.179 | 11456 | 11456 |
| single | 1e-10 | reversed | 12.897 | 13.017 | 13.153 | 11456 | 11456 |
| single | 1e-10 | shuffle-7 | 13.002 | 13.043 | 13.104 | 11456 | 11456 |
| portfolio | 1 | identity | 12.916 | 13.193 | 13.514 | 9280 | 10880 |
| portfolio | 1 | reversed | 26.213 | 26.390 | 26.941 | 10880 | 10976 |
| portfolio | 1 | shuffle-7 | 39.628 | 40.151 | 40.333 | 10976 | 10976 |
| portfolio | 1e14 | identity | 13.241 | 13.313 | 13.416 | 11072 | 11120 |
| portfolio | 1e14 | reversed | 26.035 | 26.339 | 26.499 | 11120 | 11152 |
| portfolio | 1e14 | shuffle-7 | 39.409 | 39.937 | 69.187 | 11152 | 11152 |
| portfolio | 1e-10 | identity | 13.368 | 13.445 | 13.550 | 11216 | 11216 |
| portfolio | 1e-10 | reversed | 26.416 | 26.572 | 26.681 | 11216 | 11216 |
| portfolio | 1e-10 | shuffle-7 | 40.081 | 44.686 | 154.711 | 11216 | 11280 |
| canonical | 1 | identity | 13.449 | 13.684 | 14.408 | 8736 | 11936 |
| canonical | 1 | reversed | 13.550 | 13.773 | 13.882 | 11936 | 12000 |
| canonical | 1 | shuffle-7 | 13.222 | 13.570 | 13.757 | 12000 | 12016 |
| canonical | 1e14 | identity | 13.473 | 13.673 | 13.878 | 12128 | 12192 |
| canonical | 1e14 | reversed | 13.369 | 13.536 | 13.826 | 12192 | 12192 |
| canonical | 1e14 | shuffle-7 | 13.242 | 13.487 | 13.623 | 12192 | 12192 |
| canonical | 1e-10 | identity | 13.389 | 13.614 | 13.809 | 12256 | 12256 |
| canonical | 1e-10 | reversed | 13.455 | 13.626 | 13.697 | 12256 | 12256 |
| canonical | 1e-10 | shuffle-7 | 13.426 | 13.708 | 13.791 | 12256 | 12256 |

Process peak across the complete nine-case run is 11,456 KiB (single),
11,280 KiB (raw portfolio) and 12,256 KiB (canonical). Different process
baselines and allocator high water prevent interpreting those differences as
exact canonical heap overhead. The raw tiny shuffle includes a 154.711 ms
outlier; five samples do not establish a reliable tail-latency distribution.
External `time -l` and CPU-brand queries could not read sandboxed system
metadata; final retained runs use direct native libtest and native RSS only.

## Verification And Replay

| Check | Result |
| --- | --- |
| Final debug triangular-grid tests | 23 passed, 0 failed, 1 cost benchmark ignored |
| Final optimized triangular-grid tests | 23 passed, 0 failed, 1 cost benchmark ignored |
| Explicit optimized cost tests | 1 per mode passed; 3 isolated processes |
| Full Solver regression | 1,468 passed, 0 failed, 10 ignored; 186 result groups |
| Rust Headless modal integration | 37 passed, 0 failed, 0 ignored |
| Solver/CLI strict all-target Clippy | Passed with `-D warnings` |
| Formatting, whitespace and changed-file line limits | Passed; source <=800, docs <=2,000 |
| Tensor/profile validators and self-tests | Passed; 59 profiles, registry-only `executed=false` |
| Documentation book/inventory | Passed; 26 HTML files, development/shipping 3.4.5 |
| Project organization | Passed; tracked debt 0 |

All regression runs above use the final candidate source. The ten full-suite
ignored tests comprise nine original ignores and the new explicit cost test.
Focused tests, the full regression and benchmark samples overlap and must not
be added as independent physical coverage. The tensor retains 13 modules,
11 paradigms, no structural gaps, 4 maturity gaps, 16 evidence-grade gaps and
11 P0 gaps; qualification is still blocked. Native registry checks do not
execute numerical or release qualification. No version bump, Git submission,
new build directory or installed runtime change is performed.

The ignored cost test is intentional: ordinary regression does not claim that
performance was measured. Optimized cost replay, one invocation per mode:

```text
cd workers/rust
KYUUBIKI_MODAL_GRID_BENCH_MODE=single cargo test -p kyuubiki-solver --lib --release triangular_grid_portfolio_cost_benchmark --locked --offline -- --ignored --test-threads=1 --nocapture
KYUUBIKI_MODAL_GRID_BENCH_MODE=portfolio cargo test -p kyuubiki-solver --lib --release triangular_grid_portfolio_cost_benchmark --locked --offline -- --ignored --test-threads=1 --nocapture
KYUUBIKI_MODAL_GRID_BENCH_MODE=canonical cargo test -p kyuubiki-solver --lib --release triangular_grid_portfolio_cost_benchmark --locked --offline -- --ignored --test-threads=1 --nocapture
```

The retained run used the executable returned by Cargo's optimized no-run
artifact message, avoiding compilation time in each process. No remote,
installed application, live Agent, million-node, public tiny-case success or
release qualification is claimed. Nonuniform material/geometry, signature ties,
independently rebuilt seeds, clustered-mode membership and orthogonality, plus a
separately reviewed production admission contract remain open.

Follow-on [heterogeneous boundary validation](modal-grid-heterogeneous-reliability-20261004.md)
adds material/section gradients, layers and unequal element lengths, retaining
upstream spectrum failures separately from downstream candidate rejections.
Its additional scoped successes do not resolve generic production admission.
