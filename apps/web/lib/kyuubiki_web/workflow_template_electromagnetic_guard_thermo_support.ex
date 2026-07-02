defmodule KyuubikiWeb.WorkflowTemplateElectromagneticGuardThermoSupport do
  @moduledoc false

  def input_node(id, artifact_type, dataset_value) do
    %{
      "id" => id,
      "kind" => "input",
      "outputs" => [port("model", artifact_type, dataset_value)]
    }
  end

  def solve_node(
        id,
        operator_id,
        input_artifact_type,
        output_artifact_type,
        input_dataset_value,
        output_dataset_value
      ) do
    %{
      "id" => id,
      "kind" => "solve",
      "operator_id" => operator_id,
      "inputs" => [port("model", input_artifact_type, input_dataset_value)],
      "outputs" => [port("result", output_artifact_type, output_dataset_value)]
    }
  end

  def condition_node(electrostatic_result_artifact_type, dataset_value) do
    %{
      "id" => "gate",
      "kind" => "condition",
      "config" => %{
        "predicate" => %{"path" => "max_electric_field", "operator" => "gt", "value" => 8.0}
      },
      "inputs" => [port("value", electrostatic_result_artifact_type, dataset_value)],
      "outputs" => [
        port("if_true", electrostatic_result_artifact_type, dataset_value),
        port("if_false", electrostatic_result_artifact_type, dataset_value)
      ]
    }
  end

  def hotspot_extract_node(
        electrostatic_result_artifact_type,
        input_dataset_value,
        output_dataset_value
      ) do
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
      "inputs" => [port("result", electrostatic_result_artifact_type, input_dataset_value)],
      "outputs" => [port("summary", "report/summary", output_dataset_value)]
    }
  end

  def bridge_node(
        id,
        operator_id,
        input_artifact_type,
        input_id,
        output_artifact_type,
        output_id,
        input_dataset_value,
        output_dataset_value,
        config
      ) do
    %{
      "id" => id,
      "kind" => "transform",
      "operator_id" => operator_id,
      "config" => config,
      "inputs" => [port(input_id, input_artifact_type, input_dataset_value)],
      "outputs" => [port(output_id, output_artifact_type, output_dataset_value)]
    }
  end

  def bridge_validator_node(
        id,
        operator_id,
        source_artifact_type,
        source_dataset_value,
        target_artifact_type,
        target_dataset_value,
        config
      ) do
    %{
      "id" => id,
      "kind" => "transform",
      "operator_id" => operator_id,
      "config" => config,
      "inputs" => [
        port(source_dataset_value, source_artifact_type, source_dataset_value),
        port(target_dataset_value, target_artifact_type, target_dataset_value)
      ],
      "outputs" => [port(target_dataset_value, target_artifact_type, target_dataset_value)]
    }
  end

  def thermo_summary_node(thermo_result_artifact_type, input_dataset_value, output_dataset_value) do
    %{
      "id" => "extract_thermo_summary",
      "kind" => "extract",
      "operator_id" => "extract.result_summary",
      "config" => %{"fields" => ["max_displacement", "max_stress", "max_temperature_delta"]},
      "inputs" => [port("result", thermo_result_artifact_type, input_dataset_value)],
      "outputs" => [port("summary", "report/summary", output_dataset_value)]
    }
  end

  def merge_node do
    %{
      "id" => "merge_summary",
      "kind" => "transform",
      "operator_id" => "transform.first_available",
      "inputs" => [
        port("left", "report/summary", "hotspot_summary"),
        port("right", "report/summary", "thermo_summary")
      ],
      "outputs" => [port("result", "report/summary", "merged_summary")]
    }
  end

  def export_json_node do
    %{
      "id" => "export_json",
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [port("summary", "report/summary", "merged_summary")],
      "outputs" => [port("json", "export/json", "summary_json")]
    }
  end

  def output_node(input_dataset_value) do
    %{
      "id" => "json_output",
      "kind" => "output",
      "inputs" => [port("json", "export/json", input_dataset_value)],
      "outputs" => []
    }
  end

  def edge(id, from_node, from_port, to_node, to_port, artifact_type, dataset_value) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type
    }
    |> maybe_put_dataset_value(dataset_value)
  end

  def electrostatic_heat_contract("bridge.electrostatic_field_to_heat_quad_2d") do
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

  def electrostatic_heat_contract("bridge.electrostatic_field_to_heat_triangle_2d") do
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

  def heat_quad_seed_model do
    %{
      "nodes" => [
        %{
          "id" => "h0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "h1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "h2",
          "x" => 1.0,
          "y" => 1.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "h3",
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

  def heat_triangle_seed_model do
    %{
      "nodes" => [
        %{
          "id" => "h0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "h1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_temperature" => true,
          "temperature" => 20.0,
          "heat_load" => 0.0
        },
        %{
          "id" => "h2",
          "x" => 0.0,
          "y" => 1.0,
          "fix_temperature" => false,
          "temperature" => 0.0,
          "heat_load" => 0.0
        }
      ],
      "elements" => [
        %{
          "id" => "ht0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "thickness" => 0.02,
          "conductivity" => 45.0
        }
      ]
    }
  end

  defp port(id, artifact_type, dataset_value) do
    %{"id" => id, "artifact_type" => artifact_type}
    |> maybe_put_dataset_value(dataset_value)
  end

  defp maybe_put_dataset_value(map, nil), do: map

  defp maybe_put_dataset_value(map, dataset_value),
    do: Map.put(map, "dataset_value", dataset_value)
end
