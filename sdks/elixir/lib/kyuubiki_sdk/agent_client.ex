defmodule KyuubikiSdk.AgentClient do
  @moduledoc "High-level AI-oriented client over Kyuubiki session flows."

  alias KyuubikiSdk.Error
  alias KyuubikiSdk.Session
  alias KyuubikiSdk.WorkflowResults

  def run_study(session, solve_kind, payload, opts \\ []) do
    poll_interval = Keyword.get(opts, :poll_interval, 1_000)
    timeout = Keyword.get(opts, :timeout, 300_000)
    include_result = Keyword.get(opts, :include_result, true)

    with {:ok, outcome} <-
           Session.submit_and_wait(
             session,
             solve_kind,
             payload,
             poll_interval: poll_interval,
             timeout: timeout
           ) do
      {:ok, build_run_outcome(session, outcome, include_result)}
    end
  end

  def run_workflow_catalog(session, workflow_id, input_artifacts \\ %{}, opts \\ []) do
    poll_interval = Keyword.get(opts, :poll_interval, 1_000)
    timeout = Keyword.get(opts, :timeout, 300_000)
    include_result = Keyword.get(opts, :include_result, true)
    graph = Keyword.get(opts, :graph) || fetch_workflow_catalog_graph(session, workflow_id)

    with {:ok, outcome} <-
           Session.submit_workflow_catalog_and_wait(
             session,
             workflow_id,
             input_artifacts,
             poll_interval: poll_interval,
             timeout: timeout
           ) do
      {:ok, build_run_outcome(session, outcome, include_result, graph)}
    end
  end

  def run_workflow_graph(session, graph, input_artifacts \\ %{}, opts \\ []) do
    poll_interval = Keyword.get(opts, :poll_interval, 1_000)
    timeout = Keyword.get(opts, :timeout, 300_000)
    include_result = Keyword.get(opts, :include_result, true)

    with {:ok, outcome} <-
           Session.submit_workflow_graph_and_wait(
             session,
             graph,
             input_artifacts,
             poll_interval: poll_interval,
             timeout: timeout
           ) do
      {:ok, build_run_outcome(session, outcome, include_result, graph)}
    end
  end

  def fetch_job_bundle(session, job_id, opts \\ []) do
    include_result = Keyword.get(opts, :include_result, true)

    with {:ok, client} <- fetch_control_plane(session),
         {:ok, job} <- KyuubikiSdk.ControlPlaneClient.fetch_job(client, job_id) do
      result =
        if include_result do
          case KyuubikiSdk.ControlPlaneClient.fetch_result(client, job_id) do
            {:ok, payload} -> payload
            _ -> nil
          end
        end

      {:ok, %{job: job, result: result}}
    end
  end

  def browse_result_chunks(session, job_id, kind, opts \\ []) do
    with {:ok, client} <- fetch_control_plane(session) do
      KyuubikiSdk.ControlPlaneClient.fetch_result_chunk(
        client,
        job_id,
        kind,
        offset: Keyword.get(opts, :offset, 0),
        limit: Keyword.get(opts, :limit, 500)
      )
    end
  end

  def stream_result_chunks(session, job_id, kind, opts \\ []) do
    page_size = Keyword.get(opts, :page_size, 500)
    start_offset = Keyword.get(opts, :start_offset, 0)
    max_pages = Keyword.get(opts, :max_pages)

    Stream.resource(
      fn -> %{offset: start_offset, pages: 0, done: false} end,
      fn
        %{done: true} = state ->
          {:halt, state}

        %{offset: offset, pages: pages} = state ->
          page_opts = [offset: offset, limit: page_size]

          case browse_result_chunks(session, job_id, kind, page_opts) do
            {:ok, page} ->
              returned = page["returned"] || 0
              total = page["total"] || 0
              next_pages = pages + 1

              done =
                returned <= 0 or offset + returned >= total or
                  (is_integer(max_pages) and next_pages >= max_pages)

              {[page], %{state | offset: offset + returned, pages: next_pages, done: done}}

            {:error, reason} ->
              raise reason
          end
      end,
      fn _state -> :ok end
    )
  end

  def run_study_with_retry(session, solve_kind, payload, opts \\ []) do
    max_attempts = Keyword.get(opts, :max_attempts, 3)
    retry_on = MapSet.new(Keyword.get(opts, :retry_on, [:timeout, :transport]))
    backoff = Keyword.get(opts, :backoff, 1_000)
    backoff_multiplier = Keyword.get(opts, :backoff_multiplier, 2.0)
    run_opts = Keyword.take(opts, [:poll_interval, :timeout, :include_result])
    do_run_study_with_retry(session, solve_kind, payload, max_attempts, retry_on, backoff, backoff_multiplier, run_opts, 1, [])
  end

  def classify_failure(opts) when is_list(opts) do
    cond do
      error = Keyword.get(opts, :error) ->
        classify_error(error)

      terminal = Keyword.get(opts, :terminal) ->
        case get_in(terminal, ["job", "status"]) do
          "completed" -> :completed
          "failed" -> :failed
          "cancelled" -> :cancelled
          _ -> :pending
        end

      true ->
        :unknown
    end
  end

  defp maybe_fetch_result(session, %{"job" => %{"job_id" => job_id, "status" => "completed"}}, true) do
    case fetch_control_plane(session) do
      {:ok, client} ->
        case KyuubikiSdk.ControlPlaneClient.fetch_result(client, job_id) do
          {:ok, result} -> result
          _ -> nil
        end

      _ ->
        nil
    end
  end

  defp maybe_fetch_result(_session, _terminal, _include_result), do: nil

  defp build_run_outcome(session, outcome, include_result, workflow_graph \\ nil) do
    terminal = outcome[:terminal]
    submitted = outcome[:submitted]
    history = outcome[:history]
    result = maybe_fetch_result(session, terminal, include_result)

    {output_manifest, validated_outputs} =
      case {workflow_graph, result} do
        {graph, payload} when is_map(graph) and is_map(payload) ->
          case {WorkflowResults.build_output_manifest(graph), WorkflowResults.validate_result_against_graph(graph, payload)} do
            {{:ok, manifest}, {:ok, validated}} -> {manifest, validated}
            _ -> {nil, nil}
          end

        _ ->
          {nil, nil}
      end

    workflow_runtime =
      case result do
        payload when is_map(payload) ->
          case WorkflowResults.normalize_runtime(payload) do
            {:ok, runtime} -> runtime
            _ -> nil
          end

        _ ->
          nil
      end

    workflow_progression =
      case WorkflowResults.normalize_progression(history, result) do
        {:ok, progression} -> progression
        _ -> nil
      end

    %{
      submitted: submitted,
      terminal: terminal,
      history: history,
      result: result,
      workflow_runtime: workflow_runtime,
      workflow_progression: workflow_progression,
      output_manifest: output_manifest,
      validated_outputs: validated_outputs
    }
  end

  defp fetch_control_plane(%Session{control_plane: nil}), do: {:error, KyuubikiSdk.Error.transport("control plane client is not configured")}
  defp fetch_control_plane(%Session{control_plane: client}), do: {:ok, client}

  defp fetch_workflow_catalog_graph(session, workflow_id) do
    case fetch_control_plane(session) do
      {:ok, client} ->
        case KyuubikiSdk.ControlPlaneClient.fetch_workflow_catalog_workflow(client, workflow_id) do
          {:ok, %{"workflow" => %{"graph" => graph}}} when is_map(graph) -> graph
          _ -> nil
        end

      _ ->
        nil
    end
  end

  defp do_run_study_with_retry(session, solve_kind, payload, max_attempts, retry_on, backoff, backoff_multiplier, run_opts, attempt, attempts) do
    case run_study(session, solve_kind, payload, run_opts) do
      {:ok, outcome} ->
        {:ok, Map.merge(outcome, %{attempt_count: attempt, attempts: Enum.reverse(attempts)})}

      {:error, reason} ->
        classification = classify_error(reason)

        next_attempts = [
          %{attempt: attempt, classification: classification, message: Exception.message(reason)}
          | attempts
        ]

        if attempt >= max_attempts or not MapSet.member?(retry_on, classification) do
          {:error, reason}
        else
          Process.sleep(backoff)

          do_run_study_with_retry(
            session,
            solve_kind,
            payload,
            max_attempts,
            retry_on,
            round(backoff * backoff_multiplier),
            backoff_multiplier,
            run_opts,
            attempt + 1,
            next_attempts
          )
        end
    end
  end

  defp classify_error(%Error{type: :timeout}), do: :timeout
  defp classify_error(%Error{type: :transport}), do: :transport
  defp classify_error(%Error{type: :rpc}), do: :rpc
  defp classify_error(%Error{type: :http, status_code: 401}), do: :auth
  defp classify_error(%Error{type: :http, status_code: 403}), do: :auth
  defp classify_error(%Error{type: :http, status_code: 404}), do: :not_found
  defp classify_error(%Error{type: :http, status_code: status}) when is_integer(status) and status >= 500, do: :server
  defp classify_error(%Error{type: :http}), do: :http
  defp classify_error(_error), do: :unknown
end
