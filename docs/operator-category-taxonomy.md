# Operator category taxonomy

Kyuubiki operators keep their precise `kind`, `domain`, `family`, and
`capability_tags`, but the product surface also needs a stable large-category
layer for search, store grouping, workflow building, and agent dispatch review.

Every operator catalog descriptor now exposes:

- `operator_category_id`: stable machine-facing category id.
- `operator_category`: display metadata with `id`, `label`, `lane`, and
  `summary`.

## Current categories

| Category id | Label | Lane | Use |
| --- | --- | --- | --- |
| `physics_solver` | Physics Solvers | `physics` | FEM/physics solve operators that turn study models into results. |
| `multiphysics_bridge` | Multiphysics Bridges | `coupling` | Operators that map upstream results into downstream models. |
| `material_research` | Material Research | `research` | Material cards, validation, and material-research primitives. |
| `postprocess_extract` | Postprocess Extractors | `inspection` | Result summary, field statistics, peak, and diagnostic extraction. |
| `diagnostics_guard` | Diagnostics and Guards | `safety` | Preflight, diagnostic composition, threshold guard, and blocking checks. |
| `optimization_selection` | Optimization and Selection | `optimization` | Ranking, scoring, Pareto, benchmarking, and candidate selection. |
| `automation_loop` | Automation Loop | `automation` | Iteration decision, next-round request, orchestration handoff, and snapshots. |
| `data_transform` | Data Transforms | `dataflow` | General payload merge, normalize, select, compose, and focus transforms. |
| `delivery_export` | Delivery and Export | `delivery` | Export operators that produce user-facing artifacts. |

## API usage

The operator catalog can be filtered by category:

```text
GET /api/v1/operators?category=material_research
```

The response includes a filtered `operator_categories` summary alongside
`operators` and `modules`, so UI panels can render category counts without
recomputing them client-side.

## Classification rules

The taxonomy is intentionally derived from existing descriptor fields rather
than duplicated per operator:

- `kind = solver` maps to `physics_solver`.
- `kind = workflow_bridge` maps to `multiphysics_bridge`.
- `kind = extract` maps to `postprocess_extract`.
- `kind = export` maps to `delivery_export`.
- Material operators map to `material_research`, unless their tags make them
  more specifically `optimization_selection` or `automation_loop`.
- Guard, diagnostic, preflight, and threshold tags map to `diagnostics_guard`.
- Ranking, score, benchmark, Pareto, and candidate-selection tags map to
  `optimization_selection`.
- Iteration, next-round, orchestration, snapshot, and state tags map to
  `automation_loop`.
- Remaining transform operators map to `data_transform`.

This keeps operator authors focused on accurate `kind`, `family`, and tags,
while the catalog provides the larger product-facing grouping automatically.
