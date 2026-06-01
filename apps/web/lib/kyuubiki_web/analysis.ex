defmodule KyuubikiWeb.Analysis do
  @moduledoc """
  Asynchronous orchestration for FEM study jobs.
  """

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Library
  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.SecurityEvents.Store, as: SecurityEventStore

  @spec submit_axial_bar(map()) :: {:ok, map()} | {:error, term()}
  def submit_axial_bar(params) when is_map(params) do
    with {:ok, normalized} <- normalize_axial_bar(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_bar_1d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_thermal_bar_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_bar_1d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_thermal_bar_1d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_thermal_bar_1d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_heat_bar_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_heat_bar_1d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_heat_bar_1d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_heat_bar_1d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_heat_plane_triangle_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_heat_plane_triangle_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_heat_plane_triangle_2d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_heat_plane_triangle_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_heat_plane_quad_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_heat_plane_quad_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_heat_plane_quad_2d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_heat_plane_quad_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_thermal_truss_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_truss_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_thermal_truss_2d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_thermal_truss_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_thermal_truss_3d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_truss_3d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_thermal_truss_3d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_thermal_truss_3d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_beam_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_beam_1d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_beam_1d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_beam_1d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_thermal_beam_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_beam_1d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_thermal_beam_1d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_thermal_beam_1d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_torsion_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_torsion_1d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_torsion_1d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_torsion_1d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_spring_1d(map()) :: {:ok, map()} | {:error, term()}
  def submit_spring_1d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_spring_1d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_spring_1d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_spring_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_spring_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_spring_2d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_spring_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_spring_3d(map()) :: {:ok, map()} | {:error, term()}
  def submit_spring_3d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_spring_3d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_spring_3d", normalized)
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

  @spec submit_thermal_plane_triangle_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_plane_triangle_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_thermal_plane_triangle_2d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_thermal_plane_triangle_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_plane_quad_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_plane_quad_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_plane_quad_2d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_plane_quad_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_thermal_plane_quad_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_plane_quad_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_thermal_plane_quad_2d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_thermal_plane_quad_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_frame_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_frame_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_frame_2d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_frame_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_frame_3d(map()) :: {:ok, map()} | {:error, term()}
  def submit_frame_3d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_frame_3d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_frame_3d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_thermal_frame_2d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_frame_2d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_thermal_frame_2d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_thermal_frame_2d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec submit_thermal_frame_3d(map()) :: {:ok, map()} | {:error, term()}
  def submit_thermal_frame_3d(params) when is_map(params) do
    with {:ok, normalized} <- normalize_thermal_frame_3d(params),
         {:ok, job_context} <- derive_job_context(params),
         {:ok, job} <- create_job(job_context) do
      start_background_job(job.job_id, "solve_thermal_frame_3d", normalized)
      {:ok, serialize_payload(job)}
    end
  end

  @spec run_workflow_graph(map()) :: {:ok, map()} | {:error, term()}
  def run_workflow_graph(params) when is_map(params) do
    normalized = stringify_keys(params)

    with %{} = graph <- Map.get(normalized, "graph"),
         %{} = input_artifacts <- Map.get(normalized, "input_artifacts"),
         {:ok, completed_nodes, artifacts} <- execute_workflow_graph(graph, input_artifacts) do
      {:ok,
       %{
         "workflow_id" => Map.get(graph, "id"),
         "completed_nodes" => completed_nodes,
         "artifacts" => artifacts
       }}
    else
      nil -> {:error, :invalid_workflow_graph_request}
      [] -> {:error, :invalid_workflow_graph_request}
      {:error, _reason} = error -> error
      _ -> {:error, :invalid_workflow_graph_request}
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

  def create_security_event(attrs) when is_map(attrs) do
    case SecurityEventStore.create(attrs) do
      {:ok, event} -> {:ok, %{"event" => event}}
      {:error, _reason} = error -> error
    end
  end

  def list_security_events(filters \\ %{}) when is_map(filters) do
    %{"events" => SecurityEventStore.list(filters)}
  end

  def export_security_events(filters \\ %{}) when is_map(filters) do
    events = SecurityEventStore.list(filters)

    summary =
      Enum.reduce(
        events,
        %{"total" => length(events), "by_source" => %{}, "by_risk" => %{}, "by_status" => %{}},
        fn event, acc ->
          acc
          |> put_in(
            ["by_source", event["source"]],
            get_in(acc, ["by_source", event["source"]]) |> Kernel.||(0) |> Kernel.+(1)
          )
          |> put_in(
            ["by_risk", event["risk"]],
            get_in(acc, ["by_risk", event["risk"]]) |> Kernel.||(0) |> Kernel.+(1)
          )
          |> put_in(
            ["by_status", event["status"]],
            get_in(acc, ["by_status", event["status"]]) |> Kernel.||(0) |> Kernel.+(1)
          )
        end
      )

    %{
      "exported_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601(),
      "schema" => %{
        "name" => "kyuubiki.security-events.export/v1",
        "version" => 1,
        "fields" => [
          %{"name" => "event_id", "type" => "string"},
          %{"name" => "event_type", "type" => "string"},
          %{"name" => "source", "type" => "string"},
          %{"name" => "action", "type" => "string"},
          %{"name" => "risk", "type" => "string"},
          %{"name" => "status", "type" => "string"},
          %{"name" => "note", "type" => "string|null"},
          %{"name" => "context", "type" => "object"},
          %{"name" => "occurred_at", "type" => "datetime"},
          %{"name" => "inserted_at", "type" => "datetime|null"},
          %{"name" => "updated_at", "type" => "datetime|null"}
        ]
      },
      "filters" => normalize_export_filters(filters),
      "summary" => summary,
      "events" => events
    }
  end

  def export_security_events_csv(filters \\ %{}) when is_map(filters) do
    events = SecurityEventStore.list(filters)

    rows = [
      [
        "event_id",
        "event_type",
        "source",
        "action",
        "risk",
        "status",
        "note",
        "study_kind",
        "project_id",
        "model_version_id",
        "occurred_at",
        "inserted_at",
        "updated_at"
      ]
      | Enum.map(events, fn event ->
          context = event["context"] || %{}

          [
            event["event_id"],
            event["event_type"],
            event["source"],
            event["action"],
            event["risk"],
            event["status"],
            event["note"],
            context["study_kind"],
            context["project_id"],
            context["model_version_id"],
            event["occurred_at"],
            event["inserted_at"],
            event["updated_at"]
          ]
        end)
    ]

    rows
    |> Enum.map_join("\n", fn row -> Enum.map_join(row, ",", &csv_escape/1) end)
    |> Kernel.<>("\n")
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
      "results" => list_results()["results"],
      "security_events" => list_security_events()["events"]
    }
  end

  defp normalize_export_filters(filters) do
    filters
    |> Map.take([
      "source",
      "risk",
      "status",
      "action",
      "study_kind",
      "project_id",
      "model_version_id",
      "occurred_after",
      "occurred_before",
      "limit"
    ])
    |> Enum.reject(fn {_key, value} -> is_nil(value) or value == "" end)
    |> Map.new()
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

  defp execute_workflow_graph(%{"nodes" => nodes} = graph, input_artifacts)
       when is_list(nodes) and is_map(input_artifacts) do
    edges = Map.get(graph, "edges", [])

    do_execute_workflow_graph(
      nodes,
      edges,
      input_artifacts,
      MapSet.new(),
      [],
      %{}
    )
  end

  defp execute_workflow_graph(_graph, _input_artifacts), do: {:error, :invalid_workflow_graph}

  defp do_execute_workflow_graph(nodes, edges, input_artifacts, completed, ordered, artifacts) do
    {next_completed, next_ordered, next_artifacts, progressed?} =
      Enum.reduce(nodes, {completed, ordered, artifacts, false}, fn node,
                                                                   {done, order, acc, moved?} ->
        node_id = Map.get(node, "id")

        cond do
          MapSet.member?(done, node_id) ->
            {done, order, acc, moved?}

          true ->
            incoming = Enum.filter(edges, &(get_in(&1, ["to", "node"]) == node_id))
            kind = Map.get(node, "kind")
            ready? = kind == "input" or Enum.all?(incoming, &Map.has_key?(acc, edge_from_key(&1)))

            if ready? do
              case execute_workflow_node(node, incoming, input_artifacts, acc) do
                {:ok, updated_artifacts} ->
                  {
                    MapSet.put(done, node_id),
                    order ++ [node_id],
                    updated_artifacts,
                    true
                  }

                {:error, reason} ->
                  throw({:workflow_node_error, node_id, reason})
              end
            else
              {done, order, acc, moved?}
            end
        end
      end)

    if MapSet.size(next_completed) == length(nodes) do
      {:ok, next_ordered, next_artifacts}
    else
      if progressed? do
        do_execute_workflow_graph(
          nodes,
          edges,
          input_artifacts,
          next_completed,
          next_ordered,
          next_artifacts
        )
      else
        pending =
          nodes
          |> Enum.map(&Map.get(&1, "id"))
          |> Enum.reject(&MapSet.member?(next_completed, &1))

        {:error, {:workflow_stalled, pending}}
      end
    end
  catch
    {:workflow_node_error, node_id, reason} ->
      {:error, {:workflow_node_error, node_id, reason}}
  end

  defp execute_workflow_node(%{"kind" => "input", "id" => node_id} = node, _incoming, inputs, artifacts) do
    case Map.fetch(inputs, node_id) do
      {:ok, value} ->
        updated =
          Enum.reduce(Map.get(node, "outputs", []), artifacts, fn output, acc ->
            Map.put(acc, artifact_key(node_id, Map.get(output, "id")), value)
          end)

        {:ok, updated}

      :error ->
        {:error, :missing_input_artifact}
    end
  end

  defp execute_workflow_node(%{"kind" => "solve"} = node, incoming, _inputs, artifacts) do
    with {:ok, operator_id} <- fetch_operator_id(node),
         {:ok, payload} <- resolve_single_input_payload(node, incoming, artifacts),
         {:ok, result} <- run_workflow_solve_operator(operator_id, payload) do
      {:ok, publish_node_outputs(node, result, artifacts)}
    end
  end

  defp execute_workflow_node(%{"kind" => "transform"} = node, incoming, _inputs, artifacts) do
    with {:ok, operator_id} <- fetch_operator_id(node),
         {:ok, payload} <- resolve_single_input_payload(node, incoming, artifacts),
         {:ok, result} <-
           run_workflow_transform_operator(operator_id, payload, Map.get(node, "config")) do
      {:ok, publish_node_outputs(node, result, artifacts)}
    end
  end

  defp execute_workflow_node(%{"kind" => "extract"} = node, incoming, _inputs, artifacts) do
    with {:ok, operator_id} <- fetch_operator_id(node),
         {:ok, payload} <- resolve_single_input_payload(node, incoming, artifacts),
         {:ok, result} <-
           run_workflow_extract_operator(operator_id, payload, Map.get(node, "config")) do
      {:ok, publish_node_outputs(node, result, artifacts)}
    end
  end

  defp execute_workflow_node(%{"kind" => "export"} = node, incoming, _inputs, artifacts) do
    with {:ok, operator_id} <- fetch_operator_id(node),
         {:ok, payload} <- resolve_single_input_payload(node, incoming, artifacts),
         {:ok, result} <-
           run_workflow_export_operator(operator_id, payload, Map.get(node, "config")) do
      {:ok, publish_node_outputs(node, result, artifacts)}
    end
  end

  defp execute_workflow_node(%{"kind" => "output"} = node, incoming, _inputs, artifacts) do
    updated =
      Enum.reduce(incoming, artifacts, fn edge, acc ->
        Map.put(acc, artifact_key(Map.get(node, "id"), get_in(edge, ["to", "port"])), Map.fetch!(acc, edge_from_key(edge)))
      end)

    {:ok, updated}
  end

  defp execute_workflow_node(%{"kind" => kind}, _incoming, _inputs, _artifacts),
    do: {:error, {:unsupported_workflow_node_kind, kind}}

  defp fetch_operator_id(%{"operator_id" => operator_id}) when is_binary(operator_id) and operator_id != "",
    do: {:ok, operator_id}

  defp fetch_operator_id(_node), do: {:error, :missing_operator_id}

  defp resolve_single_input_payload(%{"id" => node_id}, incoming, artifacts) do
    case incoming do
      [first | _] ->
        key = edge_from_key(first)

        case Map.fetch(artifacts, key) do
          {:ok, value} -> {:ok, value}
          :error -> {:error, {:missing_upstream_artifact, key}}
        end

      [] ->
        {:error, {:missing_workflow_input, node_id}}
    end
  end

  defp publish_node_outputs(node, value, artifacts) do
    Enum.reduce(Map.get(node, "outputs", []), artifacts, fn output, acc ->
      Map.put(acc, artifact_key(Map.get(node, "id"), Map.get(output, "id")), value)
    end)
  end

  defp run_workflow_solve_operator("solve.heat_plane_quad_2d", payload) when is_map(payload) do
    AgentClient.solve_heat_plane_quad_2d(payload)
  end

  defp run_workflow_solve_operator("solve.thermal_plane_quad_2d", payload) when is_map(payload) do
    AgentClient.solve_thermal_plane_quad_2d(payload)
  end

  defp run_workflow_solve_operator(operator_id, _payload),
    do: {:error, {:unsupported_workflow_solve_operator, operator_id}}

  defp run_workflow_transform_operator(
         "bridge.temperature_field_to_thermo_quad_2d",
         heat_result,
         thermo_seed_model
       )
       when is_map(heat_result) and is_map(thermo_seed_model) do
    bridge_heat_result_to_thermal_plane_quad_model(heat_result, thermo_seed_model)
  end

  defp run_workflow_transform_operator(operator_id, _payload, _config),
    do: {:error, {:unsupported_workflow_transform_operator, operator_id}}

  defp run_workflow_extract_operator("extract.result_summary", payload, config)
       when is_map(payload) do
    extract_result_summary(payload, config || %{})
  end

  defp run_workflow_extract_operator(operator_id, _payload, _config),
    do: {:error, {:unsupported_workflow_extract_operator, operator_id}}

  defp run_workflow_export_operator("export.summary_json", payload, _config) when is_map(payload) do
    export_summary_json(payload)
  end

  defp run_workflow_export_operator("export.summary_csv", payload, config) when is_map(payload) do
    export_summary_csv(payload, config || %{})
  end

  defp run_workflow_export_operator(operator_id, _payload, _config),
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
                   Map.put(thermo_node, "temperature_delta", Map.get(heat_node, "temperature"))
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

  defp bridge_heat_result_to_thermal_plane_quad_model(_heat_result, _thermo_seed_model),
    do: {:error, :invalid_bridge_payload}

  defp close_enough?(left, right) when is_number(left) and is_number(right),
    do: abs(left - right) <= 1.0e-9

  defp close_enough?(_, _), do: false

  defp edge_from_key(edge),
    do: artifact_key(get_in(edge, ["from", "node"]), get_in(edge, ["from", "port"]))

  defp artifact_key(node_id, port_id) when is_binary(node_id) and is_binary(port_id),
    do: "#{node_id}.#{port_id}"

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
      value when is_integer(value) and value >= 0 ->
        {:ok, value}

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

  defp normalize_thermal_truss_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_truss_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_truss_2d(_params), do: {:error, :invalid_thermal_truss_model}

  defp normalize_truss_3d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_truss_3d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_truss_3d(_params), do: {:error, :invalid_truss_3d_model}

  defp normalize_thermal_truss_3d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_truss_3d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_truss_3d(_params), do: {:error, :invalid_thermal_truss_3d_model}

  defp normalize_plane_triangle_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_plane_triangle_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_plane_triangle_2d(_params), do: {:error, :invalid_plane_model}

  defp normalize_thermal_plane_triangle_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_plane_triangle_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_plane_triangle_2d(_params), do: {:error, :invalid_thermal_plane_model}

  defp normalize_plane_quad_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_plane_quad_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_plane_quad_2d(_params), do: {:error, :invalid_plane_quad_model}

  defp normalize_thermal_plane_quad_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_plane_quad_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_plane_quad_2d(_params), do: {:error, :invalid_thermal_plane_quad_model}

  defp normalize_beam_1d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_beam_1d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_beam_1d(_params), do: {:error, :invalid_beam_model}

  defp normalize_thermal_beam_1d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_beam_1d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_beam_1d(_params), do: {:error, :invalid_thermal_beam_model}

  defp normalize_thermal_bar_1d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_bar_1d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_bar_1d(_params), do: {:error, :invalid_thermal_bar_model}

  defp normalize_heat_bar_1d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_heat_bar_1d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_heat_bar_1d(_params), do: {:error, :invalid_heat_bar_model}

  defp normalize_heat_plane_triangle_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements),
       do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  defp normalize_heat_plane_triangle_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements),
       do: {:ok, %{nodes: nodes, elements: elements}}

  defp normalize_heat_plane_triangle_2d(_params), do: {:error, :invalid_heat_plane_triangle_model}

  defp normalize_heat_plane_quad_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements),
       do: {:ok, %{"nodes" => nodes, "elements" => elements}}

  defp normalize_heat_plane_quad_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements),
       do: {:ok, %{nodes: nodes, elements: elements}}

  defp normalize_heat_plane_quad_2d(_params), do: {:error, :invalid_heat_plane_quad_model}

  defp normalize_torsion_1d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_torsion_1d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_torsion_1d(_params), do: {:error, :invalid_torsion_model}

  defp normalize_spring_1d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_spring_1d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_spring_1d(_params), do: {:error, :invalid_spring_model}

  defp normalize_spring_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_spring_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_spring_2d(_params), do: {:error, :invalid_spring_2d_model}

  defp normalize_spring_3d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_spring_3d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_spring_3d(_params), do: {:error, :invalid_spring_3d_model}

  defp normalize_frame_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_frame_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_frame_2d(_params), do: {:error, :invalid_frame_model}

  defp normalize_frame_3d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_frame_3d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_frame_3d(_params), do: {:error, :invalid_frame_3d_model}

  defp normalize_thermal_frame_2d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_frame_2d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_frame_2d(_params), do: {:error, :invalid_thermal_frame_model}

  defp normalize_thermal_frame_3d(%{"nodes" => nodes, "elements" => elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_frame_3d(%{nodes: nodes, elements: elements})
       when is_list(nodes) and is_list(elements) do
    {:ok, %{"nodes" => nodes, "elements" => elements}}
  end

  defp normalize_thermal_frame_3d(_params), do: {:error, :invalid_thermal_frame_3d_model}

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

  defp stringify_keys(value) when is_list(value), do: Enum.map(value, &stringify_keys/1)

  defp stringify_keys(value) when is_map(value) do
    value
    |> Enum.map(fn {key, nested} -> {to_string(key), stringify_keys(nested)} end)
    |> Map.new()
  end

  defp stringify_keys(value), do: value
end
