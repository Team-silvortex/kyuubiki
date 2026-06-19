defmodule KyuubikiWeb.WorkflowTemplateDiagnosticsEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def list do
    [
      diagnostics_bundle_guard_report_entry(),
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
      "graph" => build_coupled_diagnostics_graph(),
      "entry_inputs" => [
        %{
          "node_id" => "electrostatic_model",
          "artifact_type" => "study_model/electrostatic_plane_quad_2d",
          "description" => "Electrostatic plane quad study model used as the workflow entry artifact."
        }
      ],
      "output_artifacts" => [
        %{
          "node_id" => "markdown_output",
          "artifact_type" => "export/markdown",
          "description" => "Markdown diagnostics report for the electrostatic -> heat -> thermo chain."
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

  defp build_coupled_diagnostics_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.electrostatic-heat-thermo-diagnostics-markdown",
      "name" => "Electrostatic heat thermo diagnostics markdown",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.electrostatic_heat_thermo_diagnostics_markdown/v1",
          [
            dataset_value("electrostatic_model", "model", "study_model/electrostatic_plane_quad_2d"),
            dataset_value("electrostatic_result", "result", "result/electrostatic_plane_quad_2d"),
            dataset_value("heat_model", "model", "study_model/heat_plane_quad_2d"),
            dataset_value("heat_result", "result", "result/heat_plane_quad_2d"),
            dataset_value("thermo_model", "model", "study_model/thermal_plane_quad_2d"),
            dataset_value("thermo_result", "result", "result/thermal_plane_quad_2d"),
            dataset_value("electrostatic_diagnostics", "result", "artifact/json"),
            dataset_value("thermal_diagnostics", "result", "artifact/json"),
            dataset_value("thermo_diagnostics", "result", "artifact/json"),
            dataset_value("diagnostics_bundle", "result", "artifact/json"),
            dataset_value("guard_result", "result", "artifact/json"),
            dataset_value("report_payload", "result", "artifact/json"),
            dataset_value("markdown_report", "export", "export/markdown", "utf8_text")
          ],
          %{"workflow_family" => "electrostatic_heat_thermo_diagnostics_markdown"}
        ),
      "entry_nodes" => ["electrostatic_model"],
      "output_nodes" => ["markdown_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
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
        },
        %{
          "id" => "solve_electrostatic",
          "kind" => "solve",
          "operator_id" => "solve.electrostatic_plane_quad_2d",
          "inputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/electrostatic_plane_quad_2d",
              "dataset_value" => "electrostatic_model"
            }
          ],
          "outputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/electrostatic_plane_quad_2d",
              "dataset_value" => "electrostatic_result"
            }
          ]
        },
        %{
          "id" => "bridge_field_to_heat",
          "kind" => "transform",
          "operator_id" => "bridge.electrostatic_field_to_heat_quad_2d",
          "config" => %{
            "seed_model" => heat_quad_seed_model_example(),
            "contract" => WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(50.0)
          },
          "inputs" => [
            %{
              "id" => "electrostatic_result",
              "artifact_type" => "result/electrostatic_plane_quad_2d",
              "dataset_value" => "electrostatic_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "heat_model",
              "artifact_type" => "study_model/heat_plane_quad_2d",
              "dataset_value" => "heat_model"
            }
          ]
        },
        %{
          "id" => "solve_heat",
          "kind" => "solve",
          "operator_id" => "solve.heat_plane_quad_2d",
          "inputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/heat_plane_quad_2d",
              "dataset_value" => "heat_model"
            }
          ],
          "outputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/heat_plane_quad_2d",
              "dataset_value" => "heat_result"
            }
          ]
        },
        %{
          "id" => "bridge_temperature",
          "kind" => "transform",
          "operator_id" => "bridge.temperature_field_to_thermo_quad_2d",
          "config" => %{
            "seed_model" => WorkflowCatalogSupport.thermo_quad_seed_model_example(),
            "contract" => WorkflowCatalogSupport.heat_to_thermo_bridge_contract_example()
          },
          "inputs" => [
            %{
              "id" => "heat_result",
              "artifact_type" => "result/heat_plane_quad_2d",
              "dataset_value" => "heat_result"
            }
          ],
          "outputs" => [
            %{
              "id" => "thermo_model",
              "artifact_type" => "study_model/thermal_plane_quad_2d",
              "dataset_value" => "thermo_model"
            }
          ]
        },
        %{
          "id" => "solve_thermo",
          "kind" => "solve",
          "operator_id" => "solve.thermal_plane_quad_2d",
          "inputs" => [
            %{
              "id" => "model",
              "artifact_type" => "study_model/thermal_plane_quad_2d",
              "dataset_value" => "thermo_model"
            }
          ],
          "outputs" => [
            %{
              "id" => "result",
              "artifact_type" => "result/thermal_plane_quad_2d",
              "dataset_value" => "thermo_result"
            }
          ]
        },
        diagnostics_extract_node(
          "extract_electrostatic_diagnostics",
          "extract.electrostatic_result_diagnostics",
          "electrostatic_result",
          "result/electrostatic_plane_quad_2d",
          "electrostatic_diagnostics"
        ),
        diagnostics_extract_node(
          "extract_thermal_diagnostics",
          "extract.thermal_result_diagnostics",
          "heat_result",
          "result/heat_plane_quad_2d",
          "thermal_diagnostics"
        ),
        diagnostics_extract_node(
          "extract_thermo_diagnostics",
          "extract.thermo_result_diagnostics",
          "thermo_result",
          "result/thermal_plane_quad_2d",
          "thermo_diagnostics"
        ),
        bundle_node(),
        coupled_guard_node(),
        report_node(),
        export_node("Electrostatic Heat Thermo Diagnostics"),
        output_node()
      ],
      "edges" => [
        edge("e0", "electrostatic_model", "model", "solve_electrostatic", "model", "study_model/electrostatic_plane_quad_2d", "electrostatic_model"),
        edge("e1", "solve_electrostatic", "result", "bridge_field_to_heat", "electrostatic_result", "result/electrostatic_plane_quad_2d", "electrostatic_result"),
        edge("e2", "bridge_field_to_heat", "heat_model", "solve_heat", "model", "study_model/heat_plane_quad_2d", "heat_model"),
        edge("e3", "solve_heat", "result", "bridge_temperature", "heat_result", "result/heat_plane_quad_2d", "heat_result"),
        edge("e4", "bridge_temperature", "thermo_model", "solve_thermo", "model", "study_model/thermal_plane_quad_2d", "thermo_model"),
        edge("e5", "solve_thermo", "result", "extract_thermo_diagnostics", "result", "result/thermal_plane_quad_2d", "thermo_result"),
        edge("e6", "solve_electrostatic", "result", "extract_electrostatic_diagnostics", "result", "result/electrostatic_plane_quad_2d", "electrostatic_result"),
        edge("e7", "solve_heat", "result", "extract_thermal_diagnostics", "result", "result/heat_plane_quad_2d", "heat_result"),
        edge("e8", "extract_electrostatic_diagnostics", "summary", "bundle", "electrostatic", "artifact/json", "electrostatic_diagnostics"),
        edge("e9", "extract_thermal_diagnostics", "summary", "bundle", "thermal", "artifact/json", "thermal_diagnostics"),
        edge("e10", "extract_thermo_diagnostics", "summary", "bundle", "thermo", "artifact/json", "thermo_diagnostics"),
        edge("e11", "bundle", "result", "guard", "bundle", "artifact/json", "diagnostics_bundle"),
        edge("e12", "bundle", "result", "report", "bundle", "artifact/json", "diagnostics_bundle"),
        edge("e13", "guard", "result", "report", "guard", "artifact/json", "guard_result"),
        edge("e14", "report", "result", "export", "bundle", "artifact/json", "report_payload"),
        edge("e15", "export", "markdown", "markdown_output", "markdown", "export/markdown", "markdown_report")
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
          %{"source" => "thermo", "field" => "thermo_stress_peak", "comparison" => "gt", "threshold" => 180.0, "severity" => "block", "label" => "stress ceiling"},
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

  defp coupled_guard_node do
    %{
      "id" => "guard",
      "kind" => "transform",
      "operator_id" => "transform.evaluate_diagnostics_bundle_guard",
      "config" => %{
        "rules" => [
          %{"source" => "electrostatic", "field" => "electrostatic_field_peak_magnitude", "comparison" => "gt", "threshold" => 9.0, "severity" => "warn", "label" => "field ceiling"},
          %{"source" => "thermal", "field" => "thermal_temperature_max", "comparison" => "gt", "threshold" => 120.0, "severity" => "warn", "label" => "thermal temperature"},
          %{"source" => "thermo", "field" => "thermo_stress_peak", "comparison" => "gt", "threshold" => 180.0, "severity" => "block", "label" => "stress ceiling"}
        ]
      },
      "inputs" => [%{"id" => "bundle", "artifact_type" => "artifact/json", "dataset_value" => "diagnostics_bundle"}],
      "outputs" => [%{"id" => "result", "artifact_type" => "artifact/json", "dataset_value" => "guard_result"}]
    }
  end

  defp export_node(title \\ "Diagnostics Bundle Report") do
    %{
      "id" => "export",
      "kind" => "export",
      "operator_id" => "export.diagnostics_bundle_markdown",
      "config" => %{"title" => title},
      "inputs" => [%{"id" => "bundle", "artifact_type" => "artifact/json", "dataset_value" => "report_payload"}],
      "outputs" => [%{"id" => "markdown", "artifact_type" => "export/markdown", "dataset_value" => "markdown_report"}]
    }
  end

  defp diagnostics_extract_node(id, operator_id, input_dataset, artifact_type, output_dataset) do
    %{
      "id" => id,
      "kind" => "extract",
      "operator_id" => operator_id,
      "config" => %{},
      "inputs" => [%{"id" => "result", "artifact_type" => artifact_type, "dataset_value" => input_dataset}],
      "outputs" => [%{"id" => "summary", "artifact_type" => "artifact/json", "dataset_value" => output_dataset}]
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

  defp edge(id, from_node, from_port, to_node, to_port, artifact_type, dataset_value) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type,
      "dataset_value" => dataset_value
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

  defp heat_quad_seed_model_example do
    %{
      "nodes" => [
        %{"id" => "n0", "x" => 0.0, "y" => 0.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0},
        %{"id" => "n1", "x" => 1.0, "y" => 0.0, "fix_temperature" => false, "temperature" => 0.0, "heat_load" => 0.0},
        %{"id" => "n2", "x" => 1.0, "y" => 1.0, "fix_temperature" => false, "temperature" => 0.0, "heat_load" => 0.0},
        %{"id" => "n3", "x" => 0.0, "y" => 1.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0}
      ],
      "elements" => [
        %{"id" => "hq0", "node_i" => 0, "node_j" => 1, "node_k" => 2, "node_l" => 3, "thickness" => 0.02, "conductivity" => 45.0}
      ]
    }
  end

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
