defmodule KyuubikiWeb.WorkflowSummaryOperatorSpecs do
  @moduledoc false

  def transform_specs do
    [
      {"transform.first_available", "first_available",
       "Merge two branch payloads by forwarding the first available incoming artifact.",
       ["transform", "merge", "branch", "headless_safe"]},
      {"transform.merge_summary_pair", "merge_summary_pair",
       "Merge two summary payloads into one namespaced summary artifact.",
       ["transform", "summary", "merge", "headless_safe"]},
      {"transform.compare_summary_pair", "compare_summary_pair",
       "Compare two summary payloads and emit delta, ratio, and percent-change metrics for shared numeric fields.",
       ["transform", "summary", "compare", "benchmark", "headless_safe"]},
      {"transform.aggregate_summary_collection", "aggregate_summary_collection",
       "Aggregate multiple summary payloads into min/max/mean/span benchmark metrics for shared numeric fields.",
       ["transform", "summary", "aggregate", "benchmark", "headless_safe"]},
      {"transform.normalize_summary_fields", "normalize_summary_fields",
       "Rename and normalize summary fields with scale, offset, and clamp rules before downstream comparison or aggregation.",
       ["transform", "summary", "normalize", "mapping", "headless_safe"]},
      {"transform.select_best_summary", "select_best_summary",
       "Score multiple summary payloads against weighted min/max criteria and emit the best candidate summary with selection metadata.",
       ["transform", "summary", "select", "ranking", "benchmark", "headless_safe"]},
      {"transform.compose_quality_objective", "compose_quality_objective",
       "Combine domain quality scores into one weighted optimization objective for multiphysics studies.",
       ["transform", "summary", "objective", "optimization", "multiphysics", "headless_safe"]},
      {"transform.rank_quality_candidates", "rank_quality_candidates",
       "Rank candidate designs by composite domain quality objectives and emit the best ready candidate.",
       [
         "transform",
         "summary",
         "ranking",
         "objective",
         "optimization",
         "multiphysics",
         "headless_safe"
       ]},
      {"transform.prepare_quality_next_round_request", "prepare_quality_next_round_request",
       "Convert quality candidate rankings into a stop, continue, or replan request for the next exploration round.",
       [
         "transform",
         "summary",
         "next_round",
         "objective",
         "automation",
         "optimization",
         "multiphysics",
         "headless_safe"
       ]},
      {"transform.build_quality_parameter_sweep_plan", "build_quality_parameter_sweep_plan",
       "Convert a quality next-round request into a standard parameter sweep plan for continued exploration.",
       [
         "transform",
         "summary",
         "parameter_sweep",
         "next_round",
         "automation",
         "optimization",
         "multiphysics",
         "headless_safe"
       ]},
      {"transform.materialize_quality_sweep_expansion", "materialize_quality_sweep_expansion",
       "Materialize a quality parameter sweep plan into expand_parameter_sweep payload and config.",
       [
         "transform",
         "summary",
         "parameter_sweep",
         "materialize",
         "automation",
         "optimization",
         "headless_safe"
       ]},
      {"transform.compose_quality_execution_batch", "compose_quality_execution_batch",
       "Convert expanded quality sweep cases into language-neutral operator TaskIR batches for orchestra or agent execution.",
       [
         "transform",
         "summary",
         "parameter_sweep",
         "task_ir",
         "orchestration",
         "automation",
         "optimization",
         "headless_safe"
       ]},
      {"transform.expand_parameter_sweep", "parameter_sweep",
       "Expand a base model into bounded parameterized workflow cases for batch solve, material exploration, and optimization flows.",
       [
         "transform",
         "parameter_sweep",
         "batch",
         "material_exploration",
         "optimization",
         "headless_safe"
       ]},
      {"transform.join_parameter_sweep_results", "parameter_sweep_join_results",
       "Join distributed or headless case summaries back onto parameter sweep cases before summarizing and scoring.",
       [
         "transform",
         "parameter_sweep",
         "distributed_results",
         "material_exploration",
         "headless_safe"
       ]},
      {"transform.summarize_parameter_sweep", "parameter_sweep_summary",
       "Collect parameter sweep case summaries into a stable row table with numeric column statistics for research and optimization workflows.",
       [
         "transform",
         "parameter_sweep",
         "summary_table",
         "material_exploration",
         "headless_safe"
       ]},
      {"transform.score_parameter_sweep", "parameter_sweep_score",
       "Score parameter sweep rows with weighted objectives and feasibility limits so automated studies can pick a best candidate.",
       [
         "transform",
         "parameter_sweep",
         "objective",
         "optimization",
         "headless_safe"
       ]}
    ]
  end
end
