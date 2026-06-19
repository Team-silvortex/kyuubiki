defmodule KyuubikiWeb.WorkflowTemplateCustomEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowTemplateBridgeContractGraphs
  alias KyuubikiWeb.WorkflowTemplateBridgeEntries
  alias KyuubikiWeb.WorkflowTemplateDiagnosticsEntries
  alias KyuubikiWeb.WorkflowTemplateElectromagneticEntries
  alias KyuubikiWeb.WorkflowTemplateElectromagneticGuardEntries
  alias KyuubikiWeb.WorkflowTemplateElectromagneticGuardThermoEntries
  alias KyuubikiWeb.WorkflowTemplateFocusEntries
  alias KyuubikiWeb.WorkflowTemplateSolverContractGraphs
  alias KyuubikiWeb.WorkflowTemplateThermalEntries

  def list do
    WorkflowTemplateElectromagneticEntries.list() ++
      WorkflowTemplateDiagnosticsEntries.list() ++
      WorkflowTemplateFocusEntries.list() ++
      WorkflowTemplateElectromagneticGuardEntries.list() ++
      WorkflowTemplateElectromagneticGuardThermoEntries.list() ++
      WorkflowTemplateBridgeEntries.list() ++
      WorkflowTemplateThermalEntries.list() ++
      [
        electrostatic_to_heat_quad_entry(),
        electrostatic_plane_quad_entry(),
        electrostatic_plane_quad_hotspot_alert_entry()
      ]
  end

  defp electrostatic_to_heat_quad_entry do
    custom_entry(
      "workflow.electrostatic-to-heat-quad-2d",
      "Electrostatic to heat quad",
      "Solves an electrostatic quad model, bridges field magnitude into a heat quad study, then extracts and exports a heat summary.",
      ["electromagnetic", "thermal"],
      ["electrostatic", "heat", "workflow_bridge", "quad", "2d"],
      WorkflowTemplateBridgeContractGraphs.electrostatic_to_heat_quad_graph(),
      [
        %{
          "node_id" => "electrostatic_model",
          "artifact_type" => "study_model/electrostatic_plane_quad_2d",
          "description" =>
            "Electrostatic plane quad study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded heat summary after bridging the electrostatic field."
        }
      ]
    )
  end

  defp electrostatic_plane_quad_entry do
    custom_entry(
      "workflow.electrostatic-plane-quad-2d",
      "Electrostatic plane quad",
      "Solves a 2D electrostatic quad model, extracts the peak field metrics, and exports a JSON summary artifact.",
      ["electromagnetic"],
      ["electrostatic", "extract", "export", "quad", "2d"],
      WorkflowTemplateSolverContractGraphs.electrostatic_plane_quad_summary_graph(),
      [
        %{
          "node_id" => "electrostatic_model",
          "artifact_type" => "study_model/electrostatic_plane_quad_2d",
          "description" =>
            "Electrostatic plane quad study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "json_output",
          "artifact_type" => "export/json",
          "description" => "JSON-encoded summary for the electrostatic quad solve."
        }
      ]
    )
  end

  defp electrostatic_plane_quad_hotspot_alert_entry do
    custom_entry(
      "workflow.electrostatic-plane-quad-hotspot-alert",
      "Electrostatic plane quad hotspot alert",
      "Solves a 2D electrostatic quad model, extracts electric-field hotspots, and exports a markdown alert artifact.",
      ["electromagnetic"],
      ["electrostatic", "hotspot", "alert", "field", "quad", "2d"],
      WorkflowTemplateSolverContractGraphs.electrostatic_plane_quad_hotspot_alert_graph(),
      [
        %{
          "node_id" => "electrostatic_model",
          "artifact_type" => "study_model/electrostatic_plane_quad_2d",
          "description" =>
            "Electrostatic plane quad study model used as the workflow entry artifact."
        }
      ],
      [
        %{
          "node_id" => "markdown_output",
          "artifact_type" => "export/markdown",
          "description" => "Markdown alert describing electrostatic hotspot regions."
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
