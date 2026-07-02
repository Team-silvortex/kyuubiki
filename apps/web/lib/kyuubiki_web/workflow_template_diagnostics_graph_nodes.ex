defmodule KyuubikiWeb.WorkflowTemplateDiagnosticsGraphNodes do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def input_node(id, dataset_value, input_label) do
    %{
      "id" => id,
      "kind" => "input",
      "outputs" => [
        %{
          "id" => "summary",
          "artifact_type" => "artifact/json",
          "dataset_value" => input_label,
          "description" => dataset_value
        }
      ]
    }
  end

  def bundle_node do
    %{
      "id" => "bundle",
      "kind" => "transform",
      "operator_id" => "transform.compose_diagnostics_bundle",
      "config" => %{},
      "inputs" => [
        %{
          "id" => "electrostatic",
          "artifact_type" => "artifact/json",
          "dataset_value" => "electrostatic_diagnostics"
        },
        %{
          "id" => "thermal",
          "artifact_type" => "artifact/json",
          "dataset_value" => "thermal_diagnostics"
        },
        %{
          "id" => "thermo",
          "artifact_type" => "artifact/json",
          "dataset_value" => "thermo_diagnostics"
        }
      ],
      "outputs" => [
        %{
          "id" => "result",
          "artifact_type" => "artifact/json",
          "dataset_value" => "diagnostics_bundle"
        }
      ]
    }
  end

  def guard_node do
    diagnostics_guard_node([
      %{
        "source" => "thermal",
        "field" => "thermal_temperature_max",
        "threshold" => 120.0,
        "severity" => "warn",
        "label" => "thermal temperature"
      },
      %{
        "source" => "thermo",
        "field" => "thermo_stress_peak",
        "comparison" => "gt",
        "threshold" => 180.0,
        "severity" => "block",
        "label" => "stress ceiling"
      },
      %{
        "source" => "electrostatic",
        "field" => "electrostatic_field_peak_magnitude",
        "comparison" => "gt",
        "threshold" => 9.0,
        "severity" => "warn",
        "label" => "field ceiling"
      }
    ])
  end

  def coupled_guard_node do
    diagnostics_guard_node([
      %{
        "source" => "electrostatic",
        "field" => "electrostatic_field_peak_magnitude",
        "comparison" => "gt",
        "threshold" => 9.0,
        "severity" => "warn",
        "label" => "field ceiling"
      },
      %{
        "source" => "thermal",
        "field" => "thermal_temperature_max",
        "comparison" => "gt",
        "threshold" => 120.0,
        "severity" => "warn",
        "label" => "thermal temperature"
      },
      %{
        "source" => "thermo",
        "field" => "thermo_stress_peak",
        "comparison" => "gt",
        "threshold" => 180.0,
        "severity" => "block",
        "label" => "stress ceiling"
      }
    ])
  end

  def peak_guard_node do
    diagnostics_guard_node([
      %{
        "source" => "electrostatic",
        "field" => "electrostatic_field_peak_magnitude",
        "comparison" => "gt",
        "threshold" => 9.0,
        "severity" => "warn",
        "label" => "field ceiling"
      },
      %{
        "source" => "thermal",
        "field" => "thermal_flux_peak_magnitude",
        "comparison" => "gt",
        "threshold" => 25.0,
        "severity" => "warn",
        "label" => "thermal flux peak"
      },
      %{
        "source" => "thermo",
        "field" => "thermo_peak_stress",
        "comparison" => "gt",
        "threshold" => 180.0,
        "severity" => "block",
        "label" => "thermo stress peak"
      }
    ])
  end

  def report_node do
    %{
      "id" => "report",
      "kind" => "transform",
      "operator_id" => "transform.compose_diagnostics_report_payload",
      "config" => %{},
      "inputs" => [
        %{
          "id" => "bundle",
          "artifact_type" => "artifact/json",
          "dataset_value" => "diagnostics_bundle"
        },
        %{"id" => "guard", "artifact_type" => "artifact/json", "dataset_value" => "guard_result"}
      ],
      "outputs" => [
        %{
          "id" => "result",
          "artifact_type" => "artifact/json",
          "dataset_value" => "report_payload"
        }
      ]
    }
  end

  def export_node(title \\ "Diagnostics Bundle Report") do
    %{
      "id" => "export",
      "kind" => "export",
      "operator_id" => "export.diagnostics_bundle_markdown",
      "config" => %{"title" => title},
      "inputs" => [
        %{
          "id" => "bundle",
          "artifact_type" => "artifact/json",
          "dataset_value" => "report_payload"
        }
      ],
      "outputs" => [
        %{
          "id" => "markdown",
          "artifact_type" => "export/markdown",
          "dataset_value" => "markdown_report"
        }
      ]
    }
  end

  def diagnostics_extract_node(id, operator_id, input_dataset, artifact_type, output_dataset) do
    %{
      "id" => id,
      "kind" => "extract",
      "operator_id" => operator_id,
      "config" => %{},
      "inputs" => [
        %{"id" => "result", "artifact_type" => artifact_type, "dataset_value" => input_dataset}
      ],
      "outputs" => [
        %{
          "id" => "summary",
          "artifact_type" => "artifact/json",
          "dataset_value" => output_dataset
        }
      ]
    }
  end

  def output_node do
    %{
      "id" => "markdown_output",
      "kind" => "output",
      "inputs" => [
        %{
          "id" => "markdown",
          "artifact_type" => "export/markdown",
          "dataset_value" => "markdown_report"
        }
      ],
      "outputs" => []
    }
  end

  def edge(id, from_node, from_port, to_node, to_port, artifact_type, dataset_value) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type,
      "dataset_value" => dataset_value
    }
  end

  def edge(id, from_node, from_port, to_node, to_port, artifact_type) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type
    }
  end

  def edge(id, from_node, from_port, to_node, to_port),
    do: edge(id, from_node, from_port, to_node, to_port, "artifact/json")

  def heat_quad_seed_model_example do
    %{
      "nodes" => [
        %{
          "id" => "n0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n2",
          "x" => 1.0,
          "y" => 1.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "n3",
          "x" => 0.0,
          "y" => 1.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        }
      ],
      "elements" => [
        %{
          "id" => "hq0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "node_l" => 3,
          "thickness" => 0.02,
          "conductivity" => 45.0
        }
      ]
    }
  end

  def dataset_value(id, data_class, semantic_type, element_type \\ "json_object"),
    do:
      WorkflowCatalogSupport.workflow_dataset_value_info(
        id,
        data_class,
        semantic_type,
        element_type
      )

  defp diagnostics_guard_node(rules) do
    %{
      "id" => "guard",
      "kind" => "transform",
      "operator_id" => "transform.evaluate_diagnostics_bundle_guard",
      "config" => %{"rules" => rules},
      "inputs" => [
        %{
          "id" => "bundle",
          "artifact_type" => "artifact/json",
          "dataset_value" => "diagnostics_bundle"
        }
      ],
      "outputs" => [
        %{"id" => "result", "artifact_type" => "artifact/json", "dataset_value" => "guard_result"}
      ]
    }
  end
end
