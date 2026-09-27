defmodule KyuubikiWeb.WorkflowGraphPreflight do
  @moduledoc false
  alias KyuubikiWeb.{WorkflowGraphRecovery, WorkflowJsonBudget}

  @kinds ~w(input solve transform extract export condition output)
  @operator_kinds ~w(solve transform extract export)

  # This gate checks execution structure, not the operator implementation catalog.
  # Callback runtimes may provide operators independently of the built-in engine.
  def validate(graph, input_artifacts) do
    case check(graph, input_artifacts) do
      :ok -> WorkflowGraphRecovery.validate(graph["nodes"])
      {:error, reason} -> {:error, {:invalid_workflow_graph, reason}}
    end
  end

  defp check(graph, input_artifacts) when is_map(graph) and is_map(input_artifacts) do
    nodes = Map.get(graph, "nodes")
    edges = Map.get(graph, "edges", [])

    with :ok <- schema(graph["schema_version"]),
         :ok <- identifier(graph["id"], "workflow id", true),
         :ok <- bounded_list(nodes, "workflow nodes", 2048, false),
         :ok <- bounded_list(edges, "workflow edges", 4096),
         :ok <- WorkflowJsonBudget.validate(graph["dataset_contract"], "dataset_contract", 20_000),
         {:ok, index} <- index_nodes(nodes),
         :ok <- validate_edges(edges, index),
         :ok <- validate_references(graph, index, "entry_nodes", "input"),
         :ok <- validate_references(graph, index, "output_nodes", "output"),
         :ok <- acyclic(index, edges) do
      validate_artifacts(input_artifacts, index)
    end
  end

  defp check(_graph, _inputs), do: {:error, "graph and input_artifacts must be objects"}

  defp schema("kyuubiki.workflow-graph/v1"), do: :ok
  defp schema(_), do: {:error, "schema_version must be kyuubiki.workflow-graph/v1"}

  defp index_nodes(nodes) do
    Enum.reduce_while(nodes, {:ok, %{}}, fn node, {:ok, index} ->
      with {:ok, id, ports} <- validate_node(node),
           false <- Map.has_key?(index, id) do
        {:cont, {:ok, Map.put(index, id, ports)}}
      else
        true -> {:halt, {:error, "duplicate workflow node id #{node["id"]}"}}
        {:error, _} = error -> {:halt, error}
      end
    end)
  end

  defp validate_node(node) when is_map(node) do
    id = node["id"]
    kind = node["kind"]

    with :ok <- identifier(id, "workflow node id"),
         :ok <- node_kind(kind, id),
         :ok <- operator(node["operator_id"], kind, id),
         {:ok, inputs} <- ports(Map.get(node, "inputs", []), id, "inputs"),
         {:ok, outputs} <- ports(Map.get(node, "outputs", []), id, "outputs"),
         :ok <- WorkflowJsonBudget.validate(node["config"], "node #{id} config", 20_000) do
      {:ok, id, %{kind: kind, inputs: inputs, outputs: outputs}}
    end
  end

  defp validate_node(_node), do: {:error, "workflow node must be an object"}
  defp node_kind(kind, _id) when kind in @kinds, do: :ok
  defp node_kind(_kind, id), do: {:error, "workflow node #{id} has invalid kind"}

  defp operator(nil, kind, _id) when kind not in @operator_kinds, do: :ok
  defp operator(value, _kind, id), do: identifier(value, "node #{id} operator_id", true)

  defp ports(ports, id, direction) do
    with :ok <- bounded_list(ports, "node #{id} #{direction} ports", 32) do
      Enum.reduce_while(ports, {:ok, %{}}, fn port, {:ok, index} ->
        with {:ok, port_id, type} <- port(port, id),
             false <- Map.has_key?(index, port_id) do
          {:cont, {:ok, Map.put(index, port_id, type)}}
        else
          true -> {:halt, {:error, "node #{id} has duplicate #{direction} port #{port["id"]}"}}
          {:error, _} = error -> {:halt, error}
        end
      end)
    end
  end

  defp port(port, id) when is_map(port) do
    with :ok <- identifier(port["id"], "node #{id} port id"),
         :ok <- artifact_type(port["artifact_type"], "node #{id} port artifact_type") do
      {:ok, port["id"], port["artifact_type"]}
    end
  end

  defp port(_port, id), do: {:error, "node #{id} port must be an object"}

  defp validate_edges(edges, index) do
    Enum.reduce_while(edges, {:ok, MapSet.new(), MapSet.new()}, fn edge, {:ok, ids, targets} ->
      with {:ok, id, target} <- edge(edge, index),
           :ok <- unique(ids, id, "duplicate workflow edge id #{id}"),
           :ok <- unique(targets, target, "duplicate incoming edge for target port") do
        {:cont, {:ok, MapSet.put(ids, id), MapSet.put(targets, target)}}
      else
        {:error, _} = error -> {:halt, error}
      end
    end)
    |> case do
      {:ok, _, _} -> :ok
      error -> error
    end
  end

  defp edge(edge, index) when is_map(edge) do
    id = edge["id"]
    type = edge["artifact_type"]

    with :ok <- identifier(id, "workflow edge id", true),
         :ok <- artifact_type(type, "edge #{id} artifact_type"),
         :ok <- endpoint(edge["from"], index, :outputs, type, "edge #{id} source"),
         :ok <- endpoint(edge["to"], index, :inputs, type, "edge #{id} target") do
      {:ok, id, {edge["to"]["node"], edge["to"]["port"]}}
    end
  end

  defp edge(_edge, _index), do: {:error, "workflow edge must be an object"}

  defp endpoint(%{"node" => id, "port" => port}, index, direction, type, label) do
    case Map.get(index, id) do
      nil ->
        {:error, "#{label} node is not defined"}

      node ->
        case Map.fetch(node[direction], port) do
          :error -> {:error, "#{label} port is not defined"}
          {:ok, ^type} -> :ok
          {:ok, _} -> {:error, "#{label} port artifact_type does not match edge"}
        end
    end
  end

  defp endpoint(_ref, _index, _direction, _type, label),
    do: {:error, "#{label} must name a node and port"}

  defp validate_references(graph, index, key, kind) do
    refs = Map.get(graph, key, [])

    with :ok <- bounded_list(refs, key, 2048) do
      each(refs, fn id ->
        with :ok <- identifier(id, "#{key} id") do
          case Map.get(index, id) do
            %{kind: ^kind} -> :ok
            nil -> {:error, "#{key} node #{id} is not defined"}
            _ -> {:error, "#{key} node #{id} must be an #{kind} node"}
          end
        end
      end)
    end
  end

  defp acyclic(index, edges) do
    degrees = Map.new(index, fn {id, _} -> {id, 0} end)

    {degrees, outgoing} =
      Enum.reduce(edges, {degrees, %{}}, fn edge, {counts, outgoing} ->
        source = edge["from"]["node"]
        target = edge["to"]["node"]

        {Map.update!(counts, target, &(&1 + 1)),
         Map.update(outgoing, source, [target], &[target | &1])}
      end)

    ready = for {id, 0} <- degrees, do: id
    visited = visit_ready(ready, degrees, outgoing, 0)
    if visited == map_size(index), do: :ok, else: {:error, "workflow graph contains a cycle"}
  end

  defp visit_ready([], _degrees, _outgoing, visited), do: visited

  defp visit_ready([id | rest], degrees, outgoing, visited) do
    {degrees, ready} =
      Enum.reduce(Map.get(outgoing, id, []), {degrees, rest}, fn target, {counts, ready} ->
        count = Map.fetch!(counts, target) - 1
        {Map.put(counts, target, count), if(count == 0, do: [target | ready], else: ready)}
      end)

    visit_ready(ready, degrees, outgoing, visited + 1)
  end

  defp validate_artifacts(artifacts, index) do
    each(artifacts, fn {id, value} ->
      with :ok <- identifier(id, "input_artifacts key") do
        case Map.get(index, id) do
          %{kind: "input"} -> WorkflowJsonBudget.validate(value, "input_artifacts.#{id}", 500_000)
          _ -> {:error, "input_artifacts.#{id} must reference an input node"}
        end
      end
    end)
  end

  defp identifier(value, label, allow_dot \\ false) do
    pattern = if allow_dot, do: ~r/\A[A-Za-z0-9_.-]+\z/, else: ~r/\A[A-Za-z0-9_-]+\z/

    if is_binary(value) and byte_size(value) in 1..128 and Regex.match?(pattern, value),
      do: :ok,
      else: {:error, "#{label} must be a bounded ASCII identifier"}
  end

  defp artifact_type(value, label) do
    if is_binary(value) and byte_size(value) in 1..160 and
         Regex.match?(~r/\A[A-Za-z0-9_.\/-]+\z/, value),
       do: :ok,
       else: {:error, "#{label} must be a bounded artifact type"}
  end

  defp bounded_list(value, label, limit, allow_empty \\ true) do
    if value == [] and not allow_empty do
      {:error, "#{label} must contain at least one node"}
    else
      case list_budget(value, limit) do
        :ok -> :ok
        :too_long -> {:error, "#{label} exceeds security budget #{limit}"}
        :invalid -> {:error, "#{label} must be an array"}
      end
    end
  end

  defp list_budget([], _remaining), do: :ok
  defp list_budget([_ | _], 0), do: :too_long
  defp list_budget([_ | tail], remaining), do: list_budget(tail, remaining - 1)
  defp list_budget(_invalid, _remaining), do: :invalid

  defp unique(set, key, reason),
    do: if(MapSet.member?(set, key), do: {:error, reason}, else: :ok)

  defp each(values, check) do
    Enum.reduce_while(values, :ok, fn value, :ok ->
      case check.(value) do
        :ok -> {:cont, :ok}
        error -> {:halt, error}
      end
    end)
  end
end
