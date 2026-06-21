# Solver Matrix Optimization Pack

This note records the matrix-side optimization set that is currently worth
keeping in the Rust solver, how it was validated, and what was explicitly
rejected during `tamamono 1.9.x` thermal benchmark work.

It exists to stop future contributors from repeating the same exploratory loop
without context.

## Scope

This note is about the Rust solver path under:

- `workers/rust/crates/solver`
- `workers/rust/crates/benchmark`

The anchor workload used during this round was:

- `heat-plane-quad-10k`
- profile: `10k`
- matrix: `thermal` locally, `thermal-core` for checked baseline and physical-lab comparison

## Keep These Changes

The following changes have positive evidence and should remain unless a better
replacement is benchmarked:

1. `precompute_heat_plane_triangle_element_from_nodes`
   avoids rebuilding triangle requests and cloning the full node list during
   heat-plane quad precompute.
2. `profile_heat_plane_quad_2d`
   keeps stage-level RSS visibility for the thermal quad path:
   `precompute -> assemble_global -> reduce_system -> solve_system -> assemble`.
3. `SparseMatrix::with_uniform_row_capacity`
   gives derived sparse matrices an explicit row-capacity hint instead of
   forcing repeated allocator growth.
4. `SparseMatrix::push_sorted_entry`
   lets derived matrices that are already emitted in non-decreasing column order
   append directly instead of paying `binary_search + insert` cost.
5. `reduce_sparse_system`
   and `reduce_sparse_system_with_prescribed`
   now use the sorted append path for reduced matrices.
6. `scale_sparse_matrix`
   now uses the same sorted append path and row-capacity hint.
7. `solve_spd_system`
   no longer keeps one extra long-lived scaled sparse matrix alive on the
   success path after compression.

## Rejected Changes

The following ideas were tested and should not be reintroduced casually:

1. Replacing the heat-plane quad global assembly with an unordered
   `append_unordered_entry + finalize_rows` path.
   It was structurally interesting, but the measured result did not beat the
   stronger retained variant.
2. Re-reading compressed-matrix diagonals inside every CG iteration instead of
   keeping a local diagonal vector.
   That version regressed local benchmark behavior.
3. Treating stage-end RSS as a proxy for short-lived peak RSS.
   Stage-end RSS is still useful, but it does not explain the full release-mode
   `peak_rss` spike by itself.

## Current Evidence

### Local A/B

One clean detached-HEAD worktree was used as a control group.

- current optimized worktree:
  `cargo run -p kyuubiki-benchmark -- --profile 10k --matrix thermal --case heat-plane-quad --repeat 3`
  median `657.5775 ms`, peak RSS `15 MiB`
- clean `HEAD` worktree:
  same command, median `6797.5630 ms`, peak RSS `22 MiB`

This does not prove every individual change is independently good.
It does prove that the retained optimization pack is materially better than the
clean baseline for the thermal quad path.

### Physical Lab Reference

On `kyuubiki-lab`, the validated release-mode comparison for:

- `cargo run --release -q -p kyuubiki-benchmark -- --profile 10k --matrix thermal-core --repeat 3 --baseline-compare benchmarks/thermal-core-10k-baseline.json`

showed:

- median `30.8742 ms`
- peak RSS `29 MiB`
- stage-end RSS stable at `15 MiB`
- relative to checked baseline:
  median `-98.82%`
  peak RSS `+66.25%`

Interpretation:

- the retained pack clearly improved runtime cost
- the release-mode `peak_rss` spike is still not fully solved
- the remaining spike likely comes from a short-lived internal allocation rather
  than a stage-end steady-state footprint

## How To Evaluate Future Matrix Changes

Use this order:

1. `cargo test -p kyuubiki-solver`
2. local thermal benchmark:
   `cargo run -p kyuubiki-benchmark -- --profile 10k --matrix thermal --case heat-plane-quad --repeat 3`
3. if the local result is credible, run the physical-lab release benchmark:
   `cargo run --release -q -p kyuubiki-benchmark -- --profile 10k --matrix thermal-core --repeat 3 --baseline-compare benchmarks/thermal-core-10k-baseline.json`
4. compare against this note before keeping the new change

Do not keep a matrix optimization only because it feels more elegant.
Keep it only if it improves one of:

- median runtime
- peak RSS
- stage-end RSS clarity
- implementation simplicity without benchmark regression

## Next Likely Targets

If later work continues this line, the most promising next targets are:

1. short-lived allocation spikes inside compressed-matrix solve internals
2. compressed representation construction costs beyond the current row-capacity
   and sorted-append improvements
3. extending the same retained derived-matrix strategy to other solver families
   only after family-specific benchmarks exist
