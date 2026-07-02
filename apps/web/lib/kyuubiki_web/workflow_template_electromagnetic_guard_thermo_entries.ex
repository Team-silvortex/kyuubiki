defmodule KyuubikiWeb.WorkflowTemplateElectromagneticGuardThermoEntries do
  @moduledoc false

  import KyuubikiWeb.WorkflowTemplateElectromagneticGuardThermoSupport,
    only: [
      bridge_node: 9,
      bridge_validator_node: 7,
      condition_node: 2,
      edge: 7,
      electrostatic_heat_contract: 1,
      export_json_node: 0,
      heat_quad_seed_model: 0,
      heat_triangle_seed_model: 0,
      hotspot_extract_node: 3,
      input_node: 3,
      merge_node: 0,
      output_node: 1,
      solve_node: 6,
      thermo_summary_node: 3
    ]

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
          "description" =>
            "Electrostatic study model used as the guarded coupled workflow entry artifact."
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
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.#{String.replace(id, "workflow.", "")}/v1",
          [
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "electrostatic_model",
              "model",
              entry_artifact_type
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "electrostatic_result",
              "result",
              electrostatic_result_artifact_type
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "hotspot_summary",
              "result",
              "report/summary"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "heat_model",
              "model",
              heat_model_artifact_type
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "heat_result",
              "result",
              heat_result_artifact_type
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "thermo_model",
              "model",
              thermo_model_artifact_type
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "thermo_result",
              "result",
              thermo_result_artifact_type
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "thermo_summary",
              "result",
              "report/summary"
            ),
            WorkflowCatalogSupport.workflow_dataset_value_info(
              "merged_summary",
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
          %{"workflow_family" => "electrostatic_guard_heat_thermo"}
        ),
      "entry_nodes" => [entry_node_id],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node(entry_node_id, entry_artifact_type, "electrostatic_model"),
        solve_node(
          "solve_electrostatic",
          electrostatic_solve_operator_id,
          entry_artifact_type,
          electrostatic_result_artifact_type,
          "electrostatic_model",
          "electrostatic_result"
        ),
        condition_node(electrostatic_result_artifact_type, "electrostatic_result"),
        hotspot_extract_node(
          electrostatic_result_artifact_type,
          "electrostatic_result",
          "hotspot_summary"
        ),
        bridge_node(
          "bridge_field_to_heat",
          electrostatic_heat_bridge_operator_id,
          electrostatic_result_artifact_type,
          "electrostatic_result",
          heat_model_artifact_type,
          "heat_model",
          "electrostatic_result",
          "heat_model",
          %{
            "seed_model" => heat_seed_model,
            "contract" => electrostatic_heat_contract(electrostatic_heat_bridge_operator_id)
          }
        ),
        bridge_validator_node(
          "validate_bridge_field_to_heat",
          "transform.validate_electrostatic_heat_bridge",
          electrostatic_result_artifact_type,
          "electrostatic_result",
          heat_model_artifact_type,
          "heat_model",
          %{
            "contract" => electrostatic_heat_contract(electrostatic_heat_bridge_operator_id)
          }
        ),
        solve_node(
          "solve_heat",
          heat_solve_operator_id,
          heat_model_artifact_type,
          heat_result_artifact_type,
          "heat_model",
          "heat_result"
        ),
        bridge_node(
          "bridge_temperature",
          heat_thermo_bridge_operator_id,
          heat_result_artifact_type,
          "heat_result",
          thermo_model_artifact_type,
          "thermo_model",
          "heat_result",
          "thermo_model",
          %{
            "seed_model" => thermo_seed_model,
            "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
          }
        ),
        bridge_validator_node(
          "validate_bridge_temperature",
          "transform.validate_heat_thermo_bridge",
          heat_result_artifact_type,
          "heat_result",
          thermo_model_artifact_type,
          "thermo_model",
          %{
            "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
          }
        ),
        solve_node(
          "solve_thermo",
          thermo_solve_operator_id,
          thermo_model_artifact_type,
          thermo_result_artifact_type,
          "thermo_model",
          "thermo_result"
        ),
        thermo_summary_node(thermo_result_artifact_type, "thermo_result", "thermo_summary"),
        merge_node(),
        export_json_node(),
        output_node("summary_json")
      ],
      "edges" => [
        edge(
          "e0",
          entry_node_id,
          "model",
          "solve_electrostatic",
          "model",
          entry_artifact_type,
          "electrostatic_model"
        ),
        edge(
          "e1",
          "solve_electrostatic",
          "result",
          "gate",
          "value",
          electrostatic_result_artifact_type,
          "electrostatic_result"
        ),
        edge(
          "e2",
          "gate",
          "if_true",
          "field_hotspots",
          "result",
          electrostatic_result_artifact_type,
          "electrostatic_result"
        ),
        edge(
          "e3",
          "gate",
          "if_false",
          "bridge_field_to_heat",
          "electrostatic_result",
          electrostatic_result_artifact_type,
          "electrostatic_result"
        ),
        edge(
          "e4",
          "solve_electrostatic",
          "result",
          "validate_bridge_field_to_heat",
          "electrostatic_result",
          electrostatic_result_artifact_type,
          "electrostatic_result"
        ),
        edge(
          "e5",
          "bridge_field_to_heat",
          "heat_model",
          "validate_bridge_field_to_heat",
          "heat_model",
          heat_model_artifact_type,
          "heat_model"
        ),
        edge(
          "e6",
          "validate_bridge_field_to_heat",
          "heat_model",
          "solve_heat",
          "model",
          heat_model_artifact_type,
          "heat_model"
        ),
        edge(
          "e7",
          "solve_heat",
          "result",
          "bridge_temperature",
          "heat_result",
          heat_result_artifact_type,
          "heat_result"
        ),
        edge(
          "e8",
          "solve_heat",
          "result",
          "validate_bridge_temperature",
          "heat_result",
          heat_result_artifact_type,
          "heat_result"
        ),
        edge(
          "e9",
          "bridge_temperature",
          "thermo_model",
          "validate_bridge_temperature",
          "thermo_model",
          thermo_model_artifact_type,
          "thermo_model"
        ),
        edge(
          "e10",
          "validate_bridge_temperature",
          "thermo_model",
          "solve_thermo",
          "model",
          thermo_model_artifact_type,
          "thermo_model"
        ),
        edge(
          "e11",
          "solve_thermo",
          "result",
          "extract_thermo_summary",
          "result",
          thermo_result_artifact_type,
          "thermo_result"
        ),
        edge(
          "e12",
          "field_hotspots",
          "summary",
          "merge_summary",
          "left",
          "report/summary",
          "hotspot_summary"
        ),
        edge(
          "e13",
          "extract_thermo_summary",
          "summary",
          "merge_summary",
          "right",
          "report/summary",
          "thermo_summary"
        ),
        edge(
          "e14",
          "merge_summary",
          "result",
          "export_json",
          "summary",
          "report/summary",
          "merged_summary"
        ),
        edge("e15", "export_json", "json", "json_output", "json", "export/json", "summary_json")
      ]
    }
  end
end
