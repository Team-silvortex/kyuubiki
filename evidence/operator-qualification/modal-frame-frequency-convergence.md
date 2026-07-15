# Modal Frame Frequency Convergence Evidence

This note describes the retained modal-frame sanity regression used by the
`modal-frame-sanity` qualification candidate.

## Scope

The retained regression checks compact 2D and 3D cantilever frame fixtures. It
does not claim industrial modal convergence. Instead, it verifies that the
current solver preserves the expected direction of frequency change when the
cantilever length changes and that mode ordering and shape normalization stay
within the documented review contract.

## Evidence

The executable evidence is:

- `workers/rust/crates/solver/tests/modal_frame_sanity_regression.rs`
- `workers/rust/crates/solver/tests/modal_frame_2d_review.rs`
- `workers/rust/crates/solver/tests/modal_frame_3d_review.rs`

The regression compares short and long cantilevers with the same material and
section properties. Shorter cantilevers must produce higher retained natural
frequencies than longer cantilevers for the checked modes.

## Boundary

The evidence is intentionally conservative. It covers frequency positivity,
ordering, normalization, restrained DOF zeroing, and length-scaling direction.
It does not cover damping, forced response, experimental correlation, shell or
solid modal models, or mesh refinement across many elements.
