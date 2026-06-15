defmodule KyuubikiSdk.WorkflowBuilders do
  @moduledoc "Builder helpers for workflow dataset and graph contracts."

  alias KyuubikiSdk.WorkflowContracts

  def schema_ref(schema, version) do
    %{"schema" => schema, "version" => version}
  end

  def axis(axis_id, attrs \\ %{}) do
    %{"id" => axis_id}
    |> maybe_put("label", attr(attrs, :label))
    |> maybe_put("size", attr(attrs, :size))
    |> maybe_put("semantic", attr(attrs, :semantic))
  end

  def shape(attrs \\ %{}) do
    %{}
    |> maybe_put("axes", attr(attrs, :axes))
  end

  def dataset_value(value_id, data_class, element_type, attrs \\ %{}) do
    %{
      "id" => value_id,
      "data_class" => data_class,
      "element_type" => element_type,
      "shape" => attr(attrs, :shape) || %{}
    }
    |> maybe_put("semantic_type", attr(attrs, :semantic_type))
    |> maybe_put("unit", attr(attrs, :unit))
    |> maybe_put("encoding", attr(attrs, :encoding))
    |> maybe_put("schema_ref", attr(attrs, :schema_ref))
  end

  def dataset_contract(contract_id, version, values, attrs \\ %{}) do
    contract =
      %{
        "schema_version" => WorkflowContracts.workflow_dataset_schema_version(),
        "id" => contract_id,
        "version" => version,
        "values" => values
      }
      |> maybe_put("name", attr(attrs, :name))
      |> maybe_put("description", attr(attrs, :description))
      |> maybe_put("metadata", attr(attrs, :metadata))

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
    |> maybe_put("name", attr(attrs, :name))
    |> maybe_put("required", attr(attrs, :required))
    |> maybe_put("cardinality", attr(attrs, :cardinality))
    |> maybe_put("dataset_value", attr(attrs, :dataset_value))
  end

  def defaults(attrs \\ %{}) do
    %{}
    |> maybe_put("cache_policy", attr(attrs, :cache_policy))
    |> maybe_put("orchestrated", attr(attrs, :orchestrated))
    |> maybe_put("dispatch_policy", attr(attrs, :dispatch_policy))
    |> maybe_put("placement_tags", attr(attrs, :placement_tags))
    |> maybe_put("required_capabilities", attr(attrs, :required_capabilities))
  end

  def operator_fetch_entry(node_id, operator_id, attrs \\ %{}) do
    %{"node_id" => node_id, "operator_id" => operator_id}
    |> maybe_put("package_ref", attr(attrs, :package_ref))
    |> maybe_put("version", attr(attrs, :version))
    |> maybe_put("integrity", attr(attrs, :integrity))
    |> maybe_put("cache_scope", attr(attrs, :cache_scope))
  end

  def node(node_id, kind, attrs \\ %{}) do
    %{
      "id" => node_id,
      "kind" => kind,
      "inputs" => attr(attrs, :inputs) || [],
      "outputs" => attr(attrs, :outputs) || []
    }
    |> maybe_put("operator_id", attr(attrs, :operator_id))
    |> maybe_put("name", attr(attrs, :name))
    |> maybe_put("description", attr(attrs, :description))
    |> maybe_put("config", attr(attrs, :config))
    |> maybe_put("cache_policy", attr(attrs, :cache_policy))
    |> maybe_put("placement_tags", attr(attrs, :placement_tags))
    |> maybe_put("required_capabilities", attr(attrs, :required_capabilities))
  end

  def edge(edge_id, from_node, from_port, to_node, to_port, artifact_type, attrs \\ %{}) do
    %{
      "id" => edge_id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type
    }
    |> maybe_put("dataset_value", attr(attrs, :dataset_value))
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
      |> maybe_put("description", attr(attrs, :description))
      |> maybe_put("dataset_contract", attr(attrs, :dataset_contract))
      |> maybe_put("output_nodes", attr(attrs, :output_nodes))
      |> maybe_put("defaults", attr(attrs, :defaults))
      |> maybe_put("dispatch_policy", attr(attrs, :dispatch_policy))
      |> maybe_put("operator_fetch_plan", attr(attrs, :operator_fetch_plan))
      |> maybe_put("placement_tags", attr(attrs, :placement_tags))
      |> maybe_put("required_capabilities", attr(attrs, :required_capabilities))

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
  defp maybe_put(map, key, value), do: Map.put(map, key, value)
  defp attr(attrs, key), do: Map.get(attrs, key, Map.get(attrs, Atom.to_string(key)))
end
