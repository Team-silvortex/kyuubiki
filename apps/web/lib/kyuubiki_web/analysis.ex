defmodule KyuubikiWeb.Analysis do
  @moduledoc """
  Asynchronous orchestration for FEM study jobs.
  """

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Playground.AgentClient

  @worker_id "rust-agent-rpc"

  @spec submit_axial_bar(map()) :: {:ok, map()} | {:error, term()}
  def submit_axial_bar(params) when is_map(params) do
    with {:ok, normalized} <- normalize_axial_bar(params),
         {:ok, job} <- create_job() do
      start_background_job(job.job_id, "solve_bar_1d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_truss_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_truss_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_truss_2d(params),
         {:ok, job} <- create_job() do
      start_background_job(job.job_id, "solve_truss_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_plane_triangle_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_plane_triangle_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_plane_triangle_2d(params),
         {:ok, job} <- create_job() do
      start_background_job(job.job_id, "solve_plane_triangle_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec fetch_job(String.t()) :: {:ok, map()} | {:error, term()}
  def fetch_job(job_id) when is_binary(job_id) do
    case Store.get(job_id) do
      {:ok, job} ->
        payload = serialize_payload(job)

        case AnalysisResultStore.get(job_id) do
          {:ok, result} ->
            {:ok, payload |> put_has_result(true) |> Map.put("result", result)}

          :error ->
            {:ok, payload}
        end

      :error ->
        {:error, {:job_not_found, job_id}}
    end
  end

  @spec list_jobs() :: map()
  def list_jobs do
    jobs =
      Store.list()
      |> Enum.map(fn job ->
        has_result? = match?({:ok, _result}, AnalysisResultStore.get(job.job_id))

        job
        |> serialize_job()
        |> Map.put("has_result", has_result?)
      end)

    %{"jobs" => jobs}
  end

  defp start_background_job(job_id, method, params) do
    Task.start(fn ->
      {:ok, _job} = Store.assign_worker(job_id, @worker_id)

      solver_call =
        case method do
          "solve_bar_1d" -> &AgentClient.solve_bar_1d/2
          "solve_truss_2d" -> &AgentClient.solve_truss_2d/2
          "solve_plane_triangle_2d" -> &AgentClient.solve_plane_triangle_2d/2
        end

      case solver_call.(params, &apply_agent_progress(job_id, &1)) do
        {:ok, result} ->
          :ok = AnalysisResultStore.put(job_id, result)
          _ = Store.apply_progress(%{job_id: job_id, stage: "completed", progress: 1.0})

        {:error, reason} ->
          _ =
            Store.apply_progress(%{
              job_id: job_id,
              stage: "failed",
              progress: 1.0,
              message: inspect(reason)
            })
      end
    end)
  end

  defp apply_agent_progress(job_id, progress) when is_binary(job_id) and is_map(progress) do
    attrs =
      progress
      |> Map.take(["stage", "progress", "residual", "iteration", "peak_memory", "message"])
      |> Enum.into(%{}, fn {key, value} -> {String.to_atom(key), value} end)
      |> Map.put(:job_id, job_id)

    _ = Store.apply_progress(attrs)
    :ok
  end

  defp create_job do
    Store.create(%{
      job_id: random_id(),
      project_id: random_id(),
      simulation_case_id: random_id()
    })
  end

  defp serialize_payload(job) do
    %{"job" => serialize_job(job) |> Map.put("has_result", false)}
  end

  defp serialize_job(job) do
    %{
      "job_id" => job.job_id,
      "project_id" => job.project_id,
      "simulation_case_id" => job.simulation_case_id,
      "worker_id" => job.worker_id,
      "message" => job.message,
      "status" => Atom.to_string(job.status),
      "progress" => job.progress,
      "residual" => job.residual,
      "iteration" => job.iteration,
      "created_at" => DateTime.to_iso8601(job.created_at),
      "updated_at" => DateTime.to_iso8601(job.updated_at)
    }
  end

  defp put_has_result(payload, value) do
    update_in(payload, ["job"], &Map.put(&1, "has_result", value))
  end

  defp normalize_axial_bar(params) do
    with {:ok, length} <- fetch_number(params, ["length", :length]),
         {:ok, area} <- fetch_number(params, ["area", :area]),
         {:ok, elements} <- fetch_number(params, ["elements", :elements]),
         {:ok, tip_force} <- fetch_number(params, ["tip_force", :tip_force]),
         {:ok, youngs_modulus_gpa} <-
           fetch_number(params, ["youngs_modulus_gpa", :youngs_modulus_gpa]) do
      {:ok,
       %{
         "length" => length,
         "area" => area,
         "elements" => round(elements),
         "tip_force" => tip_force,
         "youngs_modulus" => youngs_modulus_gpa * 1.0e9
       }}
    end
  end

  defp normalize_truss_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_truss_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_truss_2d(_params), do: {:error, :invalid_truss_model}

  defp normalize_plane_triangle_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_plane_triangle_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_plane_triangle_2d(_params), do: {:error, :invalid_plane_model}

  defp random_id do
    :crypto.strong_rand_bytes(8) |> Base.encode16(case: :lower)
  end

  defp fetch_number(params, [key | rest]) do
    case Map.fetch(params, key) do
      {:ok, value} -> cast_number(value)
      :error -> fetch_number(params, rest)
    end
  end

  defp fetch_number(_params, []), do: {:error, :missing_parameter}

  defp cast_number(value) when is_integer(value), do: {:ok, value * 1.0}
  defp cast_number(value) when is_float(value), do: {:ok, value}

  defp cast_number(value) when is_binary(value) do
    case Float.parse(value) do
      {number, ""} -> {:ok, number}
      _ -> {:error, :invalid_parameter}
    end
  end

  defp cast_number(_value), do: {:error, :invalid_parameter}
end
