defmodule KyuubikiWeb.WorkflowTemplateDiagnosticsEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowTemplateDiagnosticsGraphs

  def list do
    [
      diagnostics_bundle_guard_report_entry(),
      peak_diagnostics_bundle_report_entry(),
      electrostatic_heat_thermo_diagnostics_entry()
    ]
  end

  defp diagnostics_bundle_guard_report_entry do
    %{
      "id" => "workflow.diagnostics-bundle-guard-report-markdown",
      "name" => "Diagnostics bundle guard report markdown",
      "version" => "1.0.0",
      "summary" =>
        "Compose electrostatic, thermal, and thermo-mechanical diagnostics into a shared bundle, evaluate a unified guard decision, and export a markdown report.",
      "domains" => ["electromagnetic", "thermal", "thermo_mechanical"],
      "capability_tags" => [
        "diagnostics",
        "bundle",
        "guard",
        "report",
        "markdown",
        "headless_safe"
      ],
      "graph" => WorkflowTemplateDiagnosticsGraphs.diagnostics_bundle_guard_report_graph(),
      "entry_inputs" => [
        entry_input("electrostatic_input", "electrostatic diagnostics summary"),
        entry_input("thermal_input", "thermal diagnostics summary"),
        entry_input("thermo_input", "thermo-mechanical diagnostics summary")
      ],
      "output_artifacts" => [
        %{
          "node_id" => "markdown_output",
          "artifact_type" => "export/markdown",
          "description" => "Markdown diagnostics bundle report with unified guard decision."
        }
      ]
    }
  end

  defp electrostatic_heat_thermo_diagnostics_entry do
    %{
      "id" => "workflow.electrostatic-heat-thermo-diagnostics-markdown",
      "name" => "Electrostatic heat thermo diagnostics markdown",
      "version" => "1.0.0",
      "summary" =>
        "Run electrostatic, heat, and thermo-mechanical quad solves in one chain, extract per-stage diagnostics, evaluate a unified guard, and export a markdown report.",
      "domains" => ["electromagnetic", "thermal", "thermo_mechanical"],
      "capability_tags" => [
        "electrostatic",
        "thermal",
        "thermo_mechanical",
        "diagnostics",
        "guard",
        "report",
        "markdown",
        "workflow_bridge",
        "headless_safe"
      ],
      "graph" => WorkflowTemplateDiagnosticsGraphs.electrostatic_heat_thermo_diagnostics_graph(),
      "entry_inputs" => [
        %{
          "node_id" => "electrostatic_model",
          "artifact_type" => "study_model/electrostatic_plane_quad_2d",
          "description" =>
            "Electrostatic plane quad study model used as the workflow entry artifact."
        }
      ],
      "output_artifacts" => [
        %{
          "node_id" => "markdown_output",
          "artifact_type" => "export/markdown",
          "description" =>
            "Markdown diagnostics report for the electrostatic -> heat -> thermo chain."
        }
      ]
    }
  end

  defp peak_diagnostics_bundle_report_entry do
    %{
      "id" => "workflow.peak-diagnostics-bundle-report-markdown",
      "name" => "Peak diagnostics bundle report markdown",
      "version" => "1.0.0",
      "summary" =>
        "Compose electrostatic, thermal, and thermo-mechanical peak extracts into a shared bundle, evaluate a unified guard decision, and export a markdown report.",
      "domains" => ["electromagnetic", "thermal", "thermo_mechanical"],
      "capability_tags" => [
        "peak",
        "diagnostics",
        "bundle",
        "guard",
        "report",
        "markdown",
        "headless_safe"
      ],
      "graph" => WorkflowTemplateDiagnosticsGraphs.peak_diagnostics_bundle_report_graph(),
      "entry_inputs" => [
        entry_input("electrostatic_input", "electrostatic peak summary"),
        entry_input("thermal_input", "thermal peak summary"),
        entry_input("thermo_input", "thermo-mechanical peak summary")
      ],
      "output_artifacts" => [
        %{
          "node_id" => "markdown_output",
          "artifact_type" => "export/markdown",
          "description" => "Markdown peak diagnostics bundle report with unified guard decision."
        }
      ]
    }
  end

  defp entry_input(node_id, description) do
    %{
      "node_id" => node_id,
      "artifact_type" => "artifact/json",
      "description" => description
    }
  end
end
