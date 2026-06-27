defmodule KyuubikiWeb.AnalysisSolverSubmissions do
  @moduledoc false

  alias KyuubikiWeb.AnalysisJobSupport
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.FemModelNormalizer
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Playground.AgentClient

  def submit_axial_bar(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_axial_bar/1, "solve_bar_1d")

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

  def submit_electrostatic_bar_1d(params),
    do:
      submit_solver_job(
        params,
        &FemModelNormalizer.normalize_electrostatic_bar_1d/1,
        "solve_electrostatic_bar_1d"
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

  def submit_frame_3d(params),
    do: submit_solver_job(params, &FemModelNormalizer.normalize_frame_3d/1, "solve_frame_3d")

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
    with {:ok, normalized} <- normalizer.(params),
         {:ok, job_context} <- AnalysisJobSupport.derive_job_context(params),
         {:ok, job} <- AnalysisJobSupport.create_job(job_context) do
      start_background_job(
        job.job_id,
        method,
        normalized,
        orchestration_context_from_params(params)
      )

      {:ok, AnalysisJobSupport.serialize_payload(job)}
    end
  end

  defp start_background_job(job_id, method, params, orchestration_context) do
    Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
      execute_background_job(job_id, method, params, orchestration_context)
    end)
  end

  defp execute_background_job(job_id, method, params, orchestration_context) do
    timeout_ms = watchdog_job_timeout_ms()

    task =
      Task.async(fn ->
        AgentClient.request_with_agent(method, params, &apply_agent_progress(job_id, &1),
          orchestration: orchestration_context,
          job_id: job_id
        )
      end)

    case Task.yield(task, timeout_ms) || Task.shutdown(task, :brutal_kill) do
      {:ok, {:ok, result, endpoint}} ->
        unless cancelled?(job_id) do
          {:ok, _job} = Store.assign_worker(job_id, AgentClient.worker_id(endpoint))
          :ok = AnalysisResultStore.put(job_id, result)
          _ = Store.apply_progress(%{job_id: job_id, stage: "completed", progress: 1.0})
        end

      {:ok, {:error, {:rpc_error, "cancelled", message}}} ->
        cancel_job_with_message(job_id, message)

      {:ok, {:error, reason}} ->
        unless cancelled?(job_id), do: fail_job(job_id, inspect(reason))

      nil ->
        unless cancelled?(job_id),
          do: fail_job(job_id, "job execution timed out after #{timeout_ms} ms")
    end
  end

  defp apply_agent_progress(job_id, progress) when is_binary(job_id) and is_map(progress) do
    case Store.get(job_id) do
      {:ok, %{status: :cancelled}} -> :ok
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

  defp cancelled?(job_id), do: match?({:ok, %{status: :cancelled}}, Store.get(job_id))

  defp watchdog_job_timeout_ms do
    Application.get_env(:kyuubiki_web, KyuubikiWeb.Jobs.Watchdog, [])
    |> Keyword.get(:job_timeout_ms, 120_000)
  end
end
