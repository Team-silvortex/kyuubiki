defmodule KyuubikiSdk.WorkflowBuilders do
  @moduledoc "Builder helpers for workflow dataset and graph contracts."

  alias KyuubikiSdk.WorkflowContracts

  def schema_ref(schema, version) do
    %{"schema" => schema, "version" => version}
  end

  def axis(axis_id, attrs \\ %{}) do
    %{"id" => axis_id}
    |> maybe_put("label", Map.get(attrs, :label) || Map.get(attrs, "label"))
    |> maybe_put("size", Map.get(attrs, :size) || Map.get(attrs, "size"))
    |> maybe_put("semantic", Map.get(attrs, :semantic) || Map.get(attrs, "semantic"))
  end

  def shape(attrs \\ %{}) do
    %{}
    |> maybe_put("axes", Map.get(attrs, :axes) || Map.get(attrs, "axes"))
  end

  def dataset_value(value_id, data_class, element_type, attrs \\ %{}) do
    %{
      "id" => value_id,
      "data_class" => data_class,
      "element_type" => element_type,
      "shape" => Map.get(attrs, :shape) || Map.get(attrs, "shape") || %{}
    }
    |> maybe_put("semantic_type", Map.get(attrs, :semantic_type) || Map.get(attrs, "semantic_type"))
    |> maybe_put("unit", Map.get(attrs, :unit) || Map.get(attrs, "unit"))
    |> maybe_put("encoding", Map.get(attrs, :encoding) || Map.get(attrs, "encoding"))
    |> maybe_put("schema_ref", Map.get(attrs, :schema_ref) || Map.get(attrs, "schema_ref"))
  end

  def dataset_contract(contract_id, version, values, attrs \\ %{}) do
    contract =
      %{
        "schema_version" => WorkflowContracts.workflow_dataset_schema_version(),
        "id" => contract_id,
        "version" => version,
        "values" => values
      }
      |> maybe_put("name", Map.get(attrs, :name) || Map.get(attrs, "name"))
      |> maybe_put("description", Map.get(attrs, :description) || Map.get(attrs, "description"))
      |> maybe_put("metadata", Map.get(attrs, :metadata) || Map.get(attrs, "metadata"))

    if Keyword.get(List.wrap(attrs), :validate, true) == false or Map.get(attrs, :validate) == false do
      contract
    else
      case WorkflowContracts.validate_dataset_contract(contract) do
        {:ok, validated} -> validated
        {:error, error} -> raise error
      end
    end
  end

  def port(port_id, artifact_type, attrs \\ %{}) do
    %{"id" => port_id, "artifact_type" => artifact_type}
    |> maybe_put("name", Map.get(attrs, :name) || Map.get(attrs, "name"))
    |> maybe_put("required", Map.get(attrs, :required) || Map.get(attrs, "required"))
    |> maybe_put("cardinality", Map.get(attrs, :cardinality) || Map.get(attrs, "cardinality"))
    |> maybe_put("dataset_value", Map.get(attrs, :dataset_value) || Map.get(attrs, "dataset_value"))
  end

  def node(node_id, kind, attrs \\ %{}) do
    %{
      "id" => node_id,
      "kind" => kind,
      "inputs" => Map.get(attrs, :inputs) || Map.get(attrs, "inputs") || [],
      "outputs" => Map.get(attrs, :outputs) || Map.get(attrs, "outputs") || []
    }
    |> maybe_put("operator_id", Map.get(attrs, :operator_id) || Map.get(attrs, "operator_id"))
    |> maybe_put("name", Map.get(attrs, :name) || Map.get(attrs, "name"))
    |> maybe_put("description", Map.get(attrs, :description) || Map.get(attrs, "description"))
    |> maybe_put("config", Map.get(attrs, :config) || Map.get(attrs, "config"))
    |> maybe_put("cache_policy", Map.get(attrs, :cache_policy) || Map.get(attrs, "cache_policy"))
  end

  def edge(edge_id, from_node, from_port, to_node, to_port, artifact_type, attrs \\ %{}) do
    %{
      "id" => edge_id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type
    }
    |> maybe_put("dataset_value", Map.get(attrs, :dataset_value) || Map.get(attrs, "dataset_value"))
  end

  def graph(graph_id, name, version, entry_nodes, nodes, edges, attrs \\ %{}) do
    graph =
      %{
        "schema_version" => WorkflowContracts.workflow_graph_schema_version(),
        "id" => graph_id,
        "name" => name,
        "version" => version,
        "entry_nodes" => entry_nodes,
        "nodes" => nodes,
        "edges" => edges
      }
      |> maybe_put("description", Map.get(attrs, :description) || Map.get(attrs, "description"))
      |> maybe_put("dataset_contract", Map.get(attrs, :dataset_contract) || Map.get(attrs, "dataset_contract"))
      |> maybe_put("output_nodes", Map.get(attrs, :output_nodes) || Map.get(attrs, "output_nodes"))
      |> maybe_put("defaults", Map.get(attrs, :defaults) || Map.get(attrs, "defaults"))

    if Keyword.get(List.wrap(attrs), :validate, true) == false or Map.get(attrs, :validate) == false do
      graph
    else
      case WorkflowContracts.validate_graph(graph) do
        {:ok, validated} -> validated
        {:error, error} -> raise error
      end
    end
  end

  defp maybe_put(map, _key, nil), do: map
  defp maybe_put(map, _key, false), do: map
  defp maybe_put(map, key, value), do: Map.put(map, key, value)
end
