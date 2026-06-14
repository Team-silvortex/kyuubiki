defmodule KyuubikiWeb.WorkflowCatalogSupport do
  @moduledoc false

  def workflow_dataset_contract(id, values, metadata \\ %{}) when is_list(values) do
    %{
      "id" => id,
      "version" => "1.0.0",
      "values" => values,
      "metadata" => metadata
    }
  end

  def workflow_dataset_value_info(id, data_class, semantic_type, element_type \\ "json_object") do
    %{
      "id" => id,
      "data_class" => data_class,
      "element_type" => element_type,
      "shape" => %{"axes" => []},
      "semantic_type" => semantic_type,
      "unit" => nil,
      "encoding" => nil,
      "schema_ref" => nil
    }
  end

  def derive_dataset_lineage(graph, artifact_lineage)
      when is_map(graph) and is_list(artifact_lineage) do
    dataset_contract =
      case Map.get(graph, "dataset_contract") do
        value when is_map(value) -> value
        _ -> %{}
      end

    dataset_info_by_value = dataset_info_by_value(dataset_contract)
    port_info = build_port_info(graph)
    artifact_dataset_map = build_artifact_dataset_map(artifact_lineage, port_info)

    Enum.reduce(artifact_lineage, [], fn entry, acc ->
      artifact_key = Map.get(entry, "artifact_key")

      case Map.get(artifact_dataset_map, artifact_key) do
        dataset_value when is_binary(dataset_value) ->
          info = Map.get(dataset_info_by_value, dataset_value, %{})

          acc ++
            [
              %{
                "artifact_key" => artifact_key,
                "node_id" => Map.get(entry, "node_id"),
                "port_id" => Map.get(entry, "port_id"),
                "artifact_type" => port_artifact_type(port_info, entry),
                "dataset_value" => dataset_value,
                "contract_id" => Map.get(dataset_contract, "id"),
                "dataset_role" => Map.get(info, "data_class"),
                "dataset_format" => Map.get(info, "element_type"),
                "semantic_type" => Map.get(info, "semantic_type"),
                "source_datasets" => source_datasets(entry, artifact_dataset_map)
              }
            ]

        _ ->
          acc
      end
    end)
  end

  def derive_dataset_lineage(_graph, _artifact_lineage), do: []

  def electrostatic_to_heat_bridge_contract_example(
        scale,
        node_index_fields \\ ["node_i", "node_j", "node_k", "node_l"]
      ) do
    %{
      "version" => "kyuubiki.bridge-contract/v1",
      "source" => %{
        "field" => "electric_field_magnitude",
        "distribution" => "element_to_nodes",
        "node_index_fields" => node_index_fields
      },
      "transform" => %{
        "scale" => scale,
        "reduction" => "mean",
        "default_value" => 0.0
      },
      "target" => %{"field" => "heat_load"}
    }
  end

  def heat_to_thermo_bridge_contract_example do
    %{
      "version" => "kyuubiki.bridge-contract/v1",
      "source" => %{"field" => "temperature"},
      "transform" => %{"scale" => 1.0, "default_value" => 0.0},
      "target" => %{"field" => "temperature_delta"}
    }
  end

  def thermo_quad_seed_model_example do
    %{
      "nodes" => [
        %{
          "id" => "h0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_x" => true,
          "fix_y" => true,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "h1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_x" => false,
          "fix_y" => false,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "h2",
          "x" => 1.0,
          "y" => 1.0,
          "fix_x" => false,
          "fix_y" => false,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "h3",
          "x" => 0.0,
          "y" => 1.0,
          "fix_x" => true,
          "fix_y" => true,
          "temperature_delta" => 0.0
        }
      ],
      "elements" => [
        %{
          "id" => "tq0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "node_l" => 3,
          "thickness" => 0.02,
          "youngs_modulus" => 210.0e9,
          "poisson_ratio" => 0.3,
          "thermal_expansion" => 11.0e-6
        }
      ]
    }
  end

  def thermo_triangle_seed_model_example do
    %{
      "nodes" => [
        %{
          "id" => "t0",
          "x" => 0.0,
          "y" => 0.0,
          "fix_x" => true,
          "fix_y" => true,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "t1",
          "x" => 1.0,
          "y" => 0.0,
          "fix_x" => false,
          "fix_y" => false,
          "temperature_delta" => 0.0
        },
        %{
          "id" => "t2",
          "x" => 0.0,
          "y" => 1.0,
          "fix_x" => true,
          "fix_y" => true,
          "temperature_delta" => 0.0
        }
      ],
      "elements" => [
        %{
          "id" => "tt0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "thickness" => 0.02,
          "youngs_modulus" => 210.0e9,
          "poisson_ratio" => 0.3,
          "thermal_expansion" => 11.0e-6
        }
      ]
    }
  end

  defp dataset_info_by_value(%{"values" => values}) when is_list(values) do
    Enum.reduce(values, %{}, fn value, acc ->
      case Map.get(value, "id") do
        dataset_value when is_binary(dataset_value) -> Map.put(acc, dataset_value, value)
        _ -> acc
      end
    end)
  end

  defp dataset_info_by_value(_dataset_contract), do: %{}

  defp build_port_info(graph) do
    Enum.reduce(Map.get(graph, "nodes", []), %{}, fn node, acc ->
      node_id = Map.get(node, "id")

      acc
      |> merge_ports(node_id, Map.get(node, "inputs", []))
      |> merge_ports(node_id, Map.get(node, "outputs", []))
    end)
  end

  defp merge_ports(acc, node_id, ports) when is_binary(node_id) and is_list(ports) do
    Enum.reduce(ports, acc, fn port, port_acc ->
      case Map.get(port, "id") do
        port_id when is_binary(port_id) ->
          Map.put(port_acc, {node_id, port_id}, %{
            "dataset_value" => Map.get(port, "dataset_value"),
            "artifact_type" => Map.get(port, "artifact_type")
          })

        _ ->
          port_acc
      end
    end)
  end

  defp merge_ports(acc, _node_id, _ports), do: acc

  defp build_artifact_dataset_map(artifact_lineage, port_info) do
    Enum.reduce(artifact_lineage, %{}, fn entry, acc ->
      dataset_value =
        port_info
        |> Map.get({Map.get(entry, "node_id"), Map.get(entry, "port_id")}, %{})
        |> Map.get("dataset_value")

      if is_binary(dataset_value) do
        Map.put(acc, Map.get(entry, "artifact_key"), dataset_value)
      else
        acc
      end
    end)
  end

  defp source_datasets(entry, artifact_dataset_map) do
    entry
    |> Map.get("source_artifacts", [])
    |> Enum.map(&Map.get(artifact_dataset_map, &1))
    |> Enum.filter(&is_binary/1)
    |> Enum.uniq()
  end

  defp port_artifact_type(port_info, entry) do
    port_info
    |> Map.get({Map.get(entry, "node_id"), Map.get(entry, "port_id")}, %{})
    |> Map.get("artifact_type")
  end
end
