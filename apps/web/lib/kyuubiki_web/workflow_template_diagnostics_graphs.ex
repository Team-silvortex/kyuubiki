defmodule KyuubikiWeb.WorkflowTemplateDiagnosticsGraphs do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowTemplateDiagnosticsGraphNodes, as: Nodes

  def diagnostics_bundle_guard_report_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.diagnostics-bundle-guard-report-markdown",
      "name" => "Diagnostics bundle guard report markdown",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.diagnostics_bundle_guard_report/v1",
          [
            Nodes.dataset_value("electrostatic_diagnostics", "result", "artifact/json"),
            Nodes.dataset_value("thermal_diagnostics", "result", "artifact/json"),
            Nodes.dataset_value("thermo_diagnostics", "result", "artifact/json"),
            Nodes.dataset_value("diagnostics_bundle", "result", "artifact/json"),
            Nodes.dataset_value("guard_result", "result", "artifact/json"),
            Nodes.dataset_value("report_payload", "result", "artifact/json"),
            Nodes.dataset_value("markdown_report", "export", "export/markdown", "utf8_text")
          ],
          %{"workflow_family" => "diagnostics_bundle_guard_report"}
        ),
      "entry_nodes" => ["electrostatic_input", "thermal_input", "thermo_input"],
      "output_nodes" => ["markdown_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        Nodes.input_node("electrostatic_input", "electrostatic", "electrostatic_diagnostics"),
        Nodes.input_node("thermal_input", "thermal", "thermal_diagnostics"),
        Nodes.input_node("thermo_input", "thermo", "thermo_diagnostics"),
        Nodes.bundle_node(),
        Nodes.guard_node(),
        Nodes.report_node(),
        Nodes.export_node(),
        Nodes.output_node()
      ],
      "edges" => [
        Nodes.edge("e0", "electrostatic_input", "summary", "bundle", "electrostatic"),
        Nodes.edge("e1", "thermal_input", "summary", "bundle", "thermal"),
        Nodes.edge("e2", "thermo_input", "summary", "bundle", "thermo"),
        Nodes.edge("e3", "bundle", "result", "guard", "bundle"),
        Nodes.edge("e4", "bundle", "result", "report", "bundle"),
        Nodes.edge("e5", "guard", "result", "report", "guard"),
        Nodes.edge("e6", "report", "result", "export", "bundle"),
        Nodes.edge("e7", "export", "markdown", "markdown_output", "markdown", "export/markdown")
      ]
    }
  end

  def peak_diagnostics_bundle_report_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.peak-diagnostics-bundle-report-markdown",
      "name" => "Peak diagnostics bundle report markdown",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.peak_diagnostics_bundle_guard_report/v1",
          [
            Nodes.dataset_value("electrostatic_diagnostics", "result", "artifact/json"),
            Nodes.dataset_value("thermal_diagnostics", "result", "artifact/json"),
            Nodes.dataset_value("thermo_diagnostics", "result", "artifact/json"),
            Nodes.dataset_value("diagnostics_bundle", "result", "artifact/json"),
            Nodes.dataset_value("guard_result", "result", "artifact/json"),
            Nodes.dataset_value("report_payload", "result", "artifact/json"),
            Nodes.dataset_value("markdown_report", "export", "export/markdown", "utf8_text")
          ],
          %{"workflow_family" => "peak_diagnostics_bundle_guard_report"}
        ),
      "entry_nodes" => ["electrostatic_input", "thermal_input", "thermo_input"],
      "output_nodes" => ["markdown_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        Nodes.input_node("electrostatic_input", "electrostatic", "electrostatic_diagnostics"),
        Nodes.input_node("thermal_input", "thermal", "thermal_diagnostics"),
        Nodes.input_node("thermo_input", "thermo", "thermo_diagnostics"),
        Nodes.bundle_node(),
        Nodes.peak_guard_node(),
        Nodes.report_node(),
        Nodes.export_node("Peak Diagnostics Bundle Report"),
        Nodes.output_node()
      ],
      "edges" => [
        Nodes.edge("e0", "electrostatic_input", "summary", "bundle", "electrostatic"),
        Nodes.edge("e1", "thermal_input", "summary", "bundle", "thermal"),
        Nodes.edge("e2", "thermo_input", "summary", "bundle", "thermo"),
        Nodes.edge("e3", "bundle", "result", "guard", "bundle"),
        Nodes.edge("e4", "bundle", "result", "report", "bundle"),
        Nodes.edge("e5", "guard", "result", "report", "guard"),
        Nodes.edge("e6", "report", "result", "export", "bundle"),
        Nodes.edge("e7", "export", "markdown", "markdown_output", "markdown", "export/markdown")
      ]
    }
  end

  def electrostatic_heat_thermo_diagnostics_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.electrostatic-heat-thermo-diagnostics-markdown",
      "name" => "Electrostatic heat thermo diagnostics markdown",
      "version" => "1.0.0",
      "dataset_contract" => coupled_dataset_contract(),
      "entry_nodes" => ["electrostatic_model"],
      "output_nodes" => ["markdown_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => coupled_nodes(),
      "edges" => coupled_edges()
    }
  end

  defp coupled_dataset_contract do
    WorkflowCatalogSupport.workflow_dataset_contract(
      "kyuubiki.dataset.electrostatic_heat_thermo_diagnostics_markdown/v1",
      [
        Nodes.dataset_value(
          "electrostatic_model",
          "model",
          "study_model/electrostatic_plane_quad_2d"
        ),
        Nodes.dataset_value(
          "electrostatic_result",
          "result",
          "result/electrostatic_plane_quad_2d"
        ),
        Nodes.dataset_value("heat_model", "model", "study_model/heat_plane_quad_2d"),
        Nodes.dataset_value("heat_result", "result", "result/heat_plane_quad_2d"),
        Nodes.dataset_value("thermo_model", "model", "study_model/thermal_plane_quad_2d"),
        Nodes.dataset_value("thermo_result", "result", "result/thermal_plane_quad_2d"),
        Nodes.dataset_value("electrostatic_diagnostics", "result", "artifact/json"),
        Nodes.dataset_value("thermal_diagnostics", "result", "artifact/json"),
        Nodes.dataset_value("thermo_diagnostics", "result", "artifact/json"),
        Nodes.dataset_value("diagnostics_bundle", "result", "artifact/json"),
        Nodes.dataset_value("guard_result", "result", "artifact/json"),
        Nodes.dataset_value("report_payload", "result", "artifact/json"),
        Nodes.dataset_value("markdown_report", "export", "export/markdown", "utf8_text")
      ],
      %{"workflow_family" => "electrostatic_heat_thermo_diagnostics_markdown"}
    )
  end

  defp coupled_nodes do
    [
      input_model_node(),
      solve_node("solve_electrostatic", "solve.electrostatic_plane_quad_2d", "electrostatic"),
      bridge_field_to_heat_node(),
      solve_node("solve_heat", "solve.heat_plane_quad_2d", "heat"),
      bridge_temperature_node(),
      solve_node("solve_thermo", "solve.thermal_plane_quad_2d", "thermo"),
      Nodes.diagnostics_extract_node(
        "extract_electrostatic_diagnostics",
        "extract.electrostatic_result_diagnostics",
        "electrostatic_result",
        "result/electrostatic_plane_quad_2d",
        "electrostatic_diagnostics"
      ),
      Nodes.diagnostics_extract_node(
        "extract_thermal_diagnostics",
        "extract.thermal_result_diagnostics",
        "heat_result",
        "result/heat_plane_quad_2d",
        "thermal_diagnostics"
      ),
      Nodes.diagnostics_extract_node(
        "extract_thermo_diagnostics",
        "extract.thermo_result_diagnostics",
        "thermo_result",
        "result/thermal_plane_quad_2d",
        "thermo_diagnostics"
      ),
      Nodes.bundle_node(),
      Nodes.coupled_guard_node(),
      Nodes.report_node(),
      Nodes.export_node("Electrostatic Heat Thermo Diagnostics"),
      Nodes.output_node()
    ]
  end

  defp coupled_edges do
    [
      Nodes.edge(
        "e0",
        "electrostatic_model",
        "model",
        "solve_electrostatic",
        "model",
        "study_model/electrostatic_plane_quad_2d",
        "electrostatic_model"
      ),
      Nodes.edge(
        "e1",
        "solve_electrostatic",
        "result",
        "bridge_field_to_heat",
        "electrostatic_result",
        "result/electrostatic_plane_quad_2d",
        "electrostatic_result"
      ),
      Nodes.edge(
        "e2",
        "bridge_field_to_heat",
        "heat_model",
        "solve_heat",
        "model",
        "study_model/heat_plane_quad_2d",
        "heat_model"
      ),
      Nodes.edge(
        "e3",
        "solve_heat",
        "result",
        "bridge_temperature",
        "heat_result",
        "result/heat_plane_quad_2d",
        "heat_result"
      ),
      Nodes.edge(
        "e4",
        "bridge_temperature",
        "thermo_model",
        "solve_thermo",
        "model",
        "study_model/thermal_plane_quad_2d",
        "thermo_model"
      ),
      Nodes.edge(
        "e5",
        "solve_thermo",
        "result",
        "extract_thermo_diagnostics",
        "result",
        "result/thermal_plane_quad_2d",
        "thermo_result"
      ),
      Nodes.edge(
        "e6",
        "solve_electrostatic",
        "result",
        "extract_electrostatic_diagnostics",
        "result",
        "result/electrostatic_plane_quad_2d",
        "electrostatic_result"
      ),
      Nodes.edge(
        "e7",
        "solve_heat",
        "result",
        "extract_thermal_diagnostics",
        "result",
        "result/heat_plane_quad_2d",
        "heat_result"
      ),
      Nodes.edge(
        "e8",
        "extract_electrostatic_diagnostics",
        "summary",
        "bundle",
        "electrostatic",
        "artifact/json",
        "electrostatic_diagnostics"
      ),
      Nodes.edge(
        "e9",
        "extract_thermal_diagnostics",
        "summary",
        "bundle",
        "thermal",
        "artifact/json",
        "thermal_diagnostics"
      ),
      Nodes.edge(
        "e10",
        "extract_thermo_diagnostics",
        "summary",
        "bundle",
        "thermo",
        "artifact/json",
        "thermo_diagnostics"
      ),
      Nodes.edge(
        "e11",
        "bundle",
        "result",
        "guard",
        "bundle",
        "artifact/json",
        "diagnostics_bundle"
      ),
      Nodes.edge(
        "e12",
        "bundle",
        "result",
        "report",
        "bundle",
        "artifact/json",
        "diagnostics_bundle"
      ),
      Nodes.edge("e13", "guard", "result", "report", "guard", "artifact/json", "guard_result"),
      Nodes.edge(
        "e14",
        "report",
        "result",
        "export",
        "bundle",
        "artifact/json",
        "report_payload"
      ),
      Nodes.edge(
        "e15",
        "export",
        "markdown",
        "markdown_output",
        "markdown",
        "export/markdown",
        "markdown_report"
      )
    ]
  end

  defp input_model_node do
    %{
      "id" => "electrostatic_model",
      "kind" => "input",
      "outputs" => [
        %{
          "id" => "model",
          "artifact_type" => "study_model/electrostatic_plane_quad_2d",
          "dataset_value" => "electrostatic_model"
        }
      ]
    }
  end

  defp solve_node(id, operator_id, domain) do
    %{
      "id" => id,
      "kind" => "solve",
      "operator_id" => operator_id,
      "inputs" => [model_port("#{domain}_model", "study_model/#{model_type(domain)}")],
      "outputs" => [result_port("#{domain}_result", "result/#{result_type(domain)}")]
    }
  end

  defp bridge_field_to_heat_node do
    bridge_node(
      "bridge_field_to_heat",
      "bridge.electrostatic_field_to_heat_quad_2d",
      "electrostatic_result",
      "result/electrostatic_plane_quad_2d",
      "heat_model",
      "study_model/heat_plane_quad_2d",
      %{
        "seed_model" => Nodes.heat_quad_seed_model_example(),
        "contract" => WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(50.0)
      }
    )
  end

  defp bridge_temperature_node do
    bridge_node(
      "bridge_temperature",
      "bridge.temperature_field_to_thermo_quad_2d",
      "heat_result",
      "result/heat_plane_quad_2d",
      "thermo_model",
      "study_model/thermal_plane_quad_2d",
      %{
        "seed_model" => WorkflowCatalogSupport.thermo_quad_seed_model_example(),
        "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
      }
    )
  end

  defp bridge_node(
         id,
         operator_id,
         input_dataset,
         input_type,
         output_dataset,
         output_type,
         config
       ) do
    %{
      "id" => id,
      "kind" => "transform",
      "operator_id" => operator_id,
      "config" => config,
      "inputs" => [
        %{"id" => input_dataset, "artifact_type" => input_type, "dataset_value" => input_dataset}
      ],
      "outputs" => [
        %{
          "id" => output_dataset,
          "artifact_type" => output_type,
          "dataset_value" => output_dataset
        }
      ]
    }
  end

  defp model_port(dataset, type),
    do: %{"id" => "model", "artifact_type" => type, "dataset_value" => dataset}

  defp result_port(dataset, type),
    do: %{"id" => "result", "artifact_type" => type, "dataset_value" => dataset}

  defp model_type("electrostatic"), do: "electrostatic_plane_quad_2d"
  defp model_type("heat"), do: "heat_plane_quad_2d"
  defp model_type("thermo"), do: "thermal_plane_quad_2d"

  defp result_type("electrostatic"), do: "electrostatic_plane_quad_2d"
  defp result_type("heat"), do: "heat_plane_quad_2d"
  defp result_type("thermo"), do: "thermal_plane_quad_2d"
end
