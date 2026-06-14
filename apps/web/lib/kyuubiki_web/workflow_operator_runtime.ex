defmodule KyuubikiWeb.WorkflowOperatorRuntime do
  @moduledoc false

  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.WorkflowOperatorBridgeRuntime
  alias KyuubikiWeb.WorkflowOperatorHeatBridgeRuntime
  alias KyuubikiWeb.WorkflowReportingRuntime
  alias KyuubikiWeb.WorkflowSolverRegistry
  alias KyuubikiWeb.WorkflowThermalRuntime

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
      WorkflowOperatorHeatBridgeRuntime.bridge_heat_result_to_thermal_plane_quad_model(
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
      WorkflowOperatorHeatBridgeRuntime.bridge_heat_result_to_thermal_plane_triangle_model(
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
    WorkflowOperatorHeatBridgeRuntime.bridge_heat_result_to_thermal_plane_quad_model(
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
    WorkflowOperatorHeatBridgeRuntime.bridge_heat_result_to_thermal_plane_triangle_model(
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

  def run_transform_operator(operator_id, payload, config) when is_map(payload) do
    case operator_id do
      "transform.merge_summary_pair" ->
        WorkflowReportingRuntime.merge_summary_pair(payload, config || %{})

      "transform.compare_summary_pair" ->
        WorkflowReportingRuntime.compare_summary_pair(payload, config || %{})

      "transform.aggregate_summary_collection" ->
        WorkflowReportingRuntime.aggregate_summary_collection(payload, config || %{})

      "transform.normalize_summary_fields" when is_map(config) ->
        WorkflowReportingRuntime.normalize_summary_fields(payload, config)

      "transform.select_best_summary" when is_map(config) ->
        WorkflowReportingRuntime.select_best_summary(payload, config)

      "transform.evaluate_thermal_guard" when is_map(config) ->
        WorkflowThermalRuntime.evaluate_thermal_guard(payload, config)

      "transform.benchmark_coupled_heat_pair" when is_map(config) ->
        WorkflowThermalRuntime.benchmark_coupled_heat_pair(payload, config)

      _ ->
        {:error, {:unsupported_workflow_transform_operator, operator_id}}
    end
  end

  def run_transform_operator(operator_id, _payload, _config),
    do: {:error, {:unsupported_workflow_transform_operator, operator_id}}

  def run_extract_operator(operator_id, payload, config) when is_map(payload) do
    case operator_id do
      "extract.result_summary" ->
        WorkflowReportingRuntime.extract_result_summary(payload, config || %{})

      "extract.field_statistics" ->
        WorkflowReportingRuntime.extract_field_statistics(payload, config || %{})

      "extract.field_hotspots" ->
        WorkflowReportingRuntime.extract_field_hotspots(payload, config || %{})

      "extract.thermal_result_diagnostics" ->
        WorkflowReportingRuntime.extract_thermal_result_diagnostics(payload, config || %{})

      "extract.thermo_result_diagnostics" ->
        WorkflowReportingRuntime.extract_thermo_result_diagnostics(payload, config || %{})

      _ ->
        {:error, {:unsupported_workflow_extract_operator, operator_id}}
    end
  end

  def run_extract_operator(operator_id, _payload, _config),
    do: {:error, {:unsupported_workflow_extract_operator, operator_id}}

  def run_export_operator(operator_id, payload, config) when is_map(payload) do
    case operator_id do
      "export.summary_json" ->
        WorkflowReportingRuntime.export_summary_json(payload)

      "export.summary_csv" ->
        WorkflowReportingRuntime.export_summary_csv(payload, config || %{})

      "export.alert_markdown" ->
        WorkflowReportingRuntime.export_alert_markdown(payload, config || %{})

      _ ->
        {:error, {:unsupported_workflow_export_operator, operator_id}}
    end
  end

  def run_export_operator(operator_id, _payload, _config),
    do: {:error, {:unsupported_workflow_export_operator, operator_id}}

  defp solve_runtime_client do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get(:solve_runtime_client, AgentClient)
  end

  defp resolve_electrostatic_to_heat_bridge_contract(config),
    do: WorkflowOperatorBridgeRuntime.resolve_electrostatic_to_heat_bridge_contract(config)

  defp resolve_heat_to_thermo_bridge_contract(config),
    do: WorkflowOperatorHeatBridgeRuntime.resolve_heat_to_thermo_bridge_contract(config)
end
