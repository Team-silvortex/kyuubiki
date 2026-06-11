defmodule KyuubikiSdk.WorkflowContracts do
  @moduledoc "Typed workflow dataset and graph contract helpers with validation."

  alias KyuubikiSdk.Error

  @workflow_dataset_schema_version "kyuubiki.workflow-dataset/v1"
  @workflow_graph_schema_version "kyuubiki.workflow-graph/v1"
  @data_classes ~w(study_model result field table report export scalar metadata)
  @encodings ~w(json json_lines f64_le f32_le i64_le i32_le u8)
  @node_kinds ~w(input solve transform extract export condition output)
  @cardinalities ~w(one many)
  @operator_node_kinds ~w(solve transform extract export condition)

  def workflow_dataset_schema_version, do: @workflow_dataset_schema_version
  def workflow_graph_schema_version, do: @workflow_graph_schema_version

  def validate_dataset_contract(contract) when is_map(contract) do
    errors = validate_dataset_contract_errors(contract, "dataset")
    if errors == [], do: {:ok, contract}, else: {:error, Error.validation(errors)}
  end

  def validate_dataset_contract(_contract), do: {:error, Error.validation(["dataset must be an object"])}

  def validate_graph(graph) when is_map(graph) do
    errors = validate_graph_errors(graph)
    if errors == [], do: {:ok, graph}, else: {:error, Error.validation(errors)}
  end

  def validate_graph(_graph), do: {:error, Error.validation(["graph must be an object"])}

  defp validate_graph_errors(graph) do
    dataset_ids =
      case Map.get(graph, "dataset_contract") do
        contract when is_map(contract) ->
          case validate_dataset_contract(contract) do
            {:ok, _} -> MapSet.new(Enum.map(Map.get(contract, "values", []), &Map.get(&1, "id")))
            {:error, %Error{message: message}} -> message_to_lines(message, "graph.dataset_contract")
          end

        nil ->
          MapSet.new()

        _ ->
          ["graph.dataset_contract must be an object"]
      end

    base_errors =
      []
      |> require_string(graph, "schema_version", "graph")
      |> require_schema_version(Map.get(graph, "schema_version"), @workflow_graph_schema_version, "graph")
      |> require_string(graph, "id", "graph")
      |> require_string(graph, "name", "graph")
      |> require_string(graph, "version", "graph")
      |> optional_string(graph, "description", "graph")
      |> require_list(graph, "entry_nodes", "graph", 1)
      |> optional_list(graph, "output_nodes", "graph")
      |> require_list(graph, "nodes", "graph", 1)
      |> require_list(graph, "edges", "graph", 0)
      |> validate_defaults(Map.get(graph, "defaults"))

    {node_errors, node_ids, input_ports, output_ports} = validate_nodes(Map.get(graph, "nodes", []), dataset_ids)
    entry_errors = validate_named_node_refs(Map.get(graph, "entry_nodes", []), node_ids, "graph.entry_nodes")
    output_errors = validate_named_node_refs(Map.get(graph, "output_nodes", []), node_ids, "graph.output_nodes")
    edge_errors = validate_edges(Map.get(graph, "edges", []), dataset_ids, input_ports, output_ports)

    List.flatten([base_errors, normalize_nested_errors(dataset_ids), node_errors, entry_errors, output_errors, edge_errors])
  end

  defp validate_nodes(nodes, dataset_ids) when is_list(nodes) do
    Enum.with_index(nodes)
    |> Enum.reduce({[], MapSet.new(), %{}, %{}}, fn {node, index}, {errors, node_ids, input_ports, output_ports} ->
      path = "graph.nodes[#{index}]"

      if is_map(node) do
        current_errors =
          []
          |> require_string(node, "id", path)
          |> require_string(node, "kind", path)
          |> validate_enum(Map.get(node, "kind"), @node_kinds, "#{path}.kind")
          |> optional_string(node, "operator_id", path)
          |> optional_string(node, "name", path)
          |> optional_string(node, "description", path)
          |> validate_node_config(Map.get(node, "config"), "#{path}.config")
          |> validate_cache_policy(Map.get(node, "cache_policy"), "#{path}.cache_policy")
          |> require_list(node, "inputs", path, 0)
          |> require_list(node, "outputs", path, 0)
          |> require_operator_id(node, path)

        node_id = Map.get(node, "id")
        duplicate_errors = if present_string?(node_id) and MapSet.member?(node_ids, node_id), do: ["graph.nodes contains duplicate id #{inspect(node_id)}"], else: []
        next_node_ids = if present_string?(node_id), do: MapSet.put(node_ids, node_id), else: node_ids
        {input_errors, next_input_ports} = collect_ports(node_id, Map.get(node, "inputs", []), dataset_ids, "#{path}.inputs")
        {output_errors, next_output_ports} = collect_ports(node_id, Map.get(node, "outputs", []), dataset_ids, "#{path}.outputs")

        {
          errors ++ current_errors ++ duplicate_errors ++ input_errors ++ output_errors,
          next_node_ids,
          Map.merge(input_ports, next_input_ports),
          Map.merge(output_ports, next_output_ports)
        }
      else
        {errors ++ ["#{path} must be an object"], node_ids, input_ports, output_ports}
      end
    end)
  end

  defp collect_ports(node_id, ports, dataset_ids, path) when is_list(ports) do
    Enum.with_index(ports)
    |> Enum.reduce({[], %{}, MapSet.new()}, fn {port, index}, {errors, bucket, port_ids} ->
      port_path = "#{path}[#{index}]"

      if is_map(port) do
        current_errors =
          []
          |> require_string(port, "id", port_path)
          |> require_string(port, "artifact_type", port_path)
          |> optional_string(port, "name", port_path)
          |> validate_boolean(Map.get(port, "required"), "#{port_path}.required")
          |> validate_enum(Map.get(port, "cardinality"), @cardinalities, "#{port_path}.cardinality")
          |> validate_dataset_value(Map.get(port, "dataset_value"), dataset_ids, "#{port_path}.dataset_value")

        port_id = Map.get(port, "id")
        duplicate_errors = if present_string?(port_id) and MapSet.member?(port_ids, port_id), do: ["#{path} contains duplicate port id #{inspect(port_id)}"], else: []
        next_port_ids = if present_string?(port_id), do: MapSet.put(port_ids, port_id), else: port_ids
        next_bucket = if present_string?(node_id) and present_string?(port_id), do: Map.put(bucket, {node_id, port_id}, port), else: bucket
        {errors ++ current_errors ++ duplicate_errors, next_bucket, next_port_ids}
      else
        {errors ++ ["#{port_path} must be an object"], bucket, port_ids}
      end
    end)
    |> then(fn {errors, bucket, _port_ids} -> {errors, bucket} end)
  end

  defp validate_edges(edges, dataset_ids, input_ports, output_ports) when is_list(edges) do
    Enum.with_index(edges)
    |> Enum.reduce({[], MapSet.new()}, fn {edge, index}, {errors, edge_ids} ->
      path = "graph.edges[#{index}]"

      if is_map(edge) do
        current_errors =
          []
          |> require_string(edge, "id", path)
          |> require_string(edge, "artifact_type", path)
          |> validate_dataset_value(Map.get(edge, "dataset_value"), dataset_ids, "#{path}.dataset_value")

        edge_id = Map.get(edge, "id")
        duplicate_errors = if present_string?(edge_id) and MapSet.member?(edge_ids, edge_id), do: ["graph.edges contains duplicate id #{inspect(edge_id)}"], else: []
        next_edge_ids = if present_string?(edge_id), do: MapSet.put(edge_ids, edge_id), else: edge_ids

        {from_errors, from_key} = validate_node_port_ref(Map.get(edge, "from"), "#{path}.from")
        {to_errors, to_key} = validate_node_port_ref(Map.get(edge, "to"), "#{path}.to")
        source_port = if from_key, do: Map.get(output_ports, from_key), else: nil
        target_port = if to_key, do: Map.get(input_ports, to_key), else: nil

        relation_errors =
          []
          |> maybe_add_missing_port(source_port, "#{path}.from", from_key, "output")
          |> maybe_add_missing_port(target_port, "#{path}.to", to_key, "input")
          |> maybe_validate_artifact_match(source_port, Map.get(edge, "artifact_type"), "#{path}.artifact_type", "source")
          |> maybe_validate_artifact_match(target_port, Map.get(edge, "artifact_type"), "#{path}.artifact_type", "target")
          |> maybe_validate_port_artifact_alignment(source_port, target_port, path)
          |> maybe_validate_port_dataset_alignment(source_port, Map.get(edge, "dataset_value"), "#{path}.dataset_value", "source")
          |> maybe_validate_port_dataset_alignment(target_port, Map.get(edge, "dataset_value"), "#{path}.dataset_value", "target")

        {errors ++ current_errors ++ duplicate_errors ++ from_errors ++ to_errors ++ relation_errors, next_edge_ids}
      else
        {errors ++ ["#{path} must be an object"], edge_ids}
      end
    end)
    |> elem(0)
  end

  defp validate_dataset_contract_errors(contract, path) do
    base_errors =
      []
      |> require_string(contract, "schema_version", path)
      |> require_schema_version(Map.get(contract, "schema_version"), @workflow_dataset_schema_version, path)
      |> require_string(contract, "id", path)
      |> require_string(contract, "version", path)
      |> optional_string(contract, "name", path)
      |> optional_string(contract, "description", path)
      |> require_list(contract, "values", path, 1)
      |> validate_metadata(Map.get(contract, "metadata"), "#{path}.metadata")

    values_errors =
      Map.get(contract, "values", [])
      |> validate_dataset_values(path)

    base_errors ++ values_errors
  end

  defp validate_dataset_values(values, path) when is_list(values) do
    Enum.with_index(values)
    |> Enum.reduce({[], MapSet.new()}, fn {value, index}, {errors, ids} ->
      value_path = "#{path}.values[#{index}]"

      if is_map(value) do
        current_errors =
          []
          |> require_string(value, "id", value_path)
          |> require_string(value, "data_class", value_path)
          |> validate_enum(Map.get(value, "data_class"), @data_classes, "#{value_path}.data_class")
          |> require_string(value, "element_type", value_path)
          |> validate_shape(Map.get(value, "shape"), "#{value_path}.shape")
          |> optional_string(value, "semantic_type", value_path)
          |> optional_string(value, "unit", value_path)
          |> validate_enum(Map.get(value, "encoding"), @encodings, "#{value_path}.encoding")
          |> validate_schema_ref(Map.get(value, "schema_ref"), "#{value_path}.schema_ref")

        value_id = Map.get(value, "id")
        duplicate_errors = if present_string?(value_id) and MapSet.member?(ids, value_id), do: ["#{path}.values contains duplicate id #{inspect(value_id)}"], else: []
        next_ids = if present_string?(value_id), do: MapSet.put(ids, value_id), else: ids
        {errors ++ current_errors ++ duplicate_errors, next_ids}
      else
        {errors ++ ["#{value_path} must be an object"], ids}
      end
    end)
    |> elem(0)
  end

  defp validate_shape(errors, nil, path), do: errors ++ ["#{path} must be an object"]

  defp validate_shape(errors, shape, path) when is_map(shape) do
    axes = Map.get(shape, "axes")

    cond do
      is_nil(axes) -> errors
      is_list(axes) -> errors ++ validate_axes(axes, "#{path}.axes")
      true -> errors ++ ["#{path}.axes must be a list"]
    end
  end

  defp validate_shape(errors, _shape, path), do: errors ++ ["#{path} must be an object"]

  defp validate_axes(axes, path) do
    Enum.with_index(axes)
    |> Enum.reduce({[], MapSet.new()}, fn {axis, index}, {errors, axis_ids} ->
      axis_path = "#{path}[#{index}]"

      if is_map(axis) do
        current_errors =
          []
          |> require_string(axis, "id", axis_path)
          |> optional_string(axis, "label", axis_path)
          |> optional_string(axis, "semantic", axis_path)
          |> validate_non_negative_integer(Map.get(axis, "size"), "#{axis_path}.size")

        axis_id = Map.get(axis, "id")
        duplicate_errors = if present_string?(axis_id) and MapSet.member?(axis_ids, axis_id), do: ["#{path} contains duplicate id #{inspect(axis_id)}"], else: []
        next_axis_ids = if present_string?(axis_id), do: MapSet.put(axis_ids, axis_id), else: axis_ids
        {errors ++ current_errors ++ duplicate_errors, next_axis_ids}
      else
        {errors ++ ["#{axis_path} must be an object"], axis_ids}
      end
    end)
    |> elem(0)
  end

  defp validate_schema_ref(errors, nil, _path), do: errors

  defp validate_schema_ref(errors, schema_ref, path) when is_map(schema_ref) do
    errors
    |> require_string(schema_ref, "schema", path)
    |> require_string(schema_ref, "version", path)
  end

  defp validate_schema_ref(errors, _schema_ref, path), do: errors ++ ["#{path} must be an object"]

  defp validate_defaults(errors, nil), do: errors

  defp validate_defaults(errors, defaults) when is_map(defaults) do
    errors
    |> validate_cache_policy(Map.get(defaults, "cache_policy"), "graph.defaults.cache_policy")
    |> validate_boolean(Map.get(defaults, "orchestrated"), "graph.defaults.orchestrated")
  end

  defp validate_defaults(errors, _defaults), do: errors ++ ["graph.defaults must be an object"]

  defp validate_metadata(errors, nil, _path), do: errors

  defp validate_metadata(errors, metadata, path) when is_map(metadata) do
    Enum.reduce(metadata, errors, fn {key, value}, acc ->
      acc
      |> maybe_append(if(not present_string?(key), do: "#{path} contains an empty key", else: nil))
      |> maybe_append(if(not is_binary(value), do: "#{path}[#{inspect(key)}] must be a string", else: nil))
    end)
  end

  defp validate_metadata(errors, _metadata, path), do: errors ++ ["#{path} must be an object"]

  defp validate_named_node_refs(node_ids, known_ids, path) when is_list(node_ids) do
    Enum.with_index(node_ids)
    |> Enum.reduce([], fn {node_id, index}, errors ->
      cond do
        not present_string?(node_id) ->
          errors ++ ["#{path}[#{index}] must be a non-empty string"]

        not MapSet.member?(known_ids, node_id) ->
          errors ++ ["#{path}[#{index}] references unknown node #{inspect(node_id)}"]

        true ->
          errors
      end
    end)
  end

  defp validate_named_node_refs(_node_ids, _known_ids, path), do: ["#{path} must be a list"]

  defp validate_node_port_ref(nil, path), do: {["#{path} must be an object"], nil}

  defp validate_node_port_ref(ref, path) when is_map(ref) do
    errors =
      []
      |> require_string(ref, "node", path)
      |> require_string(ref, "port", path)

    if errors == [] do
      {[], {Map.get(ref, "node"), Map.get(ref, "port")}}
    else
      {errors, nil}
    end
  end

  defp validate_node_port_ref(_ref, path), do: {["#{path} must be an object"], nil}

  defp validate_enum(errors, nil, _allowed, _path), do: errors

  defp validate_enum(errors, value, allowed, path) do
    if value in allowed, do: errors, else: errors ++ ["#{path} is invalid"]
  end

  defp validate_boolean(errors, nil, _path), do: errors
  defp validate_boolean(errors, value, _path) when is_boolean(value), do: errors
  defp validate_boolean(errors, _value, path), do: errors ++ ["#{path} must be a boolean"]

  defp validate_non_negative_integer(errors, nil, _path), do: errors
  defp validate_non_negative_integer(errors, value, _path) when is_integer(value) and value >= 0, do: errors
  defp validate_non_negative_integer(errors, _value, path), do: errors ++ ["#{path} must be a non-negative integer"]

  defp validate_dataset_value(errors, nil, _dataset_ids, _path), do: errors

  defp validate_dataset_value(errors, value, dataset_ids, path) do
    cond do
      not present_string?(value) ->
        errors ++ ["#{path} must be a non-empty string"]

      MapSet.size(dataset_ids) > 0 and not MapSet.member?(dataset_ids, value) ->
        errors ++ ["#{path} #{inspect(value)} is not declared in graph.dataset_contract"]

      true ->
        errors
    end
  end

  defp validate_cache_policy(errors, nil, _path), do: errors
  defp validate_cache_policy(errors, value, _path) when value in ~w(ephemeral cached persisted), do: errors
  defp validate_cache_policy(errors, _value, path), do: errors ++ ["#{path} is invalid"]

  defp validate_node_config(errors, nil, _path), do: errors
  defp validate_node_config(errors, value, _path) when is_map(value), do: errors
  defp validate_node_config(errors, _value, path), do: errors ++ ["#{path} must be an object"]

  defp require_operator_id(errors, node, path) do
    if Map.get(node, "kind") in @operator_node_kinds do
      require_option_string(errors, Map.get(node, "operator_id"), "#{path}.operator_id")
    else
      errors
    end
  end

  defp require_string(errors, map, key, path) do
    if present_string?(Map.get(map, key)), do: errors, else: errors ++ ["#{path}.#{key} must be a non-empty string"]
  end

  defp require_option_string(errors, value, path) do
    if present_string?(value), do: errors, else: errors ++ ["#{path} must be a non-empty string"]
  end

  defp require_schema_version(errors, value, expected, path) do
    if value == expected, do: errors, else: errors ++ ["#{path}.schema_version must be #{inspect(expected)}"]
  end

  defp require_list(errors, map, key, path, min_items) do
    case Map.get(map, key) do
      value when is_list(value) and length(value) >= min_items -> errors
      value when is_list(value) -> errors ++ ["#{path}.#{key} must contain at least #{min_items} item(s)"]
      _ -> errors ++ ["#{path}.#{key} must be a list"]
    end
  end

  defp optional_list(errors, map, key, path) do
    case Map.get(map, key) do
      nil -> errors
      value when is_list(value) -> errors
      _ -> errors ++ ["#{path}.#{key} must be a list"]
    end
  end

  defp optional_string(errors, map, key, path) do
    case Map.get(map, key) do
      nil -> errors
      value when is_binary(value) -> errors
      _ -> errors ++ ["#{path}.#{key} must be a string"]
    end
  end

  defp maybe_add_missing_port(errors, nil, path, key, kind) when not is_nil(key),
    do: errors ++ ["#{path} references unknown #{kind} port #{inspect(key)}"]

  defp maybe_add_missing_port(errors, _port, _path, _key, _kind), do: errors

  defp maybe_validate_artifact_match(errors, nil, _artifact_type, _path, _label), do: errors

  defp maybe_validate_artifact_match(errors, port, artifact_type, path, label) do
    if Map.get(port, "artifact_type") == artifact_type do
      errors
    else
      errors ++ ["#{path} does not match #{label} port artifact_type"]
    end
  end

  defp maybe_validate_port_artifact_alignment(errors, nil, _target, _path), do: errors
  defp maybe_validate_port_artifact_alignment(errors, _source, nil, _path), do: errors

  defp maybe_validate_port_artifact_alignment(errors, source, target, path) do
    if Map.get(source, "artifact_type") == Map.get(target, "artifact_type") do
      errors
    else
      errors ++ ["#{path} connects ports with mismatched artifact_type values"]
    end
  end

  defp maybe_validate_port_dataset_alignment(errors, _port, nil, _path, _label), do: errors
  defp maybe_validate_port_dataset_alignment(errors, nil, _dataset_value, _path, _label), do: errors

  defp maybe_validate_port_dataset_alignment(errors, port, dataset_value, path, label) do
    port_dataset_value = Map.get(port, "dataset_value")

    if is_nil(port_dataset_value) or port_dataset_value == dataset_value do
      errors
    else
      errors ++ ["#{path} does not match #{label} port dataset_value"]
    end
  end

  defp normalize_nested_errors(%MapSet{}), do: []
  defp normalize_nested_errors(errors) when is_list(errors), do: errors

  defp message_to_lines(message, prefix) do
    message
    |> String.split("\n", trim: true)
    |> Enum.drop_while(&(&1 == "workflow contract validation failed:"))
    |> Enum.map(fn line -> "#{prefix}: " <> String.trim_leading(line, "- ") end)
  end

  defp present_string?(value), do: is_binary(value) and String.trim(value) != ""

  defp maybe_append(errors, nil), do: errors
  defp maybe_append(errors, message), do: errors ++ [message]
end
