defmodule KyuubikiWeb.WorkflowTemplateElectromagneticGuardThermoEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def list do
    [
      electrostatic_preheat_guard_heat_thermo_entry(),
      electrostatic_triangle_preheat_guard_heat_thermo_entry()
    ]
  end

  defp electrostatic_preheat_guard_heat_thermo_entry do
    guard_heat_thermo_entry(
      "workflow.electrostatic-preheat-guard-heat-thermo-json",
      "Electrostatic pre-heat guard -> heat -> thermo JSON",
      "Evaluate electrostatic quad risk, hold on high field, or automatically continue through heat and thermo-mechanical quad solves when clear.",
      "electrostatic_model",
      "study_model/electrostatic_plane_quad_2d",
      "solve.electrostatic_plane_quad_2d",
      "result/electrostatic_plane_quad_2d",
      "bridge.electrostatic_field_to_heat_quad_2d",
      "study_model/heat_plane_quad_2d",
      "solve.heat_plane_quad_2d",
      "result/heat_plane_quad_2d",
      "bridge.temperature_field_to_thermo_quad_2d",
      "study_model/thermal_plane_quad_2d",
      "solve.thermal_plane_quad_2d",
      "result/thermal_plane_quad_2d",
      heat_quad_seed_model(),
      WorkflowCatalogSupport.thermo_quad_seed_model_example()
    )
  end

  defp electrostatic_triangle_preheat_guard_heat_thermo_entry do
    guard_heat_thermo_entry(
      "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json",
      "Electrostatic triangle pre-heat guard -> heat -> thermo JSON",
      "Evaluate electrostatic triangle risk, hold on high field, or automatically continue through heat and thermo-mechanical triangle solves when clear.",
      "electrostatic_plane_triangle_model",
      "study_model/electrostatic_plane_triangle_2d",
      "solve.electrostatic_plane_triangle_2d",
      "result/electrostatic_plane_triangle_2d",
      "bridge.electrostatic_field_to_heat_triangle_2d",
      "study_model/heat_plane_triangle_2d",
      "solve.heat_plane_triangle_2d",
      "result/heat_plane_triangle_2d",
      "bridge.temperature_field_to_thermo_triangle_2d",
      "study_model/thermal_plane_triangle_2d",
      "solve.thermal_plane_triangle_2d",
      "result/thermal_plane_triangle_2d",
      heat_triangle_seed_model(),
      WorkflowCatalogSupport.thermo_triangle_seed_model_example()
    )
  end

  defp guard_heat_thermo_entry(
         id,
         name,
         summary,
         entry_node_id,
         entry_artifact_type,
         electrostatic_solve_operator_id,
         electrostatic_result_artifact_type,
         electrostatic_heat_bridge_operator_id,
         heat_model_artifact_type,
         heat_solve_operator_id,
         heat_result_artifact_type,
         heat_thermo_bridge_operator_id,
         thermo_model_artifact_type,
         thermo_solve_operator_id,
         thermo_result_artifact_type,
         heat_seed_model,
         thermo_seed_model
       ) do
    %{
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "summary" => summary,
      "domains" => ["electromagnetic", "thermal", "thermo_mechanical"],
      "capability_tags" => [
        "electrostatic",
        "heat",
        "thermal",
        "thermo_mechanical",
        "guard",
        "workflow_bridge",
        "condition",
        "2d"
      ],
      "graph" =>
        build_graph(
          id,
          name,
          entry_node_id,
          entry_artifact_type,
          electrostatic_solve_operator_id,
          electrostatic_result_artifact_type,
          electrostatic_heat_bridge_operator_id,
          heat_model_artifact_type,
          heat_solve_operator_id,
          heat_result_artifact_type,
          heat_thermo_bridge_operator_id,
          thermo_model_artifact_type,
          thermo_solve_operator_id,
          thermo_result_artifact_type,
          heat_seed_model,
          thermo_seed_model
        ),
      "entry_inputs" => [
        %{
          "node_id" => entry_node_id,
          "artifact_type" => entry_artifact_type,
          "description" => "Electrostatic study model used as the guarded coupled workflow entry artifact."
        }
      ],
      "output_artifacts" => [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" =>
            "JSON-encoded guarded coupled result: hotspot hold summary when blocked, thermo summary when cleared."
        }
      ]
    }
  end

  defp build_graph(
         id,
         name,
         entry_node_id,
         entry_artifact_type,
         electrostatic_solve_operator_id,
         electrostatic_result_artifact_type,
         electrostatic_heat_bridge_operator_id,
         heat_model_artifact_type,
         heat_solve_operator_id,
         heat_result_artifact_type,
         heat_thermo_bridge_operator_id,
         thermo_model_artifact_type,
         thermo_solve_operator_id,
         thermo_result_artifact_type,
         heat_seed_model,
         thermo_seed_model
       ) do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => id,
      "name" => name,
      "version" => "1.0.0",
      "entry_nodes" => [entry_node_id],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node(entry_node_id, entry_artifact_type),
        solve_node("solve_electrostatic", electrostatic_solve_operator_id, entry_artifact_type, electrostatic_result_artifact_type),
        condition_node(electrostatic_result_artifact_type),
        hotspot_extract_node(electrostatic_result_artifact_type),
        bridge_node(
          "bridge_field_to_heat",
          electrostatic_heat_bridge_operator_id,
          electrostatic_result_artifact_type,
          "electrostatic_result",
          heat_model_artifact_type,
          "heat_model",
          %{
            "seed_model" => heat_seed_model,
            "contract" => electrostatic_heat_contract(electrostatic_heat_bridge_operator_id)
          }
        ),
        solve_node("solve_heat", heat_solve_operator_id, heat_model_artifact_type, heat_result_artifact_type),
        bridge_node(
          "bridge_temperature",
          heat_thermo_bridge_operator_id,
          heat_result_artifact_type,
          "heat_result",
          thermo_model_artifact_type,
          "thermo_model",
          %{
            "seed_model" => thermo_seed_model,
            "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
          }
        ),
        solve_node("solve_thermo", thermo_solve_operator_id, thermo_model_artifact_type, thermo_result_artifact_type),
        thermo_summary_node(thermo_result_artifact_type),
        merge_node(),
        export_json_node(),
        output_node()
      ],
      "edges" => [
        edge("e0", entry_node_id, "model", "solve_electrostatic", "model", entry_artifact_type),
        edge("e1", "solve_electrostatic", "result", "gate", "value", electrostatic_result_artifact_type),
        edge("e2", "gate", "if_true", "field_hotspots", "result", electrostatic_result_artifact_type),
        edge("e3", "gate", "if_false", "bridge_field_to_heat", "electrostatic_result", electrostatic_result_artifact_type),
        edge("e4", "bridge_field_to_heat", "heat_model", "solve_heat", "model", heat_model_artifact_type),
        edge("e5", "solve_heat", "result", "bridge_temperature", "heat_result", heat_result_artifact_type),
        edge("e6", "bridge_temperature", "thermo_model", "solve_thermo", "model", thermo_model_artifact_type),
        edge("e7", "solve_thermo", "result", "extract_thermo_summary", "result", thermo_result_artifact_type),
        edge("e8", "field_hotspots", "summary", "merge_summary", "left", "report/summary"),
        edge("e9", "extract_thermo_summary", "summary", "merge_summary", "right", "report/summary"),
        edge("e10", "merge_summary", "result", "export_json", "summary", "report/summary"),
        edge("e11", "export_json", "json", "json_output", "json", "export/json")
      ]
    }
  end

  defp input_node(id, artifact_type) do
    %{
      "id" => id,
      "kind" => "input",
      "outputs" => [%{"id" => "model", "artifact_type" => artifact_type}]
    }
  end

  defp solve_node(id, operator_id, input_artifact_type, output_artifact_type) do
    %{
      "id" => id,
      "kind" => "solve",
      "operator_id" => operator_id,
      "inputs" => [%{"id" => "model", "artifact_type" => input_artifact_type}],
      "outputs" => [%{"id" => "result", "artifact_type" => output_artifact_type}]
    }
  end

  defp condition_node(electrostatic_result_artifact_type) do
    %{
      "id" => "gate",
      "kind" => "condition",
      "config" => %{
        "predicate" => %{"path" => "max_electric_field", "operator" => "gt", "value" => 8.0}
      },
      "inputs" => [%{"id" => "value", "artifact_type" => electrostatic_result_artifact_type}],
      "outputs" => [
        %{"id" => "if_true", "artifact_type" => electrostatic_result_artifact_type},
        %{"id" => "if_false", "artifact_type" => electrostatic_result_artifact_type}
      ]
    }
  end

  defp hotspot_extract_node(electrostatic_result_artifact_type) do
    %{
      "id" => "field_hotspots",
      "kind" => "extract",
      "operator_id" => "extract.field_hotspots",
      "config" => %{
        "source" => "elements",
        "field" => "electric_field_magnitude",
        "output_prefix" => "field",
        "percentile" => 90,
        "sample_limit" => 4,
        "sample_sort" => "value_desc"
      },
      "inputs" => [%{"id" => "result", "artifact_type" => electrostatic_result_artifact_type}],
      "outputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}]
    }
  end

  defp bridge_node(id, operator_id, input_artifact_type, input_id, output_artifact_type, output_id, config) do
    %{
      "id" => id,
      "kind" => "transform",
      "operator_id" => operator_id,
      "config" => config,
      "inputs" => [%{"id" => input_id, "artifact_type" => input_artifact_type}],
      "outputs" => [%{"id" => output_id, "artifact_type" => output_artifact_type}]
    }
  end

  defp thermo_summary_node(thermo_result_artifact_type) do
    %{
      "id" => "extract_thermo_summary",
      "kind" => "extract",
      "operator_id" => "extract.result_summary",
      "config" => %{"fields" => ["max_displacement", "max_stress", "max_temperature_delta"]},
      "inputs" => [%{"id" => "result", "artifact_type" => thermo_result_artifact_type}],
      "outputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}]
    }
  end

  defp merge_node do
    %{
      "id" => "merge_summary",
      "kind" => "transform",
      "operator_id" => "transform.first_available",
      "inputs" => [
        %{"id" => "left", "artifact_type" => "report/summary"},
        %{"id" => "right", "artifact_type" => "report/summary"}
      ],
      "outputs" => [%{"id" => "result", "artifact_type" => "report/summary"}]
    }
  end

  defp export_json_node do
    %{
      "id" => "export_json",
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [%{"id" => "summary", "artifact_type" => "report/summary"}],
      "outputs" => [%{"id" => "json", "artifact_type" => "export/json"}]
    }
  end

  defp output_node do
    %{"id" => "json_output", "kind" => "output", "inputs" => [%{"id" => "json", "artifact_type" => "export/json"}], "outputs" => []}
  end

  defp edge(id, from_node, from_port, to_node, to_port, artifact_type) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type
    }
  end

  defp electrostatic_heat_contract("bridge.electrostatic_field_to_heat_quad_2d") do
    %{
      "version" => "kyuubiki.bridge-contract/v1",
      "source" => %{
        "field" => "electric_field_magnitude",
        "distribution" => "element_to_nodes",
        "node_index_fields" => ["node_i", "node_j", "node_k", "node_l"]
      },
      "transform" => %{"scale" => 50.0, "reduction" => "mean", "default_value" => 0.0},
      "target" => %{"field" => "heat_load"}
    }
  end

  defp electrostatic_heat_contract("bridge.electrostatic_field_to_heat_triangle_2d") do
    %{
      "version" => "kyuubiki.bridge-contract/v1",
      "source" => %{
        "field" => "electric_field_magnitude",
        "distribution" => "element_to_nodes",
        "node_index_fields" => ["node_i", "node_j", "node_k"]
      },
      "transform" => %{"scale" => 50.0, "reduction" => "mean", "default_value" => 0.0},
      "target" => %{"field" => "heat_load"}
    }
  end

  defp heat_quad_seed_model do
    %{
      "nodes" => [
        %{"id" => "h0", "x" => 0.0, "y" => 0.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0},
        %{"id" => "h1", "x" => 1.0, "y" => 0.0, "fix_temperature" => false, "temperature" => 0.0, "heat_load" => 0.0},
        %{"id" => "h2", "x" => 1.0, "y" => 1.0, "fix_temperature" => false, "temperature" => 0.0, "heat_load" => 0.0},
        %{"id" => "h3", "x" => 0.0, "y" => 1.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0}
      ],
      "elements" => [
        %{"id" => "hq0", "node_i" => 0, "node_j" => 1, "node_k" => 2, "node_l" => 3, "thickness" => 0.02, "conductivity" => 45.0}
      ]
    }
  end

  defp heat_triangle_seed_model do
    %{
      "nodes" => [
        %{"id" => "h0", "x" => 0.0, "y" => 0.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0},
        %{"id" => "h1", "x" => 1.0, "y" => 0.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0},
        %{"id" => "h2", "x" => 0.0, "y" => 1.0, "fix_temperature" => false, "temperature" => 0.0, "heat_load" => 0.0}
      ],
      "elements" => [
        %{"id" => "ht0", "node_i" => 0, "node_j" => 1, "node_k" => 2, "thickness" => 0.02, "conductivity" => 45.0}
      ]
    }
  end
end
