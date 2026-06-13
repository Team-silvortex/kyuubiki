defmodule KyuubikiWeb.WorkflowOperatorRuntime do
  @moduledoc false

  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.WorkflowOperatorBridgeRuntime
  alias KyuubikiWeb.WorkflowSolverRegistry

  def run_solve_operator(operator_id, payload) when is_map(payload) do
    case WorkflowSolverRegistry.fetch(operator_id) do
      {:ok, %{method: method}} -> apply(solve_runtime_client(), method, [payload])
      :error -> {:error, {:unsupported_workflow_solve_operator, operator_id}}
    end
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
      WorkflowOperatorBridgeRuntime.bridge_heat_result_to_thermal_plane_quad_model(
        heat_result,
        thermo_seed_model,
        bridge_contract
      )
    end
  end

  def run_transform_operator(
        "bridge.temperature_field_to_thermo_triangle_2d",
        heat_result,
        %{"seed_model" => thermo_seed_model} = config
      )
      when is_map(heat_result) and is_map(thermo_seed_model) and is_map(config) do
    with {:ok, bridge_contract} <- resolve_heat_to_thermo_bridge_contract(config) do
      WorkflowOperatorBridgeRuntime.bridge_heat_result_to_thermal_plane_triangle_model(
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
    WorkflowOperatorBridgeRuntime.bridge_heat_result_to_thermal_plane_quad_model(
      heat_result,
      thermo_seed_model
    )
  end

  def run_transform_operator(
        "bridge.temperature_field_to_thermo_triangle_2d",
        heat_result,
        thermo_seed_model
      )
      when is_map(heat_result) and is_map(thermo_seed_model) do
    WorkflowOperatorBridgeRuntime.bridge_heat_result_to_thermal_plane_triangle_model(
      heat_result,
      thermo_seed_model
    )
  end

  def run_transform_operator(
        "bridge.electrostatic_field_to_heat_quad_2d",
        electrostatic_result,
        %{"seed_model" => heat_seed_model} = config
      )
      when is_map(electrostatic_result) and is_map(heat_seed_model) and is_map(config) do
    with {:ok, bridge_contract} <- resolve_electrostatic_to_heat_bridge_contract(config) do
      WorkflowOperatorBridgeRuntime.bridge_electrostatic_result_to_heat_plane_quad_model(
        electrostatic_result,
        heat_seed_model,
        bridge_contract
      )
    end
  end

  def run_transform_operator(
        "bridge.electrostatic_field_to_heat_triangle_2d",
        electrostatic_result,
        %{"seed_model" => heat_seed_model} = config
      )
      when is_map(electrostatic_result) and is_map(heat_seed_model) and is_map(config) do
    with {:ok, bridge_contract} <- resolve_electrostatic_to_heat_bridge_contract(config) do
      WorkflowOperatorBridgeRuntime.bridge_electrostatic_result_to_heat_plane_triangle_model(
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

  defp solve_runtime_client do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get(:solve_runtime_client, AgentClient)
  end

  defp resolve_electrostatic_to_heat_bridge_contract(config),
    do: WorkflowOperatorBridgeRuntime.resolve_electrostatic_to_heat_bridge_contract(config)

  defp resolve_heat_to_thermo_bridge_contract(config),
    do: WorkflowOperatorBridgeRuntime.resolve_heat_to_thermo_bridge_contract(config)
end
