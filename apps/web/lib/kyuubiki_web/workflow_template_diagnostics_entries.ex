defmodule KyuubikiWeb.WorkflowTemplateDiagnosticsEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def list do
    [diagnostics_bundle_guard_report_entry()]
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
      "graph" => build_graph(),
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

  defp build_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.diagnostics-bundle-guard-report-markdown",
      "name" => "Diagnostics bundle guard report markdown",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.diagnostics_bundle_guard_report/v1",
          [
            dataset_value("electrostatic_diagnostics", "result", "artifact/json"),
            dataset_value("thermal_diagnostics", "result", "artifact/json"),
            dataset_value("thermo_diagnostics", "result", "artifact/json"),
            dataset_value("diagnostics_bundle", "result", "artifact/json"),
            dataset_value("guard_result", "result", "artifact/json"),
            dataset_value("report_payload", "result", "artifact/json"),
            dataset_value("markdown_report", "export", "export/markdown", "utf8_text")
          ],
          %{"workflow_family" => "diagnostics_bundle_guard_report"}
        ),
      "entry_nodes" => ["electrostatic_input", "thermal_input", "thermo_input"],
      "output_nodes" => ["markdown_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("electrostatic_input", "electrostatic", "electrostatic_diagnostics"),
        input_node("thermal_input", "thermal", "thermal_diagnostics"),
        input_node("thermo_input", "thermo", "thermo_diagnostics"),
        bundle_node(),
        guard_node(),
        report_node(),
        export_node(),
        output_node()
      ],
      "edges" => [
        edge("e0", "electrostatic_input", "summary", "bundle", "electrostatic"),
        edge("e1", "thermal_input", "summary", "bundle", "thermal"),
        edge("e2", "thermo_input", "summary", "bundle", "thermo"),
        edge("e3", "bundle", "result", "guard", "bundle"),
        edge("e4", "bundle", "result", "report", "bundle"),
        edge("e5", "guard", "result", "report", "guard"),
        edge("e6", "report", "result", "export", "bundle"),
        edge("e7", "export", "markdown", "markdown_output", "markdown", "export/markdown")
      ]
    }
  end

  defp input_node(id, dataset_value, input_label) do
    %{
      "id" => id,
      "kind" => "input",
      "outputs" => [%{"id" => "summary", "artifact_type" => "artifact/json", "dataset_value" => input_label, "description" => dataset_value}]
    }
  end

  defp bundle_node do
    %{
      "id" => "bundle",
      "kind" => "transform",
      "operator_id" => "transform.compose_diagnostics_bundle",
      "config" => %{},
      "inputs" => [
        %{"id" => "electrostatic", "artifact_type" => "artifact/json", "dataset_value" => "electrostatic_diagnostics"},
        %{"id" => "thermal", "artifact_type" => "artifact/json", "dataset_value" => "thermal_diagnostics"},
        %{"id" => "thermo", "artifact_type" => "artifact/json", "dataset_value" => "thermo_diagnostics"}
      ],
      "outputs" => [%{"id" => "result", "artifact_type" => "artifact/json", "dataset_value" => "diagnostics_bundle"}]
    }
  end

  defp guard_node do
    %{
      "id" => "guard",
      "kind" => "transform",
      "operator_id" => "transform.evaluate_diagnostics_bundle_guard",
      "config" => %{
        "rules" => [
          %{"source" => "thermal", "field" => "thermal_temperature_max", "threshold" => 120.0, "severity" => "warn", "label" => "thermal temperature"},
          %{"source" => "thermo", "field" => "thermo_peak_stress", "comparison" => "gt", "threshold" => 180.0, "severity" => "block", "label" => "stress ceiling"},
          %{"source" => "electrostatic", "field" => "electrostatic_field_peak_magnitude", "comparison" => "gt", "threshold" => 9.0, "severity" => "warn", "label" => "field ceiling"}
        ]
      },
      "inputs" => [%{"id" => "bundle", "artifact_type" => "artifact/json", "dataset_value" => "diagnostics_bundle"}],
      "outputs" => [%{"id" => "result", "artifact_type" => "artifact/json", "dataset_value" => "guard_result"}]
    }
  end

  defp report_node do
    %{
      "id" => "report",
      "kind" => "transform",
      "operator_id" => "transform.compose_diagnostics_report_payload",
      "config" => %{},
      "inputs" => [
        %{"id" => "bundle", "artifact_type" => "artifact/json", "dataset_value" => "diagnostics_bundle"},
        %{"id" => "guard", "artifact_type" => "artifact/json", "dataset_value" => "guard_result"}
      ],
      "outputs" => [%{"id" => "result", "artifact_type" => "artifact/json", "dataset_value" => "report_payload"}]
    }
  end

  defp export_node do
    %{
      "id" => "export",
      "kind" => "export",
      "operator_id" => "export.diagnostics_bundle_markdown",
      "config" => %{"title" => "Diagnostics Bundle Report"},
      "inputs" => [%{"id" => "bundle", "artifact_type" => "artifact/json", "dataset_value" => "report_payload"}],
      "outputs" => [%{"id" => "markdown", "artifact_type" => "export/markdown", "dataset_value" => "markdown_report"}]
    }
  end

  defp output_node do
    %{
      "id" => "markdown_output",
      "kind" => "output",
      "inputs" => [%{"id" => "markdown", "artifact_type" => "export/markdown", "dataset_value" => "markdown_report"}],
      "outputs" => []
    }
  end

  defp edge(id, from_node, from_port, to_node, to_port, artifact_type) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type
    }
  end

  defp edge(id, from_node, from_port, to_node, to_port),
    do: edge(id, from_node, from_port, to_node, to_port, "artifact/json")

  defp dataset_value(id, data_class, semantic_type, element_type \\ "json_object"),
    do: WorkflowCatalogSupport.workflow_dataset_value_info(id, data_class, semantic_type, element_type)

  defp entry_input(node_id, description) do
    %{
      "node_id" => node_id,
      "artifact_type" => "artifact/json",
      "description" => description
    }
  end
end
