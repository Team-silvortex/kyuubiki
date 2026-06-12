defmodule KyuubikiWeb.WorkflowOperatorRuntime do
  @moduledoc false

  alias KyuubikiWeb.Playground.AgentClient

  def run_solve_operator("solve.electrostatic_plane_quad_2d", payload) when is_map(payload) do
    AgentClient.solve_electrostatic_plane_quad_2d(payload)
  end

  def run_solve_operator("solve.heat_plane_quad_2d", payload) when is_map(payload) do
    AgentClient.solve_heat_plane_quad_2d(payload)
  end

  def run_solve_operator("solve.thermal_plane_quad_2d", payload) when is_map(payload) do
    AgentClient.solve_thermal_plane_quad_2d(payload)
  end

  def run_solve_operator(operator_id, _payload),
    do: {:error, {:unsupported_workflow_solve_operator, operator_id}}

  def run_transform_operator(
        "bridge.temperature_field_to_thermo_quad_2d",
        heat_result,
        %{"seed_model" => thermo_seed_model} = config
      )
      when is_map(heat_result) and is_map(thermo_seed_model) and is_map(config) do
    with {:ok, bridge_contract} <- resolve_heat_to_thermo_bridge_contract(config) do
      bridge_heat_result_to_thermal_plane_quad_model(
        heat_result,
        thermo_seed_model,
        bridge_contract
      )
    end
  end

  def run_transform_operator(
        "bridge.temperature_field_to_thermo_quad_2d",
        heat_result,
        thermo_seed_model
      )
      when is_map(heat_result) and is_map(thermo_seed_model) do
    bridge_heat_result_to_thermal_plane_quad_model(heat_result, thermo_seed_model)
  end

  def run_transform_operator(
        "bridge.electrostatic_field_to_heat_quad_2d",
        electrostatic_result,
        %{"seed_model" => heat_seed_model} = config
      )
      when is_map(electrostatic_result) and is_map(heat_seed_model) and is_map(config) do
    with {:ok, bridge_contract} <- resolve_electrostatic_to_heat_bridge_contract(config) do
      bridge_electrostatic_result_to_heat_plane_quad_model(
        electrostatic_result,
        heat_seed_model,
        bridge_contract
      )
    end
  end

  def run_transform_operator("transform.first_available", payload, _config), do: {:ok, payload}

  def run_transform_operator(operator_id, _payload, _config),
    do: {:error, {:unsupported_workflow_transform_operator, operator_id}}

  def run_extract_operator("extract.result_summary", payload, config) when is_map(payload) do
    extract_result_summary(payload, config || %{})
  end

  def run_extract_operator(operator_id, _payload, _config),
    do: {:error, {:unsupported_workflow_extract_operator, operator_id}}

  def run_export_operator("export.summary_json", payload, _config) when is_map(payload) do
    export_summary_json(payload)
  end

  def run_export_operator("export.summary_csv", payload, config) when is_map(payload) do
    export_summary_csv(payload, config || %{})
  end

  def run_export_operator(operator_id, _payload, _config),
    do: {:error, {:unsupported_workflow_export_operator, operator_id}}

  defp extract_result_summary(payload, config) when is_map(payload) and is_map(config) do
    requested_fields =
      case Map.get(config, "fields") do
        fields when is_list(fields) -> Enum.filter(fields, &is_binary/1)
        _ -> nil
      end

    summary =
      cond do
        is_list(requested_fields) ->
          Enum.reduce(requested_fields, %{}, fn field, acc ->
            case Map.fetch(payload, field) do
              {:ok, value} -> Map.put(acc, field, value)
              :error -> acc
            end
          end)

        true ->
          payload
          |> Enum.filter(fn {key, _value} -> String.starts_with?(key, "max_") end)
          |> Map.new()
      end

    if map_size(summary) == 0 do
      {:error, :empty_summary}
    else
      {:ok, summary}
    end
  end

  defp export_summary_json(payload) when is_map(payload) do
    {:ok,
     %{
       "format" => "json",
       "content_type" => "application/json",
       "content" => Jason.encode!(payload)
     }}
  end

  defp export_summary_csv(payload, config) when is_map(payload) and is_map(config) do
    requested_fields =
      case Map.get(config, "fields") do
        fields when is_list(fields) -> Enum.filter(fields, &is_binary/1)
        _ -> nil
      end

    rows =
      if is_list(requested_fields) do
        Enum.reduce(requested_fields, [["key", "value"]], fn field, acc ->
          case Map.fetch(payload, field) do
            {:ok, value} -> acc ++ [[field, value]]
            :error -> acc
          end
        end)
      else
        [["key", "value"]] ++ Enum.map(payload, fn {key, value} -> [key, value] end)
      end

    if length(rows) == 1 do
      {:error, :empty_export}
    else
      content =
        rows
        |> Enum.map_join("\n", fn row -> Enum.map_join(row, ",", &csv_escape/1) end)
        |> Kernel.<>("\n")

      {:ok,
       %{
         "format" => "csv",
         "content_type" => "text/csv",
         "content" => content
       }}
    end
  end

  defp bridge_heat_result_to_thermal_plane_quad_model(
         %{"nodes" => heat_nodes, "input" => %{"elements" => heat_elements}},
         %{"nodes" => thermo_nodes, "elements" => thermo_elements} = thermo_seed_model
       )
       when is_list(heat_nodes) and is_list(heat_elements) and is_list(thermo_nodes) and
              is_list(thermo_elements) do
    cond do
      length(heat_nodes) != length(thermo_nodes) -> {:error, :node_count_mismatch}
      length(heat_elements) != length(thermo_elements) -> {:error, :element_count_mismatch}
      true -> bridge_heat_nodes(heat_nodes, thermo_nodes, thermo_seed_model, "temperature_delta")
    end
  end

  defp bridge_heat_result_to_thermal_plane_quad_model(_heat_result, _thermo_seed_model),
    do: {:error, :invalid_bridge_payload}

  defp bridge_heat_result_to_thermal_plane_quad_model(
         %{"nodes" => heat_nodes, "input" => %{"elements" => heat_elements}},
         %{"nodes" => thermo_nodes, "elements" => thermo_elements} = thermo_seed_model,
         bridge_contract
       )
       when is_list(heat_nodes) and is_list(heat_elements) and is_list(thermo_nodes) and
              is_list(thermo_elements) do
    cond do
      length(heat_nodes) != length(thermo_nodes) ->
        {:error, :node_count_mismatch}

      length(heat_elements) != length(thermo_elements) ->
        {:error, :element_count_mismatch}

      true ->
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
                       Map.get(
                         heat_node,
                         bridge_contract.source_field,
                         bridge_contract.default_value
                       )
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
  end

  defp bridge_heat_result_to_thermal_plane_quad_model(
         _heat_result,
         _thermo_seed_model,
         _bridge_contract
       ),
       do: {:error, :invalid_bridge_payload}

  defp bridge_electrostatic_result_to_heat_plane_quad_model(
         %{"nodes" => electrostatic_nodes, "elements" => electrostatic_elements},
         %{"nodes" => heat_nodes, "elements" => heat_elements} = heat_seed_model,
         bridge_contract
       )
       when is_list(electrostatic_nodes) and is_list(electrostatic_elements) and
              is_list(heat_nodes) and is_list(heat_elements) do
    cond do
      length(electrostatic_nodes) != length(heat_nodes) ->
        {:error, :node_count_mismatch}

      true ->
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

  defp bridge_electrostatic_result_to_heat_plane_quad_model(
         _result,
         _seed_model,
         _bridge_contract
       ),
       do: {:error, :invalid_bridge_payload}

  defp resolve_electrostatic_to_heat_bridge_contract(config) when is_map(config) do
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

  defp resolve_electrostatic_to_heat_bridge_contract(_config),
    do: {:error, :invalid_bridge_contract}

  defp resolve_heat_to_thermo_bridge_contract(config) when is_map(config) do
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

  defp resolve_heat_to_thermo_bridge_contract(_config),
    do: {:error, :invalid_bridge_contract}

  defp bridge_heat_nodes(heat_nodes, thermo_nodes, thermo_seed_model, target_field) do
    bridged_nodes =
      Enum.zip(heat_nodes, thermo_nodes)
      |> Enum.reduce_while([], fn {heat_node, thermo_node}, acc ->
        if close_enough?(Map.get(heat_node, "x"), Map.get(thermo_node, "x")) and
             close_enough?(Map.get(heat_node, "y"), Map.get(thermo_node, "y")) do
          {:cont, acc ++ [Map.put(thermo_node, target_field, Map.get(heat_node, "temperature"))]}
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

  defp normalize_numeric_value(value) when is_number(value), do: value
  defp normalize_numeric_value(_value), do: 0.0

  defp close_enough?(left, right) when is_number(left) and is_number(right),
    do: abs(left - right) <= 1.0e-9

  defp close_enough?(_, _), do: false

  defp csv_escape(nil), do: ""

  defp csv_escape(value) when is_binary(value) do
    escaped = String.replace(value, "\"", "\"\"")

    if String.contains?(escaped, [",", "\"", "\n", "\r"]) do
      ~s("#{escaped}")
    else
      escaped
    end
  end

  defp csv_escape(value), do: value |> to_string() |> csv_escape()
end
