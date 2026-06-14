defmodule KyuubikiWeb.WorkflowOperatorBridgeRuntime do
  @moduledoc false

  def bridge_electrostatic_result_to_heat_plane_quad_model(
        electrostatic_result,
        heat_seed_model,
        bridge_contract
      ),
      do: bridge_electrostatic_result_to_heat_model(electrostatic_result, heat_seed_model, bridge_contract)

  def bridge_electrostatic_result_to_heat_plane_triangle_model(
        electrostatic_result,
        heat_seed_model,
        bridge_contract
      ),
      do: bridge_electrostatic_result_to_heat_model(electrostatic_result, heat_seed_model, bridge_contract)

  def resolve_electrostatic_to_heat_bridge_contract(config) when is_map(config) do
    contract = Map.get(config, "contract", %{})

    with {:ok, source_field} <-
           normalize_contract_string(
             get_in(contract, ["source", "field"]) || "electric_field_magnitude",
             :invalid_bridge_contract_source_field
           ),
         {:ok, distribution} <-
           normalize_contract_string(
             get_in(contract, ["source", "distribution"]) || "element_to_nodes",
             :invalid_bridge_contract_distribution
           ),
         :ok <- validate_bridge_distribution(distribution),
         {:ok, node_index_fields} <-
           normalize_node_index_fields(
             get_in(contract, ["source", "node_index_fields"]) ||
               ["node_i", "node_j", "node_k", "node_l"]
           ),
         {:ok, scale} <-
           normalize_bridge_scale(
             get_in(contract, ["transform", "scale"]) || Map.get(config, "field_to_heat_scale")
           ),
         {:ok, reduction} <-
           normalize_contract_string(
             get_in(contract, ["transform", "reduction"]) || "mean",
             :invalid_bridge_contract_reduction
           ),
         :ok <- validate_bridge_reduction(reduction),
         {:ok, default_value} <-
           normalize_bridge_scale(get_in(contract, ["transform", "default_value"]) || 0.0),
         {:ok, target_field} <-
           normalize_contract_string(
             get_in(contract, ["target", "field"]) || "heat_load",
             :invalid_bridge_contract_target_field
           ),
         :ok <- validate_electrostatic_bridge_source_field(source_field, distribution) do
      {:ok,
       %{
         source_field: source_field,
         distribution: distribution,
         node_index_fields: node_index_fields,
         scale: scale,
         reduction: reduction,
         default_value: default_value,
         target_field: target_field
       }}
    end
  end

  def resolve_electrostatic_to_heat_bridge_contract(_config),
    do: {:error, :invalid_bridge_contract}


  defp bridge_electrostatic_result_to_heat_model(
         %{"nodes" => electrostatic_nodes, "elements" => electrostatic_elements},
         %{"nodes" => heat_nodes, "elements" => heat_elements} = heat_seed_model,
         bridge_contract
       )
       when is_list(electrostatic_nodes) and is_list(electrostatic_elements) and
              is_list(heat_nodes) and is_list(heat_elements) and is_map(bridge_contract) do
    if length(electrostatic_nodes) != length(heat_nodes) do
      {:error, :node_count_mismatch}
    else
      validate_and_bridge_electrostatic_nodes(
        electrostatic_nodes,
        electrostatic_elements,
        heat_nodes,
        heat_elements,
        heat_seed_model,
        bridge_contract
      )
    end
  end

  defp bridge_electrostatic_result_to_heat_model(_result, _seed_model, _bridge_contract),
    do: {:error, :invalid_bridge_payload}

  defp validate_and_bridge_electrostatic_nodes(
         electrostatic_nodes,
         electrostatic_elements,
         heat_nodes,
         heat_elements,
         heat_seed_model,
         bridge_contract
       ) do
    with :ok <- ensure_node_alignment(electrostatic_nodes, heat_nodes),
         :ok <- validate_electrostatic_bridge_shape(electrostatic_elements, heat_elements, bridge_contract),
         nodal_heat_loads <-
           derive_nodal_target_field(
             electrostatic_nodes,
             electrostatic_elements,
             length(heat_nodes),
             bridge_contract
           ) do
        bridged_nodes =
          Enum.with_index(heat_nodes)
          |> Enum.map(fn {heat_node, index} ->
            Map.put(
              heat_node,
              bridge_contract.target_field,
              Enum.at(nodal_heat_loads, index, bridge_contract.default_value)
            )
          end)

        {:ok, Map.put(heat_seed_model, "nodes", bridged_nodes)}
    end
  end

  defp validate_electrostatic_bridge_shape(_electrostatic_elements, _heat_elements, %{
         distribution: "node_to_node"
       }),
       do: :ok

  defp validate_electrostatic_bridge_shape(electrostatic_elements, heat_elements, _bridge_contract) do
    if length(electrostatic_elements) == length(heat_elements),
      do: :ok,
      else: {:error, :element_count_mismatch}
  end

  defp ensure_node_alignment(source_nodes, target_nodes) do
    source_nodes
    |> Enum.zip(target_nodes)
    |> Enum.reduce_while(:ok, fn {source_node, target_node}, _acc ->
      if close_enough?(Map.get(source_node, "x"), Map.get(target_node, "x")) and
           close_enough?(Map.get(source_node, "y"), Map.get(target_node, "y")) do
        {:cont, :ok}
      else
        {:halt, {:error, :node_alignment_mismatch}}
      end
    end)
  end

  defp derive_nodal_target_field(
         nodes,
         _elements,
         _node_count,
         %{distribution: "node_to_node", source_field: source_field, scale: scale, default_value: default_value}
       ) do
    Enum.map(nodes, fn node ->
      node
      |> Map.get(source_field, default_value)
      |> normalize_numeric_value()
      |> Kernel.*(scale)
    end)
  end

  defp derive_nodal_target_field(
         _nodes,
         elements,
         node_count,
         %{source_field: source_field, node_index_fields: node_index_fields, scale: scale} =
           bridge_contract
       ) do
    {totals, counts, minima, maxima} =
      Enum.reduce(
        elements,
        {
          List.duplicate(0.0, node_count),
          List.duplicate(0, node_count),
          List.duplicate(nil, node_count),
          List.duplicate(nil, node_count)
        },
        fn element, {totals, counts, minima, maxima} ->
          magnitude =
            element
            |> electrostatic_bridge_source_value(source_field, bridge_contract.default_value)

          node_indexes =
            Enum.map(node_index_fields, &Map.get(element, &1)) |> Enum.filter(&is_integer/1)

          Enum.reduce(node_indexes, {totals, counts, minima, maxima}, fn node_index, {totals_acc, counts_acc, minima_acc, maxima_acc} ->
            next_min =
              minima_acc
              |> Enum.at(node_index)
              |> case do
                nil -> magnitude * scale
                current -> min(current, magnitude * scale)
              end

            next_max =
              maxima_acc
              |> Enum.at(node_index)
              |> case do
                nil -> magnitude * scale
                current -> max(current, magnitude * scale)
              end

            {
              List.update_at(totals_acc, node_index, &(&1 + magnitude * scale)),
              List.update_at(counts_acc, node_index, &(&1 + 1)),
              List.replace_at(minima_acc, node_index, next_min),
              List.replace_at(maxima_acc, node_index, next_max)
            }
          end)
        end
      )

    reduce_nodal_values(totals, counts, minima, maxima, bridge_contract)
  end

  defp reduce_nodal_values(totals, counts, _minima, _maxima, %{
         reduction: "sum",
         default_value: default_value
       }) do
    Enum.zip(totals, counts)
    |> Enum.map(fn
      {_total, 0} -> default_value
      {total, _count} -> total
    end)
  end

  defp reduce_nodal_values(_totals, counts, minima, _maxima, %{
         reduction: "min",
         default_value: default_value
       }) do
    Enum.zip(minima, counts)
    |> Enum.map(fn
      {_minimum, 0} -> default_value
      {minimum, _count} -> minimum || default_value
    end)
  end

  defp reduce_nodal_values(_totals, counts, _minima, maxima, %{
         reduction: "max",
         default_value: default_value
       }) do
    Enum.zip(maxima, counts)
    |> Enum.map(fn
      {_maximum, 0} -> default_value
      {maximum, _count} -> maximum || default_value
    end)
  end

  defp reduce_nodal_values(totals, counts, _minima, _maxima, %{default_value: default_value}) do
    Enum.zip(totals, counts)
    |> Enum.map(fn
      {_total, 0} -> default_value
      {total, count} -> total / count
    end)
  end

  defp electrostatic_bridge_source_value(element, "flux_magnitude", default_value) do
    element
    |> Map.get("electric_flux_density_magnitude", default_value)
    |> normalize_numeric_value()
  end

  defp electrostatic_bridge_source_value(element, source_field, default_value) do
    element
    |> Map.get(source_field, default_value)
    |> normalize_numeric_value()
  end

  defp normalize_bridge_scale(nil), do: {:ok, 1.0}
  defp normalize_bridge_scale(scale) when is_number(scale), do: {:ok, scale}
  defp normalize_bridge_scale(_scale), do: {:error, :invalid_bridge_scale}

  defp normalize_contract_string(value, _reason)
       when is_binary(value) and value != "",
       do: {:ok, value}

  defp normalize_contract_string(_value, reason), do: {:error, reason}

  defp validate_bridge_distribution("element_to_nodes"), do: :ok
  defp validate_bridge_distribution("node_to_node"), do: :ok
  defp validate_bridge_distribution(_distribution), do: {:error, :unsupported_bridge_distribution}

  defp validate_bridge_reduction("mean"), do: :ok
  defp validate_bridge_reduction("sum"), do: :ok
  defp validate_bridge_reduction("area_weighted_mean"), do: :ok
  defp validate_bridge_reduction("min"), do: :ok
  defp validate_bridge_reduction("max"), do: :ok
  defp validate_bridge_reduction(_reduction), do: {:error, :unsupported_bridge_reduction}

  defp validate_electrostatic_bridge_source_field("potential", "node_to_node"), do: :ok
  defp validate_electrostatic_bridge_source_field("charge_density", "node_to_node"), do: :ok

  defp validate_electrostatic_bridge_source_field(
         "electric_field_magnitude",
         "element_to_nodes"
       ),
       do: :ok

  defp validate_electrostatic_bridge_source_field("electric_field_x", "element_to_nodes"),
    do: :ok

  defp validate_electrostatic_bridge_source_field("electric_field_y", "element_to_nodes"),
    do: :ok

  defp validate_electrostatic_bridge_source_field("average_potential", "element_to_nodes"),
    do: :ok

  defp validate_electrostatic_bridge_source_field("flux_magnitude", "element_to_nodes"),
    do: :ok

  defp validate_electrostatic_bridge_source_field(
         "electric_flux_density_magnitude",
         "element_to_nodes"
       ),
       do: :ok

  defp validate_electrostatic_bridge_source_field(
         "electric_flux_density_x",
         "element_to_nodes"
       ),
       do: :ok

  defp validate_electrostatic_bridge_source_field(
         "electric_flux_density_y",
         "element_to_nodes"
       ),
       do: :ok

  defp validate_electrostatic_bridge_source_field(_source_field, _distribution),
    do: {:error, :invalid_bridge_contract_source_field}

  defp normalize_node_index_fields(fields) when is_list(fields) do
    normalized =
      fields
      |> Enum.filter(&(is_binary(&1) and &1 != ""))
      |> Enum.uniq()

    if normalized == [] do
      {:error, :invalid_bridge_contract_node_index_fields}
    else
      {:ok, normalized}
    end
  end

  defp normalize_node_index_fields(_fields),
    do: {:error, :invalid_bridge_contract_node_index_fields}

  defp normalize_numeric_value(value) when is_number(value), do: value
  defp normalize_numeric_value(_value), do: 0.0

  defp close_enough?(left, right) when is_number(left) and is_number(right),
    do: abs(left - right) <= 1.0e-9

  defp close_enough?(_, _), do: false
end
