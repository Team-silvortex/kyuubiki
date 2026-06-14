defmodule KyuubikiWeb.WorkflowTemplateThermalEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def list do
    [
      heat_plane_quad_guard_entry(),
      heat_thermo_quad_benchmark_entry()
    ]
  end

  defp heat_plane_quad_guard_entry do
    custom_entry(
      "workflow.heat-plane-quad-guard-json",
      "Heat plane quad guard JSON",
      "Solves a heat plane quad model, extracts thermal diagnostics, evaluates visible guard thresholds, and exports a JSON gate result.",
      ["thermal"],
      ["thermal", "guard", "diagnostics", "quad", "2d", "json"],
      heat_plane_quad_guard_graph(),
      [
        %{
          "node_id" => "heat_model",
          "artifact_type" => "study_model/heat_plane_quad_2d",
          "description" => "Heat plane quad study model used as the guarded workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded thermal diagnostics and guard decision."
        }
      ]
    )
  end

  defp heat_thermo_quad_benchmark_entry do
    custom_entry(
      "workflow.heat-thermo-quad-benchmark-json",
      "Heat thermo quad benchmark JSON",
      "Solves a heat plane quad model, bridges into thermo-mechanical quad, extracts both diagnostics, benchmarks them, and exports a JSON comparison artifact.",
      ["thermal", "thermo_mechanical"],
      ["thermal", "thermo_mechanical", "benchmark", "diagnostics", "quad", "2d", "json"],
      heat_thermo_quad_benchmark_graph(),
      [
        %{
          "node_id" => "heat_model",
          "artifact_type" => "study_model/heat_plane_quad_2d",
          "description" => "Heat plane quad study model used as the coupled benchmark workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded benchmark artifact across heat and thermo diagnostics."
        }
      ]
    )
  end

  defp heat_plane_quad_guard_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.heat-plane-quad-guard-json",
      "name" => "Heat plane quad guard JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.heat_plane_quad_guard/v1",
          [
            WorkflowCatalogSupport.workflow_dataset_value_info("heat_model", "model", "study_model/heat_plane_quad_2d"),
            WorkflowCatalogSupport.workflow_dataset_value_info("heat_result", "result", "result/heat_plane_quad_2d"),
            WorkflowCatalogSupport.workflow_dataset_value_info("thermal_diagnostics", "result", "report/summary"),
            WorkflowCatalogSupport.workflow_dataset_value_info("thermal_guard", "result", "report/summary"),
            WorkflowCatalogSupport.workflow_dataset_value_info("summary_json", "export", "export/json", "utf8_text")
          ],
          %{"workflow_family" => "heat_plane_quad_guard"}
        ),
      "entry_nodes" => ["heat_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("heat_model", "model", "study_model/heat_plane_quad_2d", "heat_model"),
        solve_node("solve_heat", "solve.heat_plane_quad_2d", "study_model/heat_plane_quad_2d", "result/heat_plane_quad_2d", "heat_model", "heat_result"),
        %{
          "id" => "extract_thermal_diagnostics",
          "kind" => "extract",
          "operator_id" => "extract.thermal_result_diagnostics",
          "inputs" => [port("result", "result/heat_plane_quad_2d", "heat_result")],
          "outputs" => [port("summary", "report/summary", "thermal_diagnostics")]
        },
        %{
          "id" => "evaluate_guard",
          "kind" => "transform",
          "operator_id" => "transform.evaluate_thermal_guard",
          "config" => %{
            "rules" => [
              %{"field" => "thermal_temperature_max", "threshold" => 120.0, "comparison" => "gte", "severity" => "warn", "label" => "temperature ceiling"},
              %{"field" => "thermal_peak_flux_magnitude", "threshold" => 2500.0, "comparison" => "gte", "severity" => "block", "label" => "flux ceiling"}
            ]
          },
          "inputs" => [port("value", "report/summary", "thermal_diagnostics")],
          "outputs" => [port("result", "report/summary", "thermal_guard")]
        },
        export_node("export_json", "report/summary", "thermal_guard", "summary_json"),
        output_node("json_output", "export/json", "summary_json")
      ],
      "edges" => [
        edge("e0", "heat_model", "model", "solve_heat", "model", "study_model/heat_plane_quad_2d", "heat_model"),
        edge("e1", "solve_heat", "result", "extract_thermal_diagnostics", "result", "result/heat_plane_quad_2d", "heat_result"),
        edge("e2", "extract_thermal_diagnostics", "summary", "evaluate_guard", "value", "report/summary", "thermal_diagnostics"),
        edge("e3", "evaluate_guard", "result", "export_json", "summary", "report/summary", "thermal_guard"),
        edge("e4", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  defp heat_thermo_quad_benchmark_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.heat-thermo-quad-benchmark-json",
      "name" => "Heat thermo quad benchmark JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.heat_thermo_quad_benchmark/v1",
          [
            WorkflowCatalogSupport.workflow_dataset_value_info("heat_model", "model", "study_model/heat_plane_quad_2d"),
            WorkflowCatalogSupport.workflow_dataset_value_info("heat_result", "result", "result/heat_plane_quad_2d"),
            WorkflowCatalogSupport.workflow_dataset_value_info("thermal_diagnostics", "result", "report/summary"),
            WorkflowCatalogSupport.workflow_dataset_value_info("thermo_model", "model", "study_model/thermal_plane_quad_2d"),
            WorkflowCatalogSupport.workflow_dataset_value_info("thermo_result", "result", "result/thermal_plane_quad_2d"),
            WorkflowCatalogSupport.workflow_dataset_value_info("thermo_diagnostics", "result", "report/summary"),
            WorkflowCatalogSupport.workflow_dataset_value_info("benchmark_summary", "result", "report/summary"),
            WorkflowCatalogSupport.workflow_dataset_value_info("summary_json", "export", "export/json", "utf8_text")
          ],
          %{"workflow_family" => "heat_thermo_quad_benchmark"}
        ),
      "entry_nodes" => ["heat_model"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("heat_model", "model", "study_model/heat_plane_quad_2d", "heat_model"),
        solve_node("solve_heat", "solve.heat_plane_quad_2d", "study_model/heat_plane_quad_2d", "result/heat_plane_quad_2d", "heat_model", "heat_result"),
        %{
          "id" => "extract_thermal_diagnostics",
          "kind" => "extract",
          "operator_id" => "extract.thermal_result_diagnostics",
          "inputs" => [port("result", "result/heat_plane_quad_2d", "heat_result")],
          "outputs" => [port("summary", "report/summary", "thermal_diagnostics")]
        },
        %{
          "id" => "bridge_temperature",
          "kind" => "transform",
          "operator_id" => "bridge.temperature_field_to_thermo_quad_2d",
          "config" => %{
            "seed_model" => WorkflowCatalogSupport.thermo_quad_seed_model_example(),
            "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
          },
          "inputs" => [port("heat_result", "result/heat_plane_quad_2d", "heat_result")],
          "outputs" => [port("thermo_model", "study_model/thermal_plane_quad_2d", "thermo_model")]
        },
        solve_node("solve_thermo", "solve.thermal_plane_quad_2d", "study_model/thermal_plane_quad_2d", "result/thermal_plane_quad_2d", "thermo_model", "thermo_result"),
        %{
          "id" => "extract_thermo_diagnostics",
          "kind" => "extract",
          "operator_id" => "extract.thermo_result_diagnostics",
          "inputs" => [port("result", "result/thermal_plane_quad_2d", "thermo_result")],
          "outputs" => [port("summary", "report/summary", "thermo_diagnostics")]
        },
        %{
          "id" => "benchmark_coupled",
          "kind" => "transform",
          "operator_id" => "transform.benchmark_coupled_heat_pair",
          "config" => %{
            "left_label" => "thermal",
            "right_label" => "thermo",
            "criteria" => [
              %{
                "field" => "temperature_vs_delta",
                "left_field" => "thermal_temperature_max",
                "right_field" => "thermo_temperature_delta_max",
                "goal" => "min",
                "weight" => 2.0
              },
              %{
                "field" => "loaded_vs_heated_nodes",
                "left_field" => "thermal_loaded_node_count",
                "right_field" => "thermo_heated_node_count",
                "goal" => "min",
                "weight" => 1.0
              },
              %{
                "field" => "flux_vs_stress",
                "left_field" => "thermal_peak_flux_magnitude",
                "right_field" => "thermo_peak_stress",
                "goal" => "min",
                "weight" => 3.0
              }
            ]
          },
          "inputs" => [
            port("left", "report/summary", "thermal_diagnostics"),
            port("right", "report/summary", "thermo_diagnostics")
          ],
          "outputs" => [port("result", "report/summary", "benchmark_summary")]
        },
        export_node("export_json", "report/summary", "benchmark_summary", "summary_json"),
        output_node("json_output", "export/json", "summary_json")
      ],
      "edges" => [
        edge("e0", "heat_model", "model", "solve_heat", "model", "study_model/heat_plane_quad_2d", "heat_model"),
        edge("e1", "solve_heat", "result", "extract_thermal_diagnostics", "result", "result/heat_plane_quad_2d", "heat_result"),
        edge("e2", "solve_heat", "result", "bridge_temperature", "heat_result", "result/heat_plane_quad_2d", "heat_result"),
        edge("e3", "bridge_temperature", "thermo_model", "solve_thermo", "model", "study_model/thermal_plane_quad_2d", "thermo_model"),
        edge("e4", "solve_thermo", "result", "extract_thermo_diagnostics", "result", "result/thermal_plane_quad_2d", "thermo_result"),
        edge("e5", "extract_thermal_diagnostics", "summary", "benchmark_coupled", "left", "report/summary", "thermal_diagnostics"),
        edge("e6", "extract_thermo_diagnostics", "summary", "benchmark_coupled", "right", "report/summary", "thermo_diagnostics"),
        edge("e7", "benchmark_coupled", "result", "export_json", "summary", "report/summary", "benchmark_summary"),
        edge("e8", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end

  defp input_node(id, port_id, artifact_type, dataset_value) do
    %{"id" => id, "kind" => "input", "outputs" => [port(port_id, artifact_type, dataset_value)]}
  end

  defp solve_node(id, operator_id, input_artifact_type, output_artifact_type, input_dataset_value, output_dataset_value) do
    %{
      "id" => id,
      "kind" => "solve",
      "operator_id" => operator_id,
      "inputs" => [port("model", input_artifact_type, input_dataset_value)],
      "outputs" => [port("result", output_artifact_type, output_dataset_value)]
    }
  end

  defp export_node(id, input_artifact_type, input_dataset_value, output_dataset_value) do
    %{
      "id" => id,
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [port("summary", input_artifact_type, input_dataset_value)],
      "outputs" => [port("json", "export/json", output_dataset_value)]
    }
  end

  defp output_node(id, input_artifact_type, input_dataset_value) do
    %{"id" => id, "kind" => "output", "inputs" => [port("json", input_artifact_type, input_dataset_value)], "outputs" => []}
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

  defp port(id, artifact_type, dataset_value) do
    %{"id" => id, "artifact_type" => artifact_type, "dataset_value" => dataset_value}
  end

  defp custom_entry(id, name, summary, domains, capability_tags, graph, entry_inputs, output_artifacts) do
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
