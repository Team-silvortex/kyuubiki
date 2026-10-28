defmodule KyuubikiWeb.Orchestra.WorkflowCancellationDelivery do
  @moduledoc "Bounded exact-target sweeps with coalesced busy-worker handoff, not solver replay."
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Orchestra.WorkflowCancellationIntent, as: Intent
  alias KyuubikiWeb.Orchestra.WorkflowCancellationHandoff, as: Handoff
  alias KyuubikiWeb.Orchestra.WorkflowCancellationStore, as: Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Playground.AgentExecutionCancellation
  alias KyuubikiWeb.Playground.AgentCancellationDirectory, as: Directory
  alias KyuubikiWeb.Storage.FailureBoundary
  @registry KyuubikiWeb.Orchestra.WorkflowRunnerRegistry
  @budget_ms 15_000
  @max_jobs 16

  def kick([]), do: :ok

  def kick(job_ids) do
    with :ok <- KyuubikiWeb.Orchestra.WorkflowJobRunner.registry_status(),
         {:ok, authority} <- Store.access(nil, :sweep_authority) do
      kick(job_ids, authority)
    end
  end

  def kick([], _authority), do: :ok

  def kick(job_ids, authority) do
    kick(job_ids, authority, now() + 30_000)
  end

  def kick(job_ids, authority, handoff_deadline) do
    with :ok <- KyuubikiWeb.Orchestra.WorkflowJobRunner.registry_status() do
      Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
        if now() < handoff_deadline do
          case Registry.register(@registry, {__MODULE__, :sweep}, nil) do
            {:ok, _} ->
              # One trigger is not permission to retry duplicate unresolved IDs.
              job_ids
              |> sweep(0, now() + @budget_ms, authority, MapSet.new())
              |> defer(self(), authority, handoff_deadline)

            {:error, {:already_registered, holder}} ->
              # Bound the message by distinct IDs, not a duplicate-heavy input prefix.
              job_ids
              |> Handoff.continuation_jobs(MapSet.new())
              |> defer(holder, authority, handoff_deadline)

            {:error, _} ->
              :ok
          end
        end
      end)
    end
  catch
    :exit, _ -> {:error, :cancellation_worker_unavailable}
  end

  defp defer([], _holder, _authority, _deadline), do: :ok

  defp defer(ids, holder, authority, deadline),
    do: GenServer.cast(Coordinator, {:cancellation_handoff, holder, ids, authority, deadline})

  defp sweep([], _count, _deadline, _authority, _visited), do: []

  defp sweep([id | rest] = jobs, count, deadline, authority, visited) do
    cond do
      count >= @max_jobs or now() >= deadline ->
        Handoff.continuation_jobs(jobs, visited)

      # Idle and deferred sweeps must reject the same IDs before storage lookup.
      not Handoff.valid_job_id?(id) or MapSet.member?(visited, id) ->
        sweep(rest, count, deadline, authority, visited)

      true ->
        case FailureBoundary.run(fn -> AnalysisResultStore.get(id) end) do
          {:ok, %{"_workflow_recovery" => recovery}} when is_map(recovery) ->
            if Intent.pending?(recovery[Intent.key()]) do
              visited = MapSet.put(visited, id)

              case deliver_job(id, deadline, authority) do
                :halt -> []
                # Do not retry a partially visited job's unknown deliveries implicitly.
                :budget -> Handoff.continuation_jobs(rest, visited)
                _ -> sweep(rest, count + 1, deadline, authority, visited)
              end
            else
              sweep(rest, count, deadline, authority, visited)
            end

          {:error, _} ->
            []

          _ ->
            sweep(rest, count, deadline, authority, visited)
        end
    end
  end

  defp deliver_job(job_id, deadline, authority) do
    with {:ok, bucket} <- Store.access(job_id, :load, authority) do
      # The verified intent has at most 128 entries. Keep retries from starving untouched work.
      bucket["entries"]
      |> Enum.with_index()
      |> Enum.filter(fn {entry, _} -> entry["status"] in ~w(pending dispatching) end)
      |> Enum.sort_by(fn {entry, index} -> {entry["attempts"], index} end)
      |> Enum.reduce_while(%{}, fn {entry, _}, cache ->
        if now() >= deadline do
          {:halt, :budget}
        else
          case deliver_entry(job_id, bucket, entry, cache, deadline, authority) do
            :halt -> {:halt, :halt}
            next -> {:cont, next}
          end
        end
      end)
    else
      error -> continue(error, %{})
    end
  catch
    :exit, _ -> :halt
  end

  defp deliver_entry(job, bucket, entry, cache, deadline, authority) do
    action = {:reserve, bucket["intent_id"], Intent.identity(entry)}

    case Store.access(job, action, authority) do
      {:ok, token} ->
        with {:ok, endpoint, _} <- Directory.lookup(entry["endpoint_fingerprint"], deadline),
             # Discovery can outlive the sweep's authority before the first observation.
             {:ok, ^authority} <- Store.access(nil, :sweep_authority, authority) do
          prepare_and_send(job, token, endpoint, cache, deadline, authority)
        else
          {:error, reason} when is_binary(reason) ->
            continue(Store.access(job, {:finish, token, reason}, authority), cache)

          error ->
            continue(error, cache)
        end

      error ->
        continue(error, cache)
    end
  end

  defp prepare_and_send(job, token, endpoint, cache, deadline, authority) do
    {target, reason, cache} = target(job, token.entry, endpoint, cache, deadline)

    result =
      if is_map(target) do
        with :ok <- Store.access(job, {:bind, token, target}, authority),
             {:ok, ^target} <- Store.access(job, {:authorize, token}, authority),
             {:ok, current, _} <- Directory.lookup(token.entry["endpoint_fingerprint"], deadline),
             # Directory lookup can wait; it must not outlive the durable delivery authority.
             {:ok, ^target} <- Store.access(job, {:authorize, token}, authority),
             true <- now() < deadline do
          receipt =
            AgentExecutionCancellation.deliver(current, target, fn ep, exact ->
              AgentExecutionCancellation.send_endpoint(ep, exact, max(deadline - now(), 1))
            end)

          Store.access(job, {:finish, token, receipt["status"]}, authority)
        else
          {:error, reason} when is_binary(reason) ->
            Store.access(job, {:finish, token, reason}, authority)

          false ->
            Store.access(job, {:finish, token, "delivery_budget_exhausted"}, authority)

          error ->
            error
        end
      else
        Store.access(job, {:finish, token, reason}, authority)
      end

    continue(result, cache)
  end

  # A stale worker must not obtain fresh tokens for later entries or jobs.
  defp continue({:error, reason}, _cache)
       when reason in [
              :stale_cancellation_sweep,
              :stale_cancellation_delivery,
              :workflow_coordinator_unavailable,
              :orchestra_standby,
              :orchestra_lease_lost,
              :orchestra_lease_store_unavailable,
              :analysis_store_unavailable
            ],
       do: :halt

  defp continue(_result, cache), do: cache

  defp target(_, %{"execution_target" => %{} = target}, _, cache, _), do: {target, nil, cache}

  defp target(job, entry, endpoint, cache, deadline) do
    if now() >= deadline do
      {nil, "delivery_budget_exhausted", cache}
    else
      observe_target(job, entry, endpoint, cache, deadline)
    end
  end

  defp observe_target(job, entry, endpoint, cache, deadline) do
    key = entry["endpoint_fingerprint"]

    description =
      Map.get_lazy(cache, key, fn ->
        AgentExecutionCancellation.describe(endpoint, max(deadline - now(), 1))
      end)

    observed = AgentExecutionCancellation.observe(job, entry["request_id"], description)
    # The job identity comes from the stored intent, not from an Agent's description.
    {observed["execution_target"], observation_reason(observed), Map.put(cache, key, description)}
  end

  defp observation_reason(%{"execution_target" => nil}), do: "execution_target_invalid"
  defp observation_reason(observed), do: observed["status"]
  defp now, do: System.monotonic_time(:millisecond)
end
