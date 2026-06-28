defmodule KyuubikiWeb.WorkflowTemplateMagnetostaticEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def list do
    [
      magnetostatic_plane_quad_guard_entry(),
      magnetostatic_plane_quad_benchmark_entry()
    ]
  end

  defp magnetostatic_plane_quad_guard_entry do
    custom_entry(
      "workflow.magnetostatic-plane-quad-guard-json",
      "Magnetostatic plane quad guard JSON",
      "Solves a magnetostatic quad model, extracts magnetic diagnostics, evaluates visible guard thresholds, and exports a JSON gate result.",
      ["electromagnetic"],
      ["magnetostatic", "guard", "diagnostics", "quad", "2d", "json"],
      magnetostatic_plane_quad_guard_graph(),
      [
        %{
          "node_id" => "magnetostatic_model",
          "artifact_type" => "study_model/magnetostatic_plane_quad_2d",
          "description" =>
            "Magnetostatic plane quad study model used as the guarded workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded magnetostatic diagnostics and guard decision."
        }
      ]
    )
  end

  defp magnetostatic_plane_quad_guard_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.magnetostatic-plane-quad-guard-json",
      "name" => "Magnetostatic plane quad guard JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.magnetostatic_plane_quad_guard/v1",
          [
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "magnetostatic_model",
              "model",
              "study_model/magnetostatic_plane_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "magnetostatic_result",
              "result",
              "result/magnetostatic_plane_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "magnetostatic_diagnostics",
              "result",
              "report/summary"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "magnetostatic_guard",
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
          %{"workflow_family" => "magnetostatic_plane_quad_guard"}
        ),
      "entry_nodes" => ["magnetostatic_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node(
          "magnetostatic_model",
          "model",
          "study_model/magnetostatic_plane_quad_2d",
          "magnetostatic_model"
        ),
        solve_node(
          "solve_magnetostatic",
          "solve.magnetostatic_plane_quad_2d",
          "study_model/magnetostatic_plane_quad_2d",
          "result/magnetostatic_plane_quad_2d",
          "magnetostatic_model",
          "magnetostatic_result"
        ),
        %{
          "id" => "extract_magnetostatic_diagnostics",
          "kind" => "extract",
          "operator_id" => "extract.magnetostatic_result_diagnostics",
          "inputs" => [
            port("result", "result/magnetostatic_plane_quad_2d", "magnetostatic_result")
          ],
          "outputs" => [port("summary", "report/summary", "magnetostatic_diagnostics")]
        },
        %{
          "id" => "evaluate_guard",
          "kind" => "transform",
          "operator_id" => "transform.evaluate_magnetostatic_guard",
          "config" => %{
            "rules" => [
              %{
                "field" => "magnetostatic_field_peak_magnitude",
                "threshold" => 12_000.0,
                "comparison" => "gte",
                "severity" => "warn",
                "label" => "H-field ceiling"
              },
              %{
                "field" => "magnetostatic_flux_peak_magnitude",
                "threshold" => 2.5,
                "comparison" => "gte",
                "severity" => "block",
                "label" => "B-field ceiling"
              }
            ]
          },
          "inputs" => [port("value", "report/summary", "magnetostatic_diagnostics")],
          "outputs" => [port("result", "report/summary", "magnetostatic_guard")]
        },
        export_node("export_json", "report/summary", "magnetostatic_guard", "summary_json"),
        output_node("json_output", "export/json", "summary_json")
      ],
      "edges" => [
        edge(
          "e1",
          "magnetostatic_model",
          "model",
          "solve_magnetostatic",
          "model",
          "study_model/magnetostatic_plane_quad_2d",
          "magnetostatic_model"
        ),
        edge(
          "e2",
          "solve_magnetostatic",
          "result",
          "extract_magnetostatic_diagnostics",
          "result",
          "result/magnetostatic_plane_quad_2d",
          "magnetostatic_result"
        ),
        edge(
          "e3",
          "extract_magnetostatic_diagnostics",
          "summary",
          "evaluate_guard",
          "value",
          "report/summary",
          "magnetostatic_diagnostics"
        ),
        edge(
          "e4",
          "evaluate_guard",
          "result",
          "export_json",
          "summary",
          "report/summary",
          "magnetostatic_guard"
        ),
        edge("e5", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  defp magnetostatic_plane_quad_benchmark_entry do
    custom_entry(
      "workflow.magnetostatic-plane-quad-benchmark-json",
      "Magnetostatic plane quad benchmark JSON",
      "Solves two magnetostatic quad candidates, extracts magnetic diagnostics, benchmarks field and energy metrics, and exports a JSON decision artifact.",
      ["electromagnetic"],
      ["magnetostatic", "benchmark", "diagnostics", "quad", "2d", "json"],
      magnetostatic_plane_quad_benchmark_graph(),
      [
        %{
          "node_id" => "candidate_a_model",
          "artifact_type" => "study_model/magnetostatic_plane_quad_2d",
          "description" => "First magnetostatic quad candidate model."
        },
        %{
          "node_id" => "candidate_b_model",
          "artifact_type" => "study_model/magnetostatic_plane_quad_2d",
          "description" => "Second magnetostatic quad candidate model."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded benchmark result across two magnetostatic candidates."
        }
      ]
    )
  end

  defp magnetostatic_plane_quad_benchmark_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.magnetostatic-plane-quad-benchmark-json",
      "name" => "Magnetostatic plane quad benchmark JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.magnetostatic_plane_quad_benchmark/v1",
          [
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "candidate_a_model",
              "model",
              "study_model/magnetostatic_plane_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "candidate_b_model",
              "model",
              "study_model/magnetostatic_plane_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "candidate_a_result",
              "result",
              "result/magnetostatic_plane_quad_2d"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "candidate_b_result",
              "result",
              "result/magnetostatic_plane_quad_2d"
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
          %{"workflow_family" => "magnetostatic_plane_quad_benchmark"}
        ),
      "entry_nodes" => ["candidate_a_model", "candidate_b_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node(
          "candidate_a_model",
          "model",
          "study_model/magnetostatic_plane_quad_2d",
          "candidate_a_model"
        ),
        input_node(
          "candidate_b_model",
          "model",
          "study_model/magnetostatic_plane_quad_2d",
          "candidate_b_model"
        ),
        solve_node(
          "solve_candidate_a",
          "solve.magnetostatic_plane_quad_2d",
          "study_model/magnetostatic_plane_quad_2d",
          "result/magnetostatic_plane_quad_2d",
          "candidate_a_model",
          "candidate_a_result"
        ),
        solve_node(
          "solve_candidate_b",
          "solve.magnetostatic_plane_quad_2d",
          "study_model/magnetostatic_plane_quad_2d",
          "result/magnetostatic_plane_quad_2d",
          "candidate_b_model",
          "candidate_b_result"
        ),
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
        %{
          "id" => "benchmark_candidates",
          "kind" => "transform",
          "operator_id" => "transform.benchmark_magnetostatic_pair",
          "config" => %{
            "left_label" => "candidate_a",
            "right_label" => "candidate_b",
            "criteria" => [
              %{
                "field" => "magnetostatic_field_peak_magnitude",
                "goal" => "min",
                "weight" => 2.0
              },
              %{
                "field" => "magnetostatic_flux_peak_magnitude",
                "goal" => "min",
                "weight" => 2.0
              },
              %{"field" => "total_stored_energy", "goal" => "min", "weight" => 1.0}
            ]
          },
          "inputs" => [
            port("left", "report/summary", "candidate_a_diagnostics"),
            port("right", "report/summary", "candidate_b_diagnostics")
          ],
          "outputs" => [port("result", "report/summary", "benchmark_summary")]
        },
        export_node("export_json", "report/summary", "benchmark_summary", "summary_json"),
        output_node("json_output", "export/json", "summary_json")
      ],
      "edges" => [
        edge(
          "e1",
          "candidate_a_model",
          "model",
          "solve_candidate_a",
          "model",
          "study_model/magnetostatic_plane_quad_2d",
          "candidate_a_model"
        ),
        edge(
          "e2",
          "candidate_b_model",
          "model",
          "solve_candidate_b",
          "model",
          "study_model/magnetostatic_plane_quad_2d",
          "candidate_b_model"
        ),
        edge(
          "e3",
          "solve_candidate_a",
          "result",
          "extract_candidate_a_diagnostics",
          "result",
          "result/magnetostatic_plane_quad_2d",
          "candidate_a_result"
        ),
        edge(
          "e4",
          "solve_candidate_b",
          "result",
          "extract_candidate_b_diagnostics",
          "result",
          "result/magnetostatic_plane_quad_2d",
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

  defp diagnostics_node(id, input_value, output_value) do
    %{
      "id" => id,
      "kind" => "extract",
      "operator_id" => "extract.magnetostatic_result_diagnostics",
      "inputs" => [port("result", "result/magnetostatic_plane_quad_2d", input_value)],
      "outputs" => [port("summary", "report/summary", output_value)]
    }
  end

  defp input_node(id, port_id, artifact_type, dataset_value),
    do: %{
      "id" => id,
      "kind" => "input",
      "outputs" => [port(port_id, artifact_type, dataset_value)]
    }

  defp solve_node(id, operator_id, input_type, output_type, input_value, output_value) do
    %{
      "id" => id,
      "kind" => "solve",
      "operator_id" => operator_id,
      "inputs" => [port("model", input_type, input_value)],
      "outputs" => [port("result", output_type, output_value)]
    }
  end

  defp export_node(id, input_type, input_value, output_value) do
    %{
      "id" => id,
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [port("summary", input_type, input_value)],
      "outputs" => [port("json", "export/json", output_value)]
    }
  end

  defp output_node(id, input_type, input_value),
    do: %{
      "id" => id,
      "kind" => "output",
      "inputs" => [port("json", input_type, input_value)],
      "outputs" => []
    }

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

  defp custom_entry(
         id,
         name,
         summary,
         domains,
         capability_tags,
         graph,
         entry_inputs,
         output_artifacts
       ) do
    %{
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "summary" => summary,
      "domains" => domains,
      "capability_tags" => capability_tags,
      "graph" => graph,
      "entry_inputs" => entry_inputs,
      "output_artifacts" => output_artifacts
    }
  end
end
