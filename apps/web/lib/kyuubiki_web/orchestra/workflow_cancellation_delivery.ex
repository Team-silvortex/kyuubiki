defmodule KyuubikiWeb.Orchestra.WorkflowCancellationDelivery do
  @moduledoc "One bounded best-effort sweep; durable exact targets permit later explicit recovery."
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Orchestra.WorkflowCancellationIntent, as: Intent
  alias KyuubikiWeb.Orchestra.WorkflowCancellationStore, as: Store
  alias KyuubikiWeb.Playground.AgentExecutionCancellation
  alias KyuubikiWeb.Playground.AgentCancellationDirectory, as: Directory
  alias KyuubikiWeb.Storage.FailureBoundary
  @registry KyuubikiWeb.Orchestra.WorkflowRunnerRegistry
  @budget_ms 15_000
  @max_jobs 16

  def kick([]), do: :ok

  def kick(job_ids) do
    with :ok <- KyuubikiWeb.Orchestra.WorkflowJobRunner.registry_status() do
      Task.Supervisor.start_child(KyuubikiWeb.TaskSupervisor, fn ->
        case Registry.register(@registry, {__MODULE__, :sweep}, nil) do
          {:ok, _} -> sweep(job_ids)
          {:error, _} -> :ok
        end
      end)
    end
  catch
    :exit, _ -> {:error, :cancellation_worker_unavailable}
  end

  defp sweep(job_ids) do
    deadline = now() + @budget_ms

    Enum.reduce_while(job_ids, 0, fn id, count ->
      if count >= @max_jobs or now() >= deadline do
        {:halt, count}
      else
        case FailureBoundary.run(fn -> AnalysisResultStore.get(id) end) do
          {:ok, %{"_workflow_recovery" => recovery}} when is_map(recovery) ->
            if Intent.pending?(recovery[Intent.key()]) do
              deliver_job(id, deadline)
              {:cont, count + 1}
            else
              {:cont, count}
            end

          {:error, _} ->
            {:halt, count}

          _ ->
            {:cont, count}
        end
      end
    end)
  end

  defp deliver_job(job_id, deadline) do
    with {:ok, bucket} <- Store.access(job_id, :load) do
      Enum.reduce_while(bucket["entries"], %{}, fn entry, cache ->
        if now() >= deadline do
          {:halt, cache}
        else
          {:cont, deliver_entry(job_id, bucket, entry, cache, deadline)}
        end
      end)
    end
  catch
    :exit, _ -> :ok
  end

  defp deliver_entry(job, bucket, entry, cache, deadline) do
    case Store.access(job, {:reserve, bucket["intent_id"], Intent.identity(entry)}) do
      {:ok, token} ->
        case Directory.lookup(entry["endpoint_fingerprint"], deadline) do
          {:ok, endpoint, _} ->
            prepare_and_send(job, token, endpoint, cache, deadline)

          {:error, reason} ->
            Store.access(job, {:finish, token, reason})
            cache
        end

      _ ->
        cache
    end
  end

  defp prepare_and_send(job, token, endpoint, cache, deadline) do
    {target, reason, cache} = target(job, token.entry, endpoint, cache, deadline)

    if is_map(target) do
      with :ok <- Store.access(job, {:bind, token, target}),
           {:ok, ^target} <- Store.access(job, {:authorize, token}),
           {:ok, current, _} <- Directory.lookup(token.entry["endpoint_fingerprint"], deadline),
           # Directory lookup can wait; it must not outlive the durable delivery authority.
           {:ok, ^target} <- Store.access(job, {:authorize, token}),
           true <- now() < deadline do
        receipt =
          AgentExecutionCancellation.deliver(current, target, fn ep, exact ->
            AgentExecutionCancellation.send_endpoint(ep, exact, max(deadline - now(), 1))
          end)

        Store.access(job, {:finish, token, receipt["status"]})
      else
        {:error, reason} when is_binary(reason) -> Store.access(job, {:finish, token, reason})
        false -> Store.access(job, {:finish, token, "delivery_budget_exhausted"})
        _ -> :ok
      end
    else
      Store.access(job, {:finish, token, reason})
    end

    cache
  end

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
