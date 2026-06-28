defmodule KyuubikiWeb.WorkflowTemplateCfdEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def list do
    [
      stokes_flow_quad_summary_entry(),
      stokes_flow_quad_guard_entry(),
      stokes_flow_quad_benchmark_entry()
    ]
  end

  defp stokes_flow_quad_summary_entry do
    custom_entry(
      "workflow.stokes-flow-quad-summary-json",
      "Stokes flow quad summary JSON",
      "Solves a low-Reynolds 2D Stokes flow quad model, extracts CFD diagnostics, and exports a JSON summary artifact.",
      ["fluid"],
      ["fluid", "cfd", "stokes", "flow", "diagnostics", "quad", "2d", "json"],
      stokes_flow_quad_summary_graph(),
      [
        %{
          "node_id" => "stokes_flow_model",
          "artifact_type" => "study_model/stokes_flow_quad_2d",
          "description" =>
            "Steady Stokes flow quad model used as the CFD workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded CFD diagnostics for the Stokes flow solve."
        }
      ]
    )
  end

  defp stokes_flow_quad_summary_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.stokes-flow-quad-summary-json",
      "name" => "Stokes flow quad summary JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.stokes_flow_quad_summary/v1",
          [
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "stokes_flow_model",
              "model",
              "study_model/stokes_flow_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "stokes_flow_result",
              "result",
              "result/stokes_flow_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "cfd_diagnostics",
              "result",
              "report/summary"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "summary_json",
              "export",
              "export/json",
              "utf8_text"
            )
          ],
          %{"workflow_family" => "stokes_flow_quad_summary"}
        ),
      "entry_nodes" => ["stokes_flow_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("stokes_flow_model", "study_model/stokes_flow_quad_2d", "stokes_flow_model"),
        solve_node(),
        diagnostics_node(),
        export_node("cfd_diagnostics"),
        output_node()
      ],
      "edges" => [
        edge(
          "e1",
          "stokes_flow_model",
          "model",
          "solve_stokes_flow",
          "model",
          "study_model/stokes_flow_quad_2d",
          "stokes_flow_model"
        ),
        edge(
          "e2",
          "solve_stokes_flow",
          "result",
          "extract_cfd_diagnostics",
          "result",
          "result/stokes_flow_quad_2d",
          "stokes_flow_result"
        ),
        edge(
          "e3",
          "extract_cfd_diagnostics",
          "summary",
          "export_json",
          "summary",
          "report/summary",
          "cfd_diagnostics"
        ),
        edge("e4", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  defp stokes_flow_quad_guard_entry do
    custom_entry(
      "workflow.stokes-flow-quad-guard-json",
      "Stokes flow quad guard JSON",
      "Solves a low-Reynolds 2D Stokes flow quad model, checks CFD divergence and Reynolds thresholds, and exports a JSON guard result.",
      ["fluid"],
      ["fluid", "cfd", "stokes", "flow", "guard", "quad", "2d", "json"],
      stokes_flow_quad_guard_graph(),
      [
        %{
          "node_id" => "stokes_flow_model",
          "artifact_type" => "study_model/stokes_flow_quad_2d",
          "description" =>
            "Steady Stokes flow quad model used as the guarded CFD workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded CFD guard decision for the Stokes flow solve."
        }
      ]
    )
  end

  defp stokes_flow_quad_guard_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.stokes-flow-quad-guard-json",
      "name" => "Stokes flow quad guard JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.stokes_flow_quad_guard/v1",
          [
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "stokes_flow_model",
              "model",
              "study_model/stokes_flow_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "stokes_flow_result",
              "result",
              "result/stokes_flow_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "cfd_diagnostics",
              "result",
              "report/summary"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "cfd_guard",
              "result",
              "report/summary"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "summary_json",
              "export",
              "export/json",
              "utf8_text"
            )
          ],
          %{"workflow_family" => "stokes_flow_quad_guard"}
        ),
      "entry_nodes" => ["stokes_flow_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("stokes_flow_model", "study_model/stokes_flow_quad_2d", "stokes_flow_model"),
        solve_node(),
        diagnostics_node(),
        guard_node(),
        export_node("cfd_guard"),
        output_node()
      ],
      "edges" => [
        edge(
          "e1",
          "stokes_flow_model",
          "model",
          "solve_stokes_flow",
          "model",
          "study_model/stokes_flow_quad_2d",
          "stokes_flow_model"
        ),
        edge(
          "e2",
          "solve_stokes_flow",
          "result",
          "extract_cfd_diagnostics",
          "result",
          "result/stokes_flow_quad_2d",
          "stokes_flow_result"
        ),
        edge(
          "e3",
          "extract_cfd_diagnostics",
          "summary",
          "evaluate_cfd_guard",
          "value",
          "report/summary",
          "cfd_diagnostics"
        ),
        edge(
          "e4",
          "evaluate_cfd_guard",
          "result",
          "export_json",
          "summary",
          "report/summary",
          "cfd_guard"
        ),
        edge("e5", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  defp stokes_flow_quad_benchmark_entry do
    custom_entry(
      "workflow.stokes-flow-quad-benchmark-json",
      "Stokes flow quad benchmark JSON",
      "Solves two low-Reynolds 2D Stokes flow candidates, extracts CFD diagnostics, benchmarks stability metrics, and exports a JSON decision artifact.",
      ["fluid"],
      ["fluid", "cfd", "stokes", "flow", "benchmark", "quad", "2d", "json"],
      stokes_flow_quad_benchmark_graph(),
      [
        %{
          "node_id" => "candidate_a_model",
          "artifact_type" => "study_model/stokes_flow_quad_2d",
          "description" => "First Stokes flow quad candidate model."
        },
        %{
          "node_id" => "candidate_b_model",
          "artifact_type" => "study_model/stokes_flow_quad_2d",
          "description" => "Second Stokes flow quad candidate model."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" =>
            "JSON-encoded CFD benchmark decision across two Stokes flow candidates."
        }
      ]
    )
  end

  defp stokes_flow_quad_benchmark_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.stokes-flow-quad-benchmark-json",
      "name" => "Stokes flow quad benchmark JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.stokes_flow_quad_benchmark/v1",
          [
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "candidate_a_model",
              "model",
              "study_model/stokes_flow_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "candidate_b_model",
              "model",
              "study_model/stokes_flow_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "candidate_a_result",
              "result",
              "result/stokes_flow_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "candidate_b_result",
              "result",
              "result/stokes_flow_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "candidate_a_diagnostics",
              "result",
              "report/summary"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "candidate_b_diagnostics",
              "result",
              "report/summary"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "benchmark_summary",
              "result",
              "report/summary"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "summary_json",
              "export",
              "export/json",
              "utf8_text"
            )
          ],
          %{"workflow_family" => "stokes_flow_quad_benchmark"}
        ),
      "entry_nodes" => ["candidate_a_model", "candidate_b_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("candidate_a_model", "study_model/stokes_flow_quad_2d", "candidate_a_model"),
        input_node("candidate_b_model", "study_model/stokes_flow_quad_2d", "candidate_b_model"),
        solve_node("solve_candidate_a", "candidate_a_model", "candidate_a_result"),
        solve_node("solve_candidate_b", "candidate_b_model", "candidate_b_result"),
        diagnostics_node(
          "extract_candidate_a_diagnostics",
          "candidate_a_result",
          "candidate_a_diagnostics"
        ),
        diagnostics_node(
          "extract_candidate_b_diagnostics",
          "candidate_b_result",
          "candidate_b_diagnostics"
        ),
        benchmark_node(),
        export_node("benchmark_summary"),
        output_node()
      ],
      "edges" => [
        edge(
          "e1",
          "candidate_a_model",
          "model",
          "solve_candidate_a",
          "model",
          "study_model/stokes_flow_quad_2d",
          "candidate_a_model"
        ),
        edge(
          "e2",
          "candidate_b_model",
          "model",
          "solve_candidate_b",
          "model",
          "study_model/stokes_flow_quad_2d",
          "candidate_b_model"
        ),
        edge(
          "e3",
          "solve_candidate_a",
          "result",
          "extract_candidate_a_diagnostics",
          "result",
          "result/stokes_flow_quad_2d",
          "candidate_a_result"
        ),
        edge(
          "e4",
          "solve_candidate_b",
          "result",
          "extract_candidate_b_diagnostics",
          "result",
          "result/stokes_flow_quad_2d",
          "candidate_b_result"
        ),
        edge(
          "e5",
          "extract_candidate_a_diagnostics",
          "summary",
          "benchmark_candidates",
          "left",
          "report/summary",
          "candidate_a_diagnostics"
        ),
        edge(
          "e6",
          "extract_candidate_b_diagnostics",
          "summary",
          "benchmark_candidates",
          "right",
          "report/summary",
          "candidate_b_diagnostics"
        ),
        edge(
          "e7",
          "benchmark_candidates",
          "result",
          "export_json",
          "summary",
          "report/summary",
          "benchmark_summary"
        ),
        edge("e8", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  defp input_node(id, artifact_type, dataset_value) do
    %{"id" => id, "kind" => "input", "outputs" => [port("model", artifact_type, dataset_value)]}
  end

  defp solve_node do
    solve_node("solve_stokes_flow", "stokes_flow_model", "stokes_flow_result")
  end

  defp solve_node(id, input_value, output_value) do
    %{
      "id" => id,
      "kind" => "solve",
      "operator_id" => "solve.stokes_flow_quad_2d",
      "inputs" => [port("model", "study_model/stokes_flow_quad_2d", input_value)],
      "outputs" => [port("result", "result/stokes_flow_quad_2d", output_value)]
    }
  end

  defp diagnostics_node do
    diagnostics_node("extract_cfd_diagnostics", "stokes_flow_result", "cfd_diagnostics")
  end

  defp diagnostics_node(id, input_value, output_value) do
    %{
      "id" => id,
      "kind" => "extract",
      "operator_id" => "extract.stokes_flow_result_diagnostics",
      "inputs" => [port("result", "result/stokes_flow_quad_2d", input_value)],
      "outputs" => [port("summary", "report/summary", output_value)]
    }
  end

  defp benchmark_node do
    %{
      "id" => "benchmark_candidates",
      "kind" => "transform",
      "operator_id" => "transform.benchmark_cfd_pair",
      "config" => %{
        "left_label" => "candidate_a",
        "right_label" => "candidate_b",
        "criteria" => [
          %{"field" => "cfd_divergence_error_peak", "goal" => "min", "weight" => 3.0},
          %{"field" => "cfd_reynolds_number_peak", "goal" => "min", "weight" => 2.0},
          %{"field" => "cfd_viscous_dissipation_total", "goal" => "min", "weight" => 1.0}
        ]
      },
      "inputs" => [
        port("left", "report/summary", "candidate_a_diagnostics"),
        port("right", "report/summary", "candidate_b_diagnostics")
      ],
      "outputs" => [port("result", "report/summary", "benchmark_summary")]
    }
  end

  defp guard_node do
    %{
      "id" => "evaluate_cfd_guard",
      "kind" => "transform",
      "operator_id" => "transform.evaluate_cfd_guard",
      "config" => %{
        "rules" => [
          %{"field" => "cfd_divergence_error_peak", "threshold" => 0.05, "severity" => "block"},
          %{"field" => "cfd_reynolds_number_peak", "threshold" => 100.0, "severity" => "warn"}
        ]
      },
      "inputs" => [port("value", "report/summary", "cfd_diagnostics")],
      "outputs" => [port("result", "report/summary", "cfd_guard")]
    }
  end

  defp export_node(input_value) do
    %{
      "id" => "export_json",
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [port("summary", "report/summary", input_value)],
      "outputs" => [port("json", "export/json", "summary_json")]
    }
  end

  defp output_node do
    %{
      "id" => "json_output",
      "kind" => "output",
      "inputs" => [port("json", "export/json", "summary_json")],
      "outputs" => []
    }
  end

  defp edge(id, from_node, from_port, to_node, to_port, artifact_type, dataset_value) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type,
      "dataset_value" => dataset_value
    }
  end

  defp port(id, artifact_type, dataset_value),
    do: %{"id" => id, "artifact_type" => artifact_type, "dataset_value" => dataset_value}

  defp custom_entry(id, name, summary, domains, capability_tags, graph, entry_inputs, outputs) do
    %{
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "summary" => summary,
      "domains" => domains,
      "capability_tags" => capability_tags,
      "graph" => graph,
      "entry_inputs" => entry_inputs,
      "output_artifacts" => outputs
    }
  end
end
