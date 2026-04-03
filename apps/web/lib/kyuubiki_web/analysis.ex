defmodule KyuubikiWeb.Analysis do
  @moduledoc """
  Asynchronous orchestration for FEM study jobs.
  """

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Library
  alias KyuubikiWeb.Playground.AgentClient

  @spec submit_axial_bar(map()) :: {:ok, map()} | {:error, term()}
  def submit_axial_bar(params) when is_map(params) do
    with {:ok, normalized} <- normalize_axial_bar(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_bar_1d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_truss_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_truss_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_truss_2d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_truss_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_truss_3d(map()) :: {:ok, map()} | {:error, term()}
  def submit_truss_3d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_truss_3d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_truss_3d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_plane_triangle_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_plane_triangle_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_plane_triangle_2d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
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

  def update_job(job_id, attrs) when is_binary(job_id) and is_map(attrs) do
    case Store.update_metadata(job_id, attrs) do
      {:ok, job} -> {:ok, serialize_payload(job)}
      {:error, _reason} = error -> error
    end
  end

  def cancel_job(job_id) when is_binary(job_id) do
    case Store.get(job_id) do
      {:ok, job} ->
        if job.status in [:completed, :failed, :cancelled] do
          {:ok, serialize_payload(job)}
        else
          _ =
            Store.apply_progress(%{
              job_id: job_id,
              stage: "cancelled",
              progress: job.progress,
              message: "job cancelled by operator"
            })

          _ = AgentClient.cancel_job(job_id)
          fetch_job(job_id)
        end

      :error ->
        {:error, {:job_not_found, job_id}}
    end
  end

  def delete_job(job_id) when is_binary(job_id) do
    _ = AnalysisResultStore.delete(job_id)

    case Store.delete(job_id) do
      {:ok, job} -> {:ok, %{"job" => serialize_job(job), "deleted" => true}}
      {:error, _reason} = error -> error
    end
  end

  def list_results do
    %{"results" => AnalysisResultStore.list()}
  end

  def fetch_result(job_id) when is_binary(job_id) do
    case AnalysisResultStore.get(job_id) do
      {:ok, result} -> {:ok, %{"job_id" => job_id, "result" => result}}
      :error -> {:error, {:result_not_found, job_id}}
    end
  end

  def fetch_result_chunk(job_id, kind, params \\ %{})
      when is_binary(job_id) and is_binary(kind) and is_map(params) do
    with {:ok, result} <- fetch_raw_result(job_id),
         {:ok, items} <- fetch_chunk_source(result, kind),
         {:ok, offset} <- normalize_chunk_integer(params, "offset", 0),
         {:ok, limit} <- normalize_chunk_integer(params, "limit", 200) do
      safe_offset = min(offset, length(items))
      safe_limit = max(limit, 1)
      chunk = items |> Enum.drop(safe_offset) |> Enum.take(safe_limit)

      {:ok,
       %{
         "job_id" => job_id,
         "kind" => kind,
         "offset" => safe_offset,
         "limit" => safe_limit,
         "returned" => length(chunk),
         "total" => length(items),
         "items" => chunk
       }}
    end
  end

  def update_result(job_id, result) when is_binary(job_id) and is_map(result) do
    :ok = AnalysisResultStore.update(job_id, result)
    fetch_result(job_id)
  end

  def delete_result(job_id) when is_binary(job_id) do
    case AnalysisResultStore.delete(job_id) do
      {:ok, result} -> {:ok, %{"job_id" => job_id, "result" => result, "deleted" => true}}
      {:error, _reason} = error -> error
    end
  end

  def export_database do
    {:ok, projects} = Library.list_projects()
    models = Enum.flat_map(projects, &Map.get(&1, "models", []))

    model_versions =
      models
      |> Enum.flat_map(fn model ->
        case Library.list_versions(model["model_id"]) do
          {:ok, versions} -> versions
          _ -> []
        end
      end)

    %{
      "exported_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601(),
      "projects" => projects,
      "models" => models,
      "model_versions" => model_versions,
      "jobs" => list_jobs()["jobs"],
      "results" => list_results()["results"]
    }
  end

  defp start_background_job(job_id, method, params) do
    Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
      execute_background_job(job_id, method, params)
    end)
  end

  defp apply_agent_progress(job_id, progress) when is_binary(job_id) and is_map(progress) do
    case Store.get(job_id) do
      {:ok, %{status: :cancelled}} ->
        :ok

      {:ok, _job} ->
        attrs =
          progress
          |> Map.take(["stage", "progress", "residual", "iteration", "peak_memory", "message"])
          |> Enum.into(%{}, fn {key, value} -> {String.to_atom(key), value} end)
          |> Map.put(:job_id, job_id)

        _ = Store.apply_progress(attrs)
        :ok

      :error ->
        :ok
    end
  end

  defp execute_background_job(job_id, method, params) do
    timeout_ms = watchdog_job_timeout_ms()

    task =
      Task.async(fn ->
        AgentClient.request_with_agent(method, params, &apply_agent_progress(job_id, &1))
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
        unless cancelled?(job_id) do
          fail_job(job_id, inspect(reason))
        end

      nil ->
        unless cancelled?(job_id) do
          fail_job(job_id, "job execution timed out after #{timeout_ms} ms")
        end
    end
  end

  defp fail_job(job_id, message) when is_binary(job_id) and is_binary(message) do
    _ =
      Store.apply_progress(%{
        job_id: job_id,
        stage: "failed",
        progress: 1.0,
        message: message
      })

    :ok
  end

  defp cancel_job_with_message(job_id, message) when is_binary(job_id) and is_binary(message) do
    _ =
      Store.apply_progress(%{
        job_id: job_id,
        stage: "cancelled",
        progress: 1.0,
        message: message
      })

    :ok
  end

  defp cancelled?(job_id) when is_binary(job_id) do
    match?({:ok, %{status: :cancelled}}, Store.get(job_id))
  end

  defp watchdog_job_timeout_ms do
    Application.get_env(:kyuubiki_web, KyuubikiWeb.Jobs.Watchdog, [])
    |> Keyword.get(:job_timeout_ms, 120_000)
  end

  defp create_job(attrs) do
    Store.create(%{
      job_id: random_id(),
      project_id: Map.get(attrs, :project_id, random_id()),
      model_version_id: Map.get(attrs, :model_version_id),
      simulation_case_id: Map.get(attrs, :simulation_case_id, random_id())
    })
  end

  defp derive_job_context(params) when is_map(params) do
    project_id = fetch_optional_string(params, ["project_id", :project_id])
    model_version_id = fetch_optional_string(params, ["model_version_id", :model_version_id])

    cond do
      is_binary(model_version_id) and model_version_id != "" ->
        case Library.get_version(model_version_id) do
          {:ok, version} ->
            {:ok,
             %{
               project_id: version["project_id"],
               model_version_id: version["version_id"],
               simulation_case_id: version["version_id"]
             }}

          :error ->
            {:error, {:model_version_not_found, model_version_id}}
        end

      is_binary(project_id) and project_id != "" ->
        {:ok, %{project_id: project_id}}

      true ->
        {:ok, %{}}
    end
  end

  defp serialize_payload(job) do
    %{"job" => serialize_job(job) |> Map.put("has_result", false)}
  end

  defp fetch_raw_result(job_id) do
    case AnalysisResultStore.get(job_id) do
      {:ok, result} -> {:ok, result}
      :error -> {:error, {:result_not_found, job_id}}
    end
  end

  defp fetch_chunk_source(result, "nodes") when is_map(result) do
    case Map.get(result, "nodes") do
      items when is_list(items) -> {:ok, items}
      _ -> {:error, {:unsupported_chunk_kind, "nodes"}}
    end
  end

  defp fetch_chunk_source(result, "elements") when is_map(result) do
    case Map.get(result, "elements") do
      items when is_list(items) -> {:ok, items}
      _ -> {:error, {:unsupported_chunk_kind, "elements"}}
    end
  end

  defp fetch_chunk_source(_result, kind), do: {:error, {:unsupported_chunk_kind, kind}}

  defp normalize_chunk_integer(params, key, default) do
    case Map.get(params, key, default) do
      value when is_integer(value) and value >= 0 -> {:ok, value}
      value when is_binary(value) ->
        case Integer.parse(value) do
          {parsed, ""} when parsed >= 0 -> {:ok, parsed}
          _ -> {:error, {:invalid_chunk_param, key}}
        end

      _ ->
        {:error, {:invalid_chunk_param, key}}
    end
  end

  defp serialize_job(job) do
    %{
      "job_id" => job.job_id,
      "project_id" => job.project_id,
      "model_version_id" => job.model_version_id,
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

  defp normalize_truss_3d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_truss_3d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_truss_3d(_params), do: {:error, :invalid_truss_3d_model}

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

  defp fetch_optional_string(params, [key | rest]) do
    case Map.fetch(params, key) do
      {:ok, value} when is_binary(value) ->
        trimmed = String.trim(value)

        if byte_size(trimmed) > 0 do
          trimmed
        else
          fetch_optional_string(params, rest)
        end

      _ ->
        fetch_optional_string(params, rest)
    end
  end

  defp fetch_optional_string(_params, []), do: nil

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
