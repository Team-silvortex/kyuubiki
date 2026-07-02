defmodule KyuubikiWeb.WorkflowTemplateBridgeEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowTemplateBridgeContractGraphs
  alias KyuubikiWeb.WorkflowTemplateThermalContractGraphs

  def list do
    [
      heat_to_thermo_quad_entry(),
      heat_to_thermo_quad_comparison_entry(),
      electrostatic_to_heat_triangle_entry(),
      electrostatic_heat_thermo_triangle_entry()
    ]
  end

  defp heat_to_thermo_quad_entry do
    custom_entry(
      "workflow.heat-to-thermo-quad-2d",
      "Heat to thermo quad",
      "Solves a heat plane quad model, bridges the temperature field into a thermo-mechanical quad model, then extracts and exports a summary.",
      ["thermal", "thermo_mechanical"],
      ["thermal", "thermo_mechanical", "workflow_bridge", "quad", "2d"],
      WorkflowTemplateBridgeContractGraphs.heat_to_thermo_quad_graph(),
      [
        %{
          "node_id" => "heat_model",
          "artifact_type" => "study_model/heat_plane_quad_2d",
          "description" => "Heat plane quad study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded result summary for the final thermo-mechanical solve."
        }
      ]
    )
  end

  defp heat_to_thermo_quad_comparison_entry do
    custom_entry(
      "workflow.heat-to-thermo-quad-comparison-json",
      "Heat to thermo quad comparison JSON",
      "Solves a heat quad model, bridges into a thermo-mechanical quad model, then compares heat and thermo summaries in one JSON artifact.",
      ["thermal", "thermo_mechanical"],
      ["thermal", "thermo_mechanical", "summary", "compare", "benchmark", "quad", "2d"],
      WorkflowTemplateThermalContractGraphs.heat_to_thermo_quad_comparison_graph(),
      [
        %{
          "node_id" => "heat_model",
          "artifact_type" => "study_model/heat_plane_quad_2d",
          "description" => "Heat plane quad study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" =>
            "JSON-encoded comparison summary across heat and thermo-mechanical quad solves."
        }
      ]
    )
  end

  defp electrostatic_to_heat_triangle_entry do
    custom_entry(
      "workflow.electrostatic-to-heat-triangle-2d",
      "Electrostatic to heat triangle",
      "Solves an electrostatic triangle model, bridges field magnitude into a heat triangle study, then extracts and exports a heat summary.",
      ["electromagnetic", "thermal"],
      ["electrostatic", "heat", "workflow_bridge", "triangle", "2d"],
      WorkflowTemplateBridgeContractGraphs.electrostatic_to_heat_triangle_graph(),
      [
        %{
          "node_id" => "electrostatic_plane_triangle_model",
          "artifact_type" => "study_model/electrostatic_plane_triangle_2d",
          "description" =>
            "Electrostatic plane triangle study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" =>
            "JSON-encoded heat triangle summary after bridging the electrostatic field."
        }
      ]
    )
  end

  defp electrostatic_heat_thermo_triangle_entry do
    custom_entry(
      "workflow.electrostatic-heat-thermo-triangle-summary-json",
      "Electrostatic heat thermo triangle summary JSON",
      "Runs the full electrostatic triangle to heat triangle to thermo-mechanical triangle chain and exports a JSON summary.",
      ["electromagnetic", "thermal", "thermo_mechanical"],
      [
        "electrostatic",
        "heat",
        "thermal",
        "thermo_mechanical",
        "workflow_bridge",
        "triangle",
        "summary",
        "2d"
      ],
      WorkflowTemplateThermalContractGraphs.electrostatic_heat_thermo_triangle_graph(),
      [
        %{
          "node_id" => "electrostatic_plane_triangle_model",
          "artifact_type" => "study_model/electrostatic_plane_triangle_2d",
          "description" =>
            "Electrostatic plane triangle study model used as the coupled workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded summary for the final thermo-mechanical triangle solve."
        }
      ]
    )
  end

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
