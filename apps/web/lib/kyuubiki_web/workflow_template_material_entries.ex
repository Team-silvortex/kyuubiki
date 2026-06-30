defmodule KyuubikiWeb.WorkflowTemplateMaterialEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def list do
    [
      material_margin_entry(),
      material_rank_entry(),
      material_fatigue_entry(),
      material_score_entry(),
      material_experiment_plan_entry(),
      material_pareto_entry()
    ]
  end

  defp material_margin_entry do
    entry(
      "workflow.material-margin-summary-json",
      "Material margin summary JSON",
      "Evaluates a solver summary against visible material limits and exports failure-index and safety-factor metrics.",
      ["material", "optimization", "margin", "safety_factor"],
      material_margin_graph(),
      [%{"node_id" => "summary_input", "artifact_type" => "report/summary"}],
      [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    )
  end

  defp material_rank_entry do
    entry(
      "workflow.material-candidate-ranking-json",
      "Material candidate ranking JSON",
      "Ranks candidate material margin summaries and exports the best feasible material choice.",
      ["material", "optimization", "ranking", "candidate_selection"],
      material_rank_graph(),
      [%{"node_id" => "candidates_input", "artifact_type" => "report/summary_collection"}],
      [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    )
  end

  defp material_score_entry do
    entry(
      "workflow.material-candidate-score-json",
      "Material candidate weighted score JSON",
      "Scores material candidates with normalized weighted min/max criteria and exports explainable optimization rankings.",
      ["material", "optimization", "score", "weighted_objective", "candidate_selection"],
      material_score_graph(),
      [%{"node_id" => "candidates_input", "artifact_type" => "report/summary_collection"}],
      [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    )
  end

  defp material_fatigue_entry do
    entry(
      "workflow.material-fatigue-life-json",
      "Material fatigue life JSON",
      "Estimates fatigue life and fatigue safety factors for material candidates, then exports the assessment as JSON.",
      ["material", "optimization", "fatigue", "life_estimation", "safety_factor"],
      material_fatigue_graph(),
      [%{"node_id" => "candidates_input", "artifact_type" => "report/summary_collection"}],
      [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    )
  end

  defp material_pareto_entry do
    entry(
      "workflow.material-pareto-frontier-json",
      "Material Pareto frontier JSON",
      "Extracts a multi-objective Pareto frontier from material candidate summaries.",
      ["material", "optimization", "pareto", "multi_objective"],
      material_pareto_graph(),
      [%{"node_id" => "candidates_input", "artifact_type" => "report/summary_collection"}],
      [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    )
  end

  defp material_experiment_plan_entry do
    entry(
      "workflow.material-experiment-plan-json",
      "Material experiment plan JSON",
      "Scores material candidates, turns the ranking into a prioritized experiment shortlist, and exports the plan as JSON.",
      ["material", "optimization", "experiment_plan", "shortlist", "candidate_selection"],
      material_experiment_plan_graph(),
      [%{"node_id" => "candidates_input", "artifact_type" => "report/summary_collection"}],
      [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    )
  end

  defp material_margin_graph do
    graph(
      "workflow.material-margin-summary-json",
      "Material margin summary JSON",
      [
        input_node("summary_input", "summary", "report/summary", "solver_summary"),
        transform_node(
          "evaluate_margin",
          "transform.evaluate_material_margins",
          %{
            "limits" => %{
              "max_stress" => %{"limit" => 250_000_000.0},
              "max_temperature" => %{"limit" => 450.0}
            }
          },
          "summary",
          "report/summary",
          "solver_summary",
          "margin",
          "report/summary",
          "material_margin"
        ),
        export_node("material_margin"),
        output_node()
      ],
      [
        edge(
          "e0",
          "summary_input",
          "summary",
          "evaluate_margin",
          "summary",
          "report/summary",
          "solver_summary"
        ),
        edge(
          "e1",
          "evaluate_margin",
          "margin",
          "export_json",
          "summary",
          "report/summary",
          "material_margin"
        ),
        edge("e2", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ],
      "material_margin_summary"
    )
  end

  defp material_rank_graph do
    graph(
      "workflow.material-candidate-ranking-json",
      "Material candidate ranking JSON",
      [
        input_node(
          "candidates_input",
          "candidates",
          "report/summary_collection",
          "candidate_summaries"
        ),
        transform_node(
          "rank_candidates",
          "transform.rank_material_candidates",
          %{},
          "candidates",
          "report/summary_collection",
          "candidate_summaries",
          "ranking",
          "report/summary",
          "material_ranking"
        ),
        export_node("material_ranking"),
        output_node()
      ],
      [
        edge(
          "e0",
          "candidates_input",
          "candidates",
          "rank_candidates",
          "candidates",
          "report/summary_collection",
          "candidate_summaries"
        ),
        edge(
          "e1",
          "rank_candidates",
          "ranking",
          "export_json",
          "summary",
          "report/summary",
          "material_ranking"
        ),
        edge("e2", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ],
      "material_candidate_ranking"
    )
  end

  defp material_score_graph do
    graph(
      "workflow.material-candidate-score-json",
      "Material candidate weighted score JSON",
      [
        input_node(
          "candidates_input",
          "candidates",
          "report/summary_collection",
          "candidate_summaries"
        ),
        transform_node(
          "score_candidates",
          "transform.score_material_candidates",
          %{
            "criteria" => [
              %{"field" => "mass", "goal" => "min", "weight" => 0.35},
              %{"field" => "cost", "goal" => "min", "weight" => 0.15},
              %{"field" => "material_safety_factor", "goal" => "max", "weight" => 0.5}
            ]
          },
          "candidates",
          "report/summary_collection",
          "candidate_summaries",
          "score",
          "report/summary",
          "material_score"
        ),
        export_node("material_score"),
        output_node()
      ],
      [
        edge(
          "e0",
          "candidates_input",
          "candidates",
          "score_candidates",
          "candidates",
          "report/summary_collection",
          "candidate_summaries"
        ),
        edge(
          "e1",
          "score_candidates",
          "score",
          "export_json",
          "summary",
          "report/summary",
          "material_score"
        ),
        edge("e2", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ],
      "material_candidate_score"
    )
  end

  defp material_experiment_plan_graph do
    graph(
      "workflow.material-experiment-plan-json",
      "Material experiment plan JSON",
      [
        input_node(
          "candidates_input",
          "candidates",
          "report/summary_collection",
          "candidate_summaries"
        ),
        transform_node(
          "score_candidates",
          "transform.score_material_candidates",
          material_score_config(),
          "candidates",
          "report/summary_collection",
          "candidate_summaries",
          "score",
          "report/summary",
          "material_score"
        ),
        transform_node(
          "plan_experiments",
          "transform.plan_material_experiments",
          %{"top_k" => 2, "label" => "material-screening"},
          "score",
          "report/summary",
          "material_score",
          "plan",
          "report/summary",
          "material_experiment_plan"
        ),
        export_node("material_experiment_plan"),
        output_node()
      ],
      [
        edge(
          "e0",
          "candidates_input",
          "candidates",
          "score_candidates",
          "candidates",
          "report/summary_collection",
          "candidate_summaries"
        ),
        edge(
          "e1",
          "score_candidates",
          "score",
          "plan_experiments",
          "score",
          "report/summary",
          "material_score"
        ),
        edge(
          "e2",
          "plan_experiments",
          "plan",
          "export_json",
          "summary",
          "report/summary",
          "material_experiment_plan"
        ),
        edge("e3", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ],
      "material_experiment_plan"
    )
  end

  defp material_fatigue_graph do
    graph(
      "workflow.material-fatigue-life-json",
      "Material fatigue life JSON",
      [
        input_node(
          "candidates_input",
          "candidates",
          "report/summary_collection",
          "candidate_summaries"
        ),
        transform_node(
          "estimate_fatigue",
          "transform.estimate_material_fatigue_life",
          %{
            "fatigue_strength" => 120.0,
            "reference_cycles" => 1.0e6,
            "slope_exponent" => 4.0,
            "target_cycles" => 8.0e5
          },
          "candidates",
          "report/summary_collection",
          "candidate_summaries",
          "fatigue",
          "report/summary",
          "material_fatigue"
        ),
        export_node("material_fatigue"),
        output_node()
      ],
      [
        edge(
          "e0",
          "candidates_input",
          "candidates",
          "estimate_fatigue",
          "candidates",
          "report/summary_collection",
          "candidate_summaries"
        ),
        edge(
          "e1",
          "estimate_fatigue",
          "fatigue",
          "export_json",
          "summary",
          "report/summary",
          "material_fatigue"
        ),
        edge("e2", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ],
      "material_fatigue_life"
    )
  end

  defp material_pareto_graph do
    graph(
      "workflow.material-pareto-frontier-json",
      "Material Pareto frontier JSON",
      [
        input_node(
          "candidates_input",
          "candidates",
          "report/summary_collection",
          "candidate_summaries"
        ),
        transform_node(
          "extract_pareto",
          "transform.extract_material_pareto_frontier",
          %{
            "objectives" => [
              %{"field" => "mass", "goal" => "min"},
              %{"field" => "material_safety_factor", "goal" => "max"}
            ]
          },
          "candidates",
          "report/summary_collection",
          "candidate_summaries",
          "frontier",
          "report/summary",
          "material_pareto"
        ),
        export_node("material_pareto"),
        output_node()
      ],
      [
        edge(
          "e0",
          "candidates_input",
          "candidates",
          "extract_pareto",
          "candidates",
          "report/summary_collection",
          "candidate_summaries"
        ),
        edge(
          "e1",
          "extract_pareto",
          "frontier",
          "export_json",
          "summary",
          "report/summary",
          "material_pareto"
        ),
        edge("e2", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ],
      "material_pareto_frontier"
    )
  end

  defp graph(id, name, nodes, edges, family) do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.#{family}/v1",
          [
            value("candidate_summaries", "result", "report/summary_collection"),
            value("solver_summary", "result", "report/summary"),
            value("material_margin", "result", "report/summary"),
            value("material_ranking", "result", "report/summary"),
            value("material_score", "result", "report/summary"),
            value("material_experiment_plan", "result", "report/summary"),
            value("material_fatigue", "result", "report/summary"),
            value("material_pareto", "result", "report/summary"),
            value("summary_json", "export", "export/json", "utf8_text")
          ],
          %{"workflow_family" => family}
        ),
      "entry_nodes" => [List.first(nodes)["id"]],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => nodes,
      "edges" => edges
    }
  end

  defp input_node(id, port_id, artifact_type, dataset_value),
    do: %{
      "id" => id,
      "kind" => "input",
      "outputs" => [port(port_id, artifact_type, dataset_value)]
    }

  defp transform_node(
         id,
         operator_id,
         config,
         input_id,
         input_type,
         input_value,
         output_id,
         output_type,
         output_value
       ),
       do: %{
         "id" => id,
         "kind" => "transform",
         "operator_id" => operator_id,
         "config" => config,
         "inputs" => [port(input_id, input_type, input_value)],
         "outputs" => [port(output_id, output_type, output_value)]
       }

  defp material_score_config,
    do: %{
      "criteria" => [
        %{"field" => "mass", "goal" => "min", "weight" => 0.35},
        %{"field" => "cost", "goal" => "min", "weight" => 0.15},
        %{"field" => "material_safety_factor", "goal" => "max", "weight" => 0.5}
      ]
    }

  defp export_node(dataset_value),
    do: %{
      "id" => "export_json",
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [port("summary", "report/summary", dataset_value)],
      "outputs" => [port("json", "export/json", "summary_json")]
    }

  defp output_node,
    do: %{
      "id" => "json_output",
      "kind" => "output",
      "inputs" => [port("json", "export/json", "summary_json")],
      "outputs" => []
    }

  defp entry(id, name, summary, tags, graph, inputs, outputs),
    do: %{
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "summary" => summary,
      "domains" => ["material"],
      "capability_tags" => tags ++ ["headless_safe"],
      "graph" => graph,
      "entry_inputs" => inputs,
      "output_artifacts" => outputs
    }

  defp edge(id, from_node, from_port, to_node, to_port, artifact_type, dataset_value),
    do: %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type,
      "dataset_value" => dataset_value
    }

  defp port(id, artifact_type, dataset_value),
    do: %{"id" => id, "artifact_type" => artifact_type, "dataset_value" => dataset_value}

  defp value(id, data_class, semantic_type, element_type \\ "json_object"),
    do:
      WorkflowCatalogSupport.workflow_dataset_value_info(
        id,
        data_class,
        semantic_type,
        element_type
      )
end
