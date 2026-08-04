defmodule KyuubikiWeb.AnalysisSolverSubmissions do
  @moduledoc false

  alias KyuubikiWeb.AnalysisJobSupport
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.FemModelNormalizer
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.ModelArtifactStore
  alias KyuubikiWeb.Playground.AgentClient

  @large_model_execution_timeout_ms 1_800_000
  @default_queue_timeout_ms 1_800_000

  def submit_axial_bar(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_axial_bar/1, "solve_bar_1d")

  def submit_acoustic_bar_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_acoustic_bar_1d/1,
        "solve_acoustic_bar_1d"
      )

  def submit_thermal_bar_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_thermal_bar_1d/1,
        "solve_thermal_bar_1d"
      )

  def submit_heat_bar_1d(params),
    do:
      submit_solver_job(params, &FemModelNormalizer.normalize_heat_bar_1d/1, "solve_heat_bar_1d")

  def submit_transient_heat_bar_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_transient_heat_bar_1d/1,
        "solve_transient_heat_bar_1d"
      )

  def submit_electrostatic_bar_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_electrostatic_bar_1d/1,
        "solve_electrostatic_bar_1d"
      )

  def submit_magnetostatic_bar_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_magnetostatic_bar_1d/1,
        "solve_magnetostatic_bar_1d"
      )

  def submit_electrostatic_plane_triangle_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_electrostatic_plane_triangle_2d/1,
        "solve_electrostatic_plane_triangle_2d"
      )

  def submit_electrostatic_plane_quad_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_electrostatic_plane_quad_2d/1,
        "solve_electrostatic_plane_quad_2d"
      )

  def submit_electric_conduction_plane_quad_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_electric_conduction_plane_quad_2d/1,
        "solve_electric_conduction_plane_quad_2d"
      )

  def submit_composite_thermo_electric_panel(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_composite_thermo_electric_panel/1,
        "solve_composite_thermo_electric_panel"
      )

  def submit_magnetostatic_plane_triangle_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_magnetostatic_plane_triangle_2d/1,
        "solve_magnetostatic_plane_triangle_2d"
      )

  def submit_magnetostatic_plane_quad_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_magnetostatic_plane_quad_2d/1,
        "solve_magnetostatic_plane_quad_2d"
      )

  def submit_heat_plane_triangle_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_heat_plane_triangle_2d/1,
        "solve_heat_plane_triangle_2d"
      )

  def submit_heat_plane_quad_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_heat_plane_quad_2d/1,
        "solve_heat_plane_quad_2d"
      )

  def submit_stokes_flow_plane_quad_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_stokes_flow_plane_quad_2d/1,
        "solve_stokes_flow_plane_quad_2d"
      )

  def submit_stokes_flow_plane_triangle_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_stokes_flow_plane_triangle_2d/1,
        "solve_stokes_flow_plane_triangle_2d"
      )

  def submit_thermal_truss_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_thermal_truss_2d/1,
        "solve_thermal_truss_2d"
      )

  def submit_thermal_truss_3d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_thermal_truss_3d/1,
        "solve_thermal_truss_3d"
      )

  def submit_beam_1d(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_beam_1d/1, "solve_beam_1d")

  def submit_thermal_beam_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_thermal_beam_1d/1,
        "solve_thermal_beam_1d"
      )

  def submit_torsion_1d(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_torsion_1d/1, "solve_torsion_1d")

  def submit_spring_1d(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_spring_1d/1, "solve_spring_1d")

  def submit_transient_spring_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_transient_spring_1d/1,
        "solve_transient_spring_1d"
      )

  def submit_harmonic_spring_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_harmonic_spring_1d/1,
        "solve_harmonic_spring_1d"
      )

  def submit_nonlinear_spring_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_nonlinear_spring_1d/1,
        "solve_nonlinear_spring_1d"
      )

  def submit_contact_gap_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_contact_gap_1d/1,
        "solve_contact_gap_1d"
      )

  def submit_cohesive_interface_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_cohesive_interface_1d/1,
        "solve_cohesive_interface_1d"
      )

  def submit_cohesive_interface_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_cohesive_interface_2d/1,
        "solve_cohesive_interface_2d"
      )

  def submit_cohesive_interface_mesh_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_cohesive_interface_mesh_2d/1,
        "solve_cohesive_interface_mesh_2d"
      )

  def submit_spring_2d(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_spring_2d/1, "solve_spring_2d")

  def submit_spring_3d(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_spring_3d/1, "solve_spring_3d")

  def submit_truss_2d(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_truss_2d/1, "solve_truss_2d")

  def submit_truss_3d(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_truss_3d/1, "solve_truss_3d")

  def submit_plane_triangle_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_plane_triangle_2d/1,
        "solve_plane_triangle_2d"
      )

  def submit_thermal_plane_triangle_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_thermal_plane_triangle_2d/1,
        "solve_thermal_plane_triangle_2d"
      )

  def submit_plane_quad_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_plane_quad_2d/1,
        "solve_plane_quad_2d"
      )

  def submit_thermal_plane_quad_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_thermal_plane_quad_2d/1,
        "solve_thermal_plane_quad_2d"
      )

  def submit_frame_2d(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_frame_2d/1, "solve_frame_2d")

  def submit_modal_frame_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_modal_frame_2d/1,
        "solve_modal_frame_2d"
      )

  def submit_buckling_beam_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_buckling_beam_1d/1,
        "solve_buckling_beam_1d"
      )

  def submit_buckling_frame_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_buckling_frame_2d/1,
        "solve_buckling_frame_2d"
      )

  def submit_frame_2d_p_delta(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_frame_2d_p_delta/1,
        "solve_frame_2d_p_delta"
      )

  def submit_frame_2d_material_p_delta(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_frame_2d_material_p_delta/1,
        "solve_frame_2d_material_p_delta"
      )

  def submit_frame_3d(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_frame_3d/1, "solve_frame_3d")

  def submit_solid_tetra_3d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_solid_tetra_3d/1,
        "solve_solid_tetra_3d"
      )

  def submit_modal_frame_3d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_modal_frame_3d/1,
        "solve_modal_frame_3d"
      )

  def submit_thermal_frame_2d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_thermal_frame_2d/1,
        "solve_thermal_frame_2d"
      )

  def submit_thermal_frame_3d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_thermal_frame_3d/1,
        "solve_thermal_frame_3d"
      )

  defp submit_solver_job(params, normalizer, method)
       when is_map(params) and is_function(normalizer, 1) and is_binary(method) do
    with {:ok, normalized} <- prepare_solver_params(params, normalizer),
         {:ok, job_context} <- AnalysisJobSupport.derive_job_context(params),
         timeout_policy = solver_timeout_policy(normalized),
         {:ok, job} <-
           AnalysisJobSupport.create_job(Map.merge(job_context, timeout_policy)) do
      start_background_job(
        job.job_id,
        method,
        normalized,
        orchestration_context_from_params(params),
        timeout_policy
      )

      {:ok, AnalysisJobSupport.serialize_payload(job)}
    end
  end

  defp prepare_solver_params(params, normalizer) do
    case ModelArtifactStore.prepare_agent_params(params) do
      {:ok, artifact_params} -> {:ok, artifact_params}
      {:inline, inline_params} -> normalizer.(inline_params)
      {:error, _reason} = error -> error
    end
  end

  defp start_background_job(job_id, method, params, orchestration_context, timeout_policy) do
    Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
      execute_background_job(job_id, method, params, orchestration_context, timeout_policy)
    end)
  end

  defp execute_background_job(job_id, method, params, orchestration_context, timeout_policy) do
    queue_timeout_ms = timeout_policy.queue_timeout_ms
    execution_timeout_ms = timeout_policy.execution_timeout_ms
    total_timeout_ms = queue_timeout_ms + execution_timeout_ms

    task =
      Task.async(fn ->
        AgentClient.request_with_agent(method, params, &apply_agent_progress(job_id, &1),
          orchestration: orchestration_context,
          job_id: job_id,
          before_dispatch: fn -> dispatch_guard(job_id) end,
          queue_timeout_ms: queue_timeout_ms,
          request_timeout_ms: execution_timeout_ms
        )
      end)

    case Task.yield(task, total_timeout_ms) || Task.shutdown(task, :brutal_kill) do
      {:ok, {:ok, result, endpoint}} ->
        unless terminal?(job_id) do
          _ = Store.assign_worker(job_id, AgentClient.worker_id(endpoint))
          :ok = AnalysisResultStore.put(job_id, result)
          _ = Store.apply_progress(%{job_id: job_id, stage: "completed", progress: 1.0})
        end

      {:ok, {:error, {:rpc_error, "cancelled", message}}} ->
        cancel_job_with_message(job_id, message)

      {:ok, {:error, reason}} ->
        unless terminal?(job_id), do: fail_job(job_id, inspect(reason))

      nil ->
        request_agent_cancel(job_id)

        unless terminal?(job_id),
          do:
            fail_job(
              job_id,
              "job dispatch exceeded total server budget; " <>
                "queue_timeout_ms=#{queue_timeout_ms}; " <>
                "execution_timeout_ms=#{execution_timeout_ms}; " <>
                "total_timeout_ms=#{total_timeout_ms}"
            )
    end
  end

  defp request_agent_cancel(job_id) do
    Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
      _ = AgentClient.cancel_job(job_id)
    end)

    :ok
  end

  defp apply_agent_progress(job_id, progress) when is_binary(job_id) and is_map(progress) do
    case Store.get(job_id) do
      {:ok, %{status: status}} when status in [:completed, :failed, :cancelled] -> :ok
      {:ok, _job} -> apply_running_progress(job_id, progress)
      :error -> :ok
    end
  end

  defp apply_running_progress(job_id, progress) do
    attrs =
      progress
      |> Map.take(["stage", "progress", "residual", "iteration", "peak_memory", "message"])
      |> Enum.into(%{}, fn {key, value} -> {String.to_atom(key), value} end)
      |> Map.put(:job_id, job_id)

    _ = Store.apply_progress(attrs)
    :ok
  end

  defp orchestration_context_from_params(params) when is_map(params) do
    normalized = AnalysisJobSupport.stringify_keys(params)

    %{}
    |> maybe_put_orchestration_value("control_mode", Map.get(normalized, "control_mode"))
    |> maybe_put_orchestration_value("orch_id", Map.get(normalized, "orch_id"))
    |> maybe_put_orchestration_value("orch_session_id", Map.get(normalized, "orch_session_id"))
    |> maybe_put_orchestration_value("cluster_id", Map.get(normalized, "cluster_id"))
  end

  defp maybe_put_orchestration_value(context, _key, nil), do: context

  defp maybe_put_orchestration_value(context, key, value) when is_binary(value) and value != "",
    do: Map.put(context, key, value)

  defp maybe_put_orchestration_value(context, _key, _value), do: context

  defp fail_job(job_id, message) do
    _ = Store.apply_progress(%{job_id: job_id, stage: "failed", progress: 1.0, message: message})
    :ok
  end

  defp cancel_job_with_message(job_id, message) do
    _ =
      Store.apply_progress(%{job_id: job_id, stage: "cancelled", progress: 1.0, message: message})

    :ok
  end

  defp terminal?(job_id) do
    match?(
      {:ok, %{status: status}} when status in [:completed, :failed, :cancelled],
      Store.get(job_id)
    )
  end

  defp dispatch_guard(job_id) do
    if terminal?(job_id),
      do: {:error, {:rpc_error, "cancelled", "job became terminal before agent dispatch"}},
      else: :ok
  end

  defp solver_timeout_policy(params) do
    %{
      queue_timeout_ms: queue_timeout_ms(),
      execution_timeout_ms: solver_execution_timeout_ms(params)
    }
  end

  defp solver_execution_timeout_ms(%{"model_artifact_ref" => _reference}) do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get(:artifact_execution_timeout_ms, @large_model_execution_timeout_ms)
  end

  defp solver_execution_timeout_ms(_params) do
    Application.get_env(:kyuubiki_web, AgentClient, [])
    |> Keyword.get(:request_timeout_ms, 120_000)
  end

  defp queue_timeout_ms do
    Application.get_env(:kyuubiki_web, AgentClient, [])
    |> Keyword.get(:queue_timeout_ms, @default_queue_timeout_ms)
  end
end
