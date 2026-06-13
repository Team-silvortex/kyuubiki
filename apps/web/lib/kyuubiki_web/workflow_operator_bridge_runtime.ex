defmodule KyuubikiWeb.WorkflowOperatorBridgeRuntime do
  @moduledoc false

  def bridge_heat_result_to_thermal_plane_quad_model(heat_result, thermo_seed_model),
    do: bridge_heat_result_to_thermal_model(heat_result, thermo_seed_model, "temperature_delta")

  def bridge_heat_result_to_thermal_plane_triangle_model(heat_result, thermo_seed_model),
    do: bridge_heat_result_to_thermal_model(heat_result, thermo_seed_model, "temperature_delta")

  def bridge_heat_result_to_thermal_plane_quad_model(
        heat_result,
        thermo_seed_model,
        bridge_contract
      ),
      do: bridge_heat_result_to_thermal_model(heat_result, thermo_seed_model, bridge_contract)

  def bridge_heat_result_to_thermal_plane_triangle_model(
        heat_result,
        thermo_seed_model,
        bridge_contract
      ),
      do: bridge_heat_result_to_thermal_model(heat_result, thermo_seed_model, bridge_contract)

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
           ) do
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

  def resolve_heat_to_thermo_bridge_contract(config) when is_map(config) do
    contract = Map.get(config, "contract", %{})

    with {:ok, source_field} <-
           normalize_contract_string(
             get_in(contract, ["source", "field"]) || "temperature",
             :invalid_bridge_contract_source_field
           ),
         {:ok, target_field} <-
           normalize_contract_string(
             get_in(contract, ["target", "field"]) || "temperature_delta",
             :invalid_bridge_contract_target_field
           ),
         {:ok, scale} <-
           normalize_bridge_scale(get_in(contract, ["transform", "scale"]) || 1.0),
         {:ok, default_value} <-
           normalize_bridge_scale(get_in(contract, ["transform", "default_value"]) || 0.0) do
      {:ok,
       %{
         source_field: source_field,
         target_field: target_field,
         scale: scale,
         default_value: default_value
       }}
    end
  end

  def resolve_heat_to_thermo_bridge_contract(_config), do: {:error, :invalid_bridge_contract}

  defp bridge_heat_result_to_thermal_model(
         %{"nodes" => heat_nodes, "input" => %{"elements" => heat_elements}},
         %{"nodes" => thermo_nodes, "elements" => thermo_elements} = thermo_seed_model,
         target_field
       )
       when is_list(heat_nodes) and is_list(heat_elements) and is_list(thermo_nodes) and
              is_list(thermo_elements) and is_binary(target_field) do
    validate_bridge_shapes(heat_nodes, heat_elements, thermo_nodes, thermo_elements, fn ->
      bridge_heat_nodes(heat_nodes, thermo_nodes, thermo_seed_model, %{
        target_field: target_field,
        source_field: "temperature",
        scale: 1.0,
        default_value: 0.0
      })
    end)
  end

  defp bridge_heat_result_to_thermal_model(
         %{"nodes" => heat_nodes, "input" => %{"elements" => heat_elements}},
         %{"nodes" => thermo_nodes, "elements" => thermo_elements} = thermo_seed_model,
         bridge_contract
       )
       when is_list(heat_nodes) and is_list(heat_elements) and is_list(thermo_nodes) and
              is_list(thermo_elements) and is_map(bridge_contract) do
    validate_bridge_shapes(heat_nodes, heat_elements, thermo_nodes, thermo_elements, fn ->
      bridge_heat_nodes(heat_nodes, thermo_nodes, thermo_seed_model, bridge_contract)
    end)
  end

  defp bridge_heat_result_to_thermal_model(_heat_result, _thermo_seed_model, _target_or_contract),
    do: {:error, :invalid_bridge_payload}

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
      with :ok <- ensure_node_alignment(electrostatic_nodes, heat_nodes),
           nodal_heat_loads <-
             derive_nodal_target_field(
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
  end

  defp bridge_electrostatic_result_to_heat_model(_result, _seed_model, _bridge_contract),
    do: {:error, :invalid_bridge_payload}

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

  defp bridge_heat_nodes(heat_nodes, thermo_nodes, thermo_seed_model, bridge_contract) do
    bridged_nodes =
      Enum.zip(heat_nodes, thermo_nodes)
      |> Enum.reduce_while([], fn {heat_node, thermo_node}, acc ->
        if close_enough?(Map.get(heat_node, "x"), Map.get(thermo_node, "x")) and
             close_enough?(Map.get(heat_node, "y"), Map.get(thermo_node, "y")) do
          {:cont,
           acc ++
             [
               Map.put(
                 thermo_node,
                 bridge_contract.target_field,
                 normalize_numeric_value(
                   Map.get(heat_node, bridge_contract.source_field, bridge_contract.default_value)
                 ) * bridge_contract.scale
               )
             ]}
        else
          {:halt, :mismatch}
        end
      end)

    case bridged_nodes do
      :mismatch -> {:error, :node_alignment_mismatch}
      nodes -> {:ok, Map.put(thermo_seed_model, "nodes", nodes)}
    end
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
         elements,
         node_count,
         %{source_field: source_field, node_index_fields: node_index_fields, scale: scale} =
           bridge_contract
       ) do
    {totals, counts} =
      Enum.reduce(
        elements,
        {List.duplicate(0.0, node_count), List.duplicate(0, node_count)},
        fn element, {totals, counts} ->
          magnitude =
            element
            |> Map.get(source_field, bridge_contract.default_value)
            |> normalize_numeric_value()

          node_indexes =
            Enum.map(node_index_fields, &Map.get(element, &1)) |> Enum.filter(&is_integer/1)

          Enum.reduce(node_indexes, {totals, counts}, fn node_index, {totals_acc, counts_acc} ->
            {
              List.update_at(totals_acc, node_index, &(&1 + magnitude * scale)),
              List.update_at(counts_acc, node_index, &(&1 + 1))
            }
          end)
        end
      )

    reduce_nodal_values(totals, counts, bridge_contract)
  end

  defp reduce_nodal_values(totals, counts, %{reduction: "sum", default_value: default_value}) do
    Enum.zip(totals, counts)
    |> Enum.map(fn
      {_total, 0} -> default_value
      {total, _count} -> total
    end)
  end

  defp reduce_nodal_values(totals, counts, %{default_value: default_value}) do
    Enum.zip(totals, counts)
    |> Enum.map(fn
      {_total, 0} -> default_value
      {total, count} -> total / count
    end)
  end

  defp normalize_bridge_scale(nil), do: {:ok, 1.0}
  defp normalize_bridge_scale(scale) when is_number(scale), do: {:ok, scale}
  defp normalize_bridge_scale(_scale), do: {:error, :invalid_bridge_scale}

  defp normalize_contract_string(value, _reason)
       when is_binary(value) and value != "",
       do: {:ok, value}

  defp normalize_contract_string(_value, reason), do: {:error, reason}

  defp validate_bridge_distribution("element_to_nodes"), do: :ok
  defp validate_bridge_distribution(_distribution), do: {:error, :unsupported_bridge_distribution}

  defp validate_bridge_reduction("mean"), do: :ok
  defp validate_bridge_reduction("sum"), do: :ok
  defp validate_bridge_reduction(_reduction), do: {:error, :unsupported_bridge_reduction}

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
