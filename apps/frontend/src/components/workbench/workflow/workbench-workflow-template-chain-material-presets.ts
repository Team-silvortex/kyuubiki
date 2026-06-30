"use client";

import type { WorkflowTemplateChainDefinition } from "@/components/workbench/workflow/workbench-workflow-template-chain-library";

export const MATERIAL_DECISION_TEMPLATE_CHAINS: WorkflowTemplateChainDefinition[] = [
  {
    id: "material_candidate_rank_pareto_decision",
    label: "material candidates -> rank + Pareto",
    source: "built-in",
    summary:
      "Rank pre-evaluated material candidate summaries and extract a multi-objective Pareto frontier for the decision tail of coupled physics workflows.",
    tags: [
      "material",
      "margin",
      "ranking",
      "pareto",
      "multi_objective",
      "optimization",
      "decision",
      "headless",
    ],
    templates: [
      {
        kind: "input",
      },
      {
        kind: "transform",
        operatorId: "transform.rank_material_candidates",
        config: { include_best_summary: true },
      },
      {
        kind: "transform",
        operatorId: "transform.extract_material_pareto_frontier",
        config: {
          objectives: [
            { field: "material_failure_index", goal: "min", weight: 4.0 },
            { field: "mass", goal: "min", weight: 1.0 },
            { field: "material_safety_factor", goal: "max", weight: 2.0 },
          ],
        },
      },
      { kind: "export", operatorId: "export.summary_json" },
    ],
    connections: [
      { from: 0, to: 1 },
      { from: 0, to: 2 },
      { from: 2, to: 3 },
    ],
  },
];
