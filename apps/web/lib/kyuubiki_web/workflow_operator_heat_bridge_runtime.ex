defmodule KyuubikiWeb.WorkflowOperatorHeatBridgeRuntime do
  @moduledoc false
  alias KyuubikiWeb.WorkflowHeatBridgeContract, as: Contract

  def bridge_heat_result_to_thermal_plane_quad_model(heat_result, thermo_seed_model),
    do: bridge_heat_result_to_thermal_model(heat_result, thermo_seed_model, "temperature_delta")

  def bridge_heat_result_to_thermal_plane_triangle_model(heat_result, thermo_seed_model),
    do: bridge_heat_result_to_thermal_model(heat_result, thermo_seed_model, "temperature_delta")

  def bridge_heat_result_to_thermal_plane_quad_model(
        heat_result,
        thermo_seed_model,
        bridge_contract
      ) do
    with {:ok, contract} <- Contract.for_shape(bridge_contract, :quad),
         do: bridge_heat_result_to_thermal_model(heat_result, thermo_seed_model, contract)
  end

  def bridge_heat_result_to_thermal_plane_triangle_model(
        heat_result,
        thermo_seed_model,
        bridge_contract
      ) do
    with {:ok, contract} <- Contract.for_shape(bridge_contract, :triangle),
         do: bridge_heat_result_to_thermal_model(heat_result, thermo_seed_model, contract)
  end

  defdelegate resolve_heat_to_thermo_bridge_contract(config), to: Contract, as: :resolve

  defp bridge_heat_result_to_thermal_model(
         %{"nodes" => heat_nodes} = heat_result,
         %{"nodes" => thermo_nodes, "elements" => thermo_elements} = thermo_seed_model,
         target_field
       )
       when is_list(heat_nodes) and is_list(thermo_nodes) and is_list(thermo_elements) and
              is_binary(target_field) do
    case resolve_heat_elements(heat_result, "node_to_node") do
      {:ok, heat_elements} ->
        validate_bridge_shapes(heat_nodes, heat_elements, thermo_nodes, thermo_elements, fn ->
          bridge_heat_nodes(heat_nodes, heat_elements, thermo_nodes, thermo_seed_model, %{
            target_field: target_field,
            source_field: "temperature",
            distribution: "node_to_node",
            node_index_fields: [],
            reduction: "copy",
            scale: 1.0,
            reference_temperature: 0.0,
            default_value: 0.0
          })
        end)

      error ->
        error
    end
  end

  defp bridge_heat_result_to_thermal_model(
         %{"nodes" => heat_nodes} = heat_result,
         %{"nodes" => thermo_nodes, "elements" => thermo_elements} = thermo_seed_model,
         %{distribution: distribution} = bridge_contract
       )
       when is_list(heat_nodes) and is_list(thermo_nodes) and is_list(thermo_elements) and
              is_map(bridge_contract) do
    case resolve_heat_elements(heat_result, distribution) do
      {:ok, heat_elements} ->
        with :ok <- validate_element_evidence(heat_result, heat_elements, bridge_contract) do
          validate_heat_bridge_shapes(
            heat_nodes,
            heat_elements,
            thermo_nodes,
            thermo_elements,
            bridge_contract,
            fn ->
              bridge_heat_nodes(
                heat_nodes,
                heat_elements,
                thermo_nodes,
                thermo_seed_model,
                bridge_contract
              )
            end
          )
        end

      error ->
        error
    end
  end

  defp bridge_heat_result_to_thermal_model(_heat_result, _thermo_seed_model, _target_or_contract),
    do: {:error, :invalid_bridge_payload}

  defp validate_heat_bridge_shapes(
         heat_nodes,
         heat_elements,
         thermo_nodes,
         thermo_elements,
         %{distribution: "element_to_nodes"},
         on_valid
       ),
       do:
         validate_bridge_shapes(
           heat_nodes,
           heat_elements,
           thermo_nodes,
           thermo_elements,
           on_valid
         )

  defp validate_heat_bridge_shapes(
         heat_nodes,
         heat_elements,
         thermo_nodes,
         thermo_elements,
         _bridge_contract,
         on_valid
       ) do
    validate_bridge_shapes(heat_nodes, heat_elements, thermo_nodes, thermo_elements, on_valid)
  end

  defp bridge_heat_nodes(
         heat_nodes,
         heat_elements,
         thermo_nodes,
         thermo_seed_model,
         bridge_contract
       ) do
    with :ok <- ensure_node_alignment(heat_nodes, thermo_nodes),
         :ok <-
           validate_field_values(heat_nodes, heat_elements, length(thermo_nodes), bridge_contract),
         nodal_values <-
           derive_heat_nodal_target_field(
             heat_nodes,
             heat_elements,
             length(thermo_nodes),
             bridge_contract
           ) do
      bridged_nodes =
        Enum.with_index(thermo_nodes)
        |> Enum.map(fn {thermo_node, index} ->
          Map.put(
            thermo_node,
            bridge_contract.target_field,
            Enum.at(nodal_values, index, bridge_contract.default_value)
          )
        end)

      {:ok, Map.put(thermo_seed_model, "nodes", bridged_nodes)}
    end
  rescue
    ArithmeticError -> {:error, :bridge_numeric_overflow}
  end

  defp derive_heat_nodal_target_field(
         heat_nodes,
         _heat_elements,
         _node_count,
         %{
           distribution: "node_to_node",
           source_field: source_field,
           scale: scale,
           default_value: default_value
         } = bridge_contract
       ) do
    reference = Map.get(bridge_contract, :reference_temperature, 0.0)

    Enum.map(heat_nodes, fn node ->
      node
      |> Map.get(source_field, default_value)
      |> normalize_numeric_value()
      |> Kernel.-(reference)
      |> Kernel.*(scale)
    end)
  end

  defp derive_heat_nodal_target_field(
         _heat_nodes,
         heat_elements,
         node_count,
         %{source_field: source_field, node_index_fields: node_index_fields, scale: scale} =
           bridge_contract
       ) do
    {totals, counts, weighted_totals, weight_sums, minima, maxima} =
      Enum.reduce(
        heat_elements,
        {
          List.duplicate(0.0, node_count),
          List.duplicate(0, node_count),
          List.duplicate(0.0, node_count),
          List.duplicate(0.0, node_count),
          List.duplicate(nil, node_count),
          List.duplicate(nil, node_count)
        },
        fn element, {totals, counts, weighted_totals, weight_sums, minima, maxima} ->
          magnitude =
            element
            |> Map.get(source_field, bridge_contract.default_value)
            |> normalize_numeric_value()
            |> Kernel.-(Map.get(bridge_contract, :reference_temperature, 0.0))
            |> Kernel.*(scale)

          weight = normalize_numeric_value(Map.get(element, "area", 1.0))

          sum_value =
            if bridge_contract.reduction in ["copy", "mean", "sum"], do: magnitude, else: 0.0

          weighted_value =
            if bridge_contract.reduction == "area_weighted_mean",
              do: magnitude * weight,
              else: 0.0

          active_weight =
            if bridge_contract.reduction == "area_weighted_mean", do: weight, else: 0.0

          node_indexes =
            Enum.map(node_index_fields, &Map.get(element, &1)) |> Enum.filter(&is_integer/1)

          Enum.reduce(
            node_indexes,
            {totals, counts, weighted_totals, weight_sums, minima, maxima},
            fn node_index,
               {totals_acc, counts_acc, weighted_totals_acc, weight_sums_acc, minima_acc,
                maxima_acc} ->
              next_min =
                minima_acc
                |> Enum.at(node_index)
                |> case do
                  nil -> magnitude
                  current -> min(current, magnitude)
                end

              next_max =
                maxima_acc
                |> Enum.at(node_index)
                |> case do
                  nil -> magnitude
                  current -> max(current, magnitude)
                end

              {
                List.update_at(totals_acc, node_index, &(&1 + sum_value)),
                List.update_at(counts_acc, node_index, &(&1 + 1)),
                List.update_at(weighted_totals_acc, node_index, &(&1 + weighted_value)),
                List.update_at(weight_sums_acc, node_index, &(&1 + active_weight)),
                List.replace_at(minima_acc, node_index, next_min),
                List.replace_at(maxima_acc, node_index, next_max)
              }
            end
          )
        end
      )

    reduce_heat_nodal_values(
      totals,
      counts,
      weighted_totals,
      weight_sums,
      minima,
      maxima,
      bridge_contract
    )
  end

  defp reduce_heat_nodal_values(
         totals,
         counts,
         _weighted_totals,
         _weight_sums,
         _minima,
         _maxima,
         %{reduction: reduction, default_value: default_value}
       )
       when reduction in ["copy", "mean"] do
    Enum.zip(totals, counts)
    |> Enum.map(fn
      {_total, 0} -> default_value
      {total, count} -> total / count
    end)
  end

  defp reduce_heat_nodal_values(
         totals,
         counts,
         _weighted_totals,
         _weight_sums,
         _minima,
         _maxima,
         %{reduction: "sum", default_value: default_value}
       ) do
    Enum.zip(totals, counts)
    |> Enum.map(fn
      {_total, 0} -> default_value
      {total, _count} -> total
    end)
  end

  defp reduce_heat_nodal_values(
         _totals,
         counts,
         weighted_totals,
         weight_sums,
         _minima,
         _maxima,
         %{reduction: "area_weighted_mean", default_value: default_value}
       ) do
    Enum.zip([weighted_totals, weight_sums, counts])
    |> Enum.map(fn
      {weighted_total, weight_sum, count} when count > 0 and weight_sum > 0 ->
        weighted_total / weight_sum

      {_weighted_total, _weight_sum, _count} ->
        default_value
    end)
  end

  defp reduce_heat_nodal_values(
         _totals,
         counts,
         _weighted_totals,
         _weight_sums,
         minima,
         _maxima,
         %{reduction: "min", default_value: default_value}
       ) do
    Enum.zip(minima, counts)
    |> Enum.map(fn
      {_minimum, 0} -> default_value
      {minimum, _count} -> minimum || default_value
    end)
  end

  defp reduce_heat_nodal_values(
         _totals,
         counts,
         _weighted_totals,
         _weight_sums,
         _minima,
         maxima,
         %{reduction: "max", default_value: default_value}
       ) do
    Enum.zip(maxima, counts)
    |> Enum.map(fn
      {_maximum, 0} -> default_value
      {maximum, _count} -> maximum || default_value
    end)
  end

  defp resolve_heat_elements(%{"elements" => elements}, "element_to_nodes")
       when is_list(elements),
       do: {:ok, elements}

  defp resolve_heat_elements(%{"input" => %{"elements" => elements}}, distribution)
       when is_list(elements) and distribution in ["node_to_node", "element_to_nodes"],
       do: {:ok, elements}

  defp resolve_heat_elements(_heat_result, _distribution), do: {:error, :invalid_bridge_payload}

  defp validate_bridge_shapes(
         source_nodes,
         source_elements,
         target_nodes,
         target_elements,
         on_valid
       ) do
    cond do
      length(source_nodes) != length(target_nodes) -> {:error, :node_count_mismatch}
      length(source_elements) != length(target_elements) -> {:error, :element_count_mismatch}
      true -> on_valid.()
    end
  end

  defp ensure_node_alignment(source_nodes, target_nodes) do
    source_nodes
    |> Enum.zip(target_nodes)
    |> Enum.reduce_while(:ok, fn {source_node, target_node}, _acc ->
      if is_map(source_node) and is_map(target_node) and
           close_enough?(Map.get(source_node, "x"), Map.get(target_node, "x")) and
           close_enough?(Map.get(source_node, "y"), Map.get(target_node, "y")) do
        {:cont, :ok}
      else
        {:halt, {:error, :node_alignment_mismatch}}
      end
    end)
  end

  defp validate_element_evidence(_heat, _elements, %{distribution: "node_to_node"}), do: :ok

  defp validate_element_evidence(heat, elements, contract) do
    input = get_in(heat, ["input", "elements"])

    valid =
      is_list(input) and length(input) == length(elements) and
        Enum.all?(Enum.zip(elements, input), fn {result, original} ->
          is_map(result) and is_map(original) and result["id"] == original["id"] and
            Enum.all?(contract.node_index_fields, fn field ->
              Map.has_key?(original, field) and result[field] == original[field]
            end)
        end)

    if valid, do: :ok, else: {:error, :bridge_element_evidence_mismatch}
  end

  defp validate_field_values(nodes, _elements, _count, %{distribution: "node_to_node"} = contract) do
    valid =
      Enum.all?(nodes, fn node ->
        is_map(node) and Contract.finite_number?(Map.get(node, contract.source_field))
      end)

    if valid, do: :ok, else: {:error, :invalid_bridge_source_value}
  end

  defp validate_field_values(_nodes, elements, count, contract) do
    valid =
      Enum.all?(elements, fn element ->
        is_map(element) and Contract.finite_number?(Map.get(element, contract.source_field)) and
          Contract.finite_number?(element["area"]) and element["area"] > 0 and
          Enum.all?(contract.node_index_fields, fn field ->
            index = element[field]
            is_integer(index) and index >= 0 and index < count
          end)
      end)

    if valid, do: :ok, else: {:error, :invalid_bridge_element_value_or_index}
  end

  defp normalize_numeric_value(value) when is_number(value), do: value
  defp normalize_numeric_value(_value), do: 0.0

  defp close_enough?(left, right) when is_number(left) and is_number(right),
    do:
      Contract.finite_number?(left) and Contract.finite_number?(right) and
        abs(left - right) <= 1.0e-9

  defp close_enough?(_, _), do: false
end
