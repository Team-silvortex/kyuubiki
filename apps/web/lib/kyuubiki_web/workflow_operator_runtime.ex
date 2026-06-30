defmodule KyuubikiWeb.WorkflowOperatorRuntime do
  @moduledoc false

  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.WorkflowBridgeIntegrityRuntime
  alias KyuubikiWeb.WorkflowBundleRuntime
  alias KyuubikiWeb.WorkflowCfdRuntime
  alias KyuubikiWeb.WorkflowElectrostaticRuntime
  alias KyuubikiWeb.WorkflowMaterialRuntime
  alias KyuubikiWeb.WorkflowOperatorBridgeRuntime
  alias KyuubikiWeb.WorkflowOperatorHeatBridgeRuntime
  alias KyuubikiWeb.WorkflowReportingRuntime
  alias KyuubikiWeb.WorkflowSolverRegistry
  alias KyuubikiWeb.WorkflowSummaryRuntime
  alias KyuubikiWeb.WorkflowThermalRuntime

  def run_solve_operator(operator_id, payload, node \\ %{})

  def run_solve_operator(operator_id, payload, node) when is_map(payload) and is_map(node) do
    case WorkflowSolverRegistry.fetch(operator_id) do
      {:ok, %{capability_tags: tags, method: method}} ->
        if roadmap_operator_tags?(tags) do
          {:error, {:roadmap_workflow_solve_operator, operator_id}}
        else
          dispatch_solve_operator(method, payload, node)
        end

      :error ->
        {:error, {:unsupported_workflow_solve_operator, operator_id}}
    end
  end

  def run_solve_operator(operator_id, _payload, _node),
    do: {:error, {:unsupported_workflow_solve_operator, operator_id}}

  defp roadmap_operator_tags?(tags) when is_list(tags),
    do: "partial" in tags or "roadmap" in tags

  defp roadmap_operator_tags?(_tags), do: false

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

  def run_transform_operator(
        "bridge.magnetostatic_field_to_heat_quad_2d",
        magnetostatic_result,
        %{"seed_model" => heat_seed_model} = config
      )
      when is_map(magnetostatic_result) and is_map(heat_seed_model) and is_map(config) do
    with {:ok, bridge_contract} <- resolve_magnetostatic_to_heat_bridge_contract(config) do
      WorkflowOperatorBridgeRuntime.bridge_magnetostatic_result_to_heat_plane_quad_model(
        magnetostatic_result,
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

      "transform.evaluate_material_margins" when is_map(config) ->
        WorkflowMaterialRuntime.evaluate_material_margins(payload, config)

      "transform.rank_material_candidates" when is_map(config) ->
        WorkflowMaterialRuntime.rank_material_candidates(payload, config)

      "transform.score_material_candidates" when is_map(config) ->
        WorkflowMaterialRuntime.score_material_candidates(payload, config)

      "transform.plan_material_experiments" when is_map(config) ->
        WorkflowMaterialRuntime.plan_material_experiments(payload, config)

      "transform.analyze_material_experiment_results" when is_map(config) ->
        WorkflowMaterialRuntime.analyze_material_experiment_results(payload, config)

      "transform.decide_material_iteration" when is_map(config) ->
        WorkflowMaterialRuntime.decide_material_iteration(payload, config)

      "transform.prepare_material_next_round_request" when is_map(config) ->
        WorkflowMaterialRuntime.prepare_material_next_round_request(payload, config)

      "transform.build_material_exploration_snapshot" when is_map(config) ->
        WorkflowMaterialRuntime.build_material_exploration_snapshot(payload, config)

      "transform.extract_material_pareto_frontier" when is_map(config) ->
        WorkflowMaterialRuntime.extract_material_pareto_frontier(payload, config)

      "transform.compose_diagnostics_bundle" when is_map(config) ->
        WorkflowSummaryRuntime.compose_diagnostics_bundle(payload, config)

      "transform.compose_diagnostics_report_payload" when is_map(config) ->
        WorkflowBundleRuntime.compose_diagnostics_report_payload(payload, config)

      "transform.select_focus_payload" when is_map(config) ->
        WorkflowBundleRuntime.select_focus_payload(payload, config)

      "transform.compose_focus_chain_input" when is_map(config) ->
        WorkflowBundleRuntime.compose_focus_chain_input(payload, config)

      "transform.compose_focus_bridge_request" when is_map(config) ->
        WorkflowBundleRuntime.compose_focus_bridge_request(payload, config)

      "transform.resolve_focus_bridge_execution" when is_map(config) ->
        WorkflowBundleRuntime.resolve_focus_bridge_execution(payload, config)

      "transform.execute_focus_bridge_execution" when is_map(config) ->
        execute_focus_bridge_execution(payload, config)

      "transform.evaluate_diagnostics_bundle_guard" when is_map(config) ->
        WorkflowSummaryRuntime.evaluate_diagnostics_bundle_guard(payload, config)

      "transform.evaluate_thermal_guard" when is_map(config) ->
        WorkflowThermalRuntime.evaluate_thermal_guard(payload, config)

      "transform.evaluate_electrostatic_guard" when is_map(config) ->
        WorkflowElectrostaticRuntime.evaluate_electrostatic_guard(payload, config)

      "transform.evaluate_magnetostatic_guard" when is_map(config) ->
        KyuubikiWeb.WorkflowMagnetostaticRuntime.evaluate_magnetostatic_guard(payload, config)

      "transform.evaluate_cfd_guard" when is_map(config) ->
        WorkflowCfdRuntime.evaluate_cfd_guard(payload, config)

      "transform.validate_electrostatic_heat_bridge" when is_map(config) ->
        WorkflowBridgeIntegrityRuntime.validate_electrostatic_heat_bridge(payload, config)

      "transform.benchmark_coupled_heat_pair" when is_map(config) ->
        WorkflowThermalRuntime.benchmark_coupled_heat_pair(payload, config)

      "transform.benchmark_electrostatic_pair" when is_map(config) ->
        WorkflowElectrostaticRuntime.benchmark_electrostatic_pair(payload, config)

      "transform.benchmark_magnetostatic_pair" when is_map(config) ->
        KyuubikiWeb.WorkflowMagnetostaticRuntime.benchmark_magnetostatic_pair(payload, config)

      "transform.benchmark_cfd_pair" when is_map(config) ->
        WorkflowCfdRuntime.benchmark_cfd_pair(payload, config)

      "transform.validate_heat_thermo_bridge" when is_map(config) ->
        WorkflowBridgeIntegrityRuntime.validate_heat_thermo_bridge(payload, config)

      _ ->
        {:error, {:unsupported_workflow_transform_operator, operator_id}}
    end
  end

  def run_transform_operator(operator_id, _payload, _config),
    do: {:error, {:unsupported_workflow_transform_operator, operator_id}}

  defp execute_focus_bridge_execution(payload, config) do
    with {:ok, execution} <- WorkflowBundleRuntime.resolve_focus_bridge_execution(payload, config),
         operator_id when is_binary(operator_id) <- Map.get(execution, "operator_id"),
         %{} = bridge_payload <- Map.get(execution, "bridge_payload"),
         %{} = bridge_config <- Map.get(execution, "bridge_config"),
         {:ok, bridge_result} <-
           run_transform_operator(operator_id, bridge_payload, bridge_config) do
      {:ok,
       %{
         "result_contract" => "kyuubiki.workflow_focus_bridge_result/v1",
         "result_kind" => "focus_bridge_result",
         "operator_id" => operator_id,
         "bridge_result" => bridge_result,
         "execution_payload" => execution,
         "metric_id" => Map.get(execution, "metric_id"),
         "focus_value" => Map.get(execution, "focus_value"),
         "bridge_payload_source" => Map.get(execution, "bridge_payload_source")
       }}
    else
      {:error, reason} -> {:error, reason}
      _ -> {:error, :invalid_focus_bridge_execution}
    end
  end

  def run_extract_operator(operator_id, payload, config) when is_map(payload) do
    case operator_id do
      "extract.result_summary" ->
        WorkflowReportingRuntime.extract_result_summary(payload, config || %{})

      "extract.field_statistics" ->
        WorkflowReportingRuntime.extract_field_statistics(payload, config || %{})

      "extract.field_hotspots" ->
        WorkflowReportingRuntime.extract_field_hotspots(payload, config || %{})

      "extract.electrostatic_result_diagnostics" ->
        WorkflowElectrostaticRuntime.extract_electrostatic_result_diagnostics(
          payload,
          config || %{}
        )

      "extract.electrostatic_peak_field" ->
        WorkflowReportingRuntime.extract_electrostatic_peak_field(payload, config || %{})

      "extract.magnetostatic_result_diagnostics" ->
        KyuubikiWeb.WorkflowMagnetostaticRuntime.extract_magnetostatic_result_diagnostics(
          payload,
          config || %{}
        )

      "extract.magnetostatic_peak_field" ->
        KyuubikiWeb.WorkflowPeakRuntime.extract_magnetostatic_peak_field(payload, config || %{})

      "extract.stokes_flow_result_diagnostics" ->
        WorkflowCfdRuntime.extract_stokes_flow_result_diagnostics(payload, config || %{})

      "extract.thermal_result_diagnostics" ->
        WorkflowReportingRuntime.extract_thermal_result_diagnostics(payload, config || %{})

      "extract.heat_peak_flux" ->
        WorkflowReportingRuntime.extract_heat_peak_flux(payload, config || %{})

      "extract.thermo_result_diagnostics" ->
        WorkflowReportingRuntime.extract_thermo_result_diagnostics(payload, config || %{})

      "extract.thermo_peak_response" ->
        WorkflowReportingRuntime.extract_thermo_peak_response(payload, config || %{})

      "extract.bridge_integrity_diagnostics" ->
        WorkflowBridgeIntegrityRuntime.extract_bridge_integrity_diagnostics(
          payload,
          config || %{}
        )

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

      "export.diagnostics_bundle_markdown" ->
        WorkflowReportingRuntime.export_diagnostics_bundle_markdown(payload, config || %{})

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

  defp dispatch_solve_operator(method, payload, node) when is_atom(method) do
    client = solve_runtime_client()
    routing_opts = solve_routing_opts(node)
    _ = Code.ensure_loaded(client)

    cond do
      function_exported?(client, :request, 4) ->
        client.request(Atom.to_string(method), payload, fn _progress -> :ok end, routing_opts)

      function_exported?(client, method, 1) ->
        apply(client, method, [payload])

      true ->
        {:error, {:unsupported_workflow_solve_method, Atom.to_string(method)}}
    end
  end

  defp solve_routing_opts(node) do
    [
      required_capabilities:
        node
        |> Map.get("required_capabilities", [])
        |> normalize_routing_values(),
      placement_tags:
        node
        |> Map.get("placement_tags", [])
        |> normalize_routing_values(),
      orchestration:
        node
        |> Map.get("orchestration_context", %{})
        |> normalize_orchestration_context()
    ]
  end

  defp normalize_routing_values(values) when is_list(values) do
    values
    |> Enum.filter(&is_binary/1)
    |> Enum.reject(&(&1 == ""))
    |> Enum.uniq()
  end

  defp normalize_routing_values(_values), do: []

  defp normalize_orchestration_context(%{} = context) do
    %{}
    |> put_orchestration_value(:control_mode, Map.get(context, "control_mode"))
    |> put_orchestration_value(:orch_id, Map.get(context, "orch_id"))
    |> put_orchestration_value(:orch_session_id, Map.get(context, "orch_session_id"))
    |> put_orchestration_value(:cluster_id, Map.get(context, "cluster_id"))
  end

  defp normalize_orchestration_context(_context), do: %{}

  defp put_orchestration_value(context, _key, nil), do: context

  defp put_orchestration_value(context, key, value) when is_binary(value),
    do: Map.put(context, key, value)

  defp put_orchestration_value(context, _key, _value), do: context

  defp resolve_electrostatic_to_heat_bridge_contract(config),
    do: WorkflowOperatorBridgeRuntime.resolve_electrostatic_to_heat_bridge_contract(config)

  defp resolve_magnetostatic_to_heat_bridge_contract(config),
    do: WorkflowOperatorBridgeRuntime.resolve_magnetostatic_to_heat_bridge_contract(config)

  defp resolve_heat_to_thermo_bridge_contract(config),
    do: WorkflowOperatorHeatBridgeRuntime.resolve_heat_to_thermo_bridge_contract(config)
end
