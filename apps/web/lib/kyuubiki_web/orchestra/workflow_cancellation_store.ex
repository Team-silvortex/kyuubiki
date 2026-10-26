defmodule KyuubikiWeb.Orchestra.WorkflowCancellationStore do
  @moduledoc false
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowCancellationIntent, as: Intent
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryOwnership, as: Ownership
  alias KyuubikiWeb.Storage.FailureBoundary

  def access(job_id, action) do
    GenServer.call(Coordinator, {:cancellation_intent, job_id, action}, 30_000)
  catch
    :exit, _ -> {:error, :workflow_coordinator_unavailable}
  end

  def handle(job_id, action, state) do
    result =
      Ownership.guarded_write(state, fn ->
        authority = {state.session_id, state.lease_session}
        FailureBoundary.run(fn -> read_and_apply(job_id, action, authority) end)
      end)

    {:reply, result, Ownership.after_write(state, result)}
  end

  defp read_and_apply(job_id, action, authority) do
    with {:ok, %{status: :failed} = job} <- Store.get(job_id),
         {:ok, %{"_workflow_recovery" => %{"state" => terminal} = recovery} = runtime} <-
           AnalysisResultStore.get(job_id),
         true <- terminal in ~w(failed recovery_blocked),
         :ok <- Intent.verify(recovery, job) do
      apply_action(action, job, runtime, recovery, authority)
    else
      {:error, _} = error -> error
      _ -> {:error, :workflow_cancellation_not_applicable}
    end
  end

  defp apply_action(:load, _job, _runtime, recovery, _authority),
    do: {:ok, recovery[Intent.key()]}

  defp apply_action({:reserve, intent_id, id}, job, runtime, recovery, {session, lease_session}) do
    with true <- recovery[Intent.key()]["intent_id"] == intent_id,
         {:ok, bucket, entry} <- Intent.reserve(recovery[Intent.key()], id),
         :ok <- persist(job, runtime, recovery, bucket) do
      if entry["status"] == "exhausted" do
        {:error, :cancellation_attempts_exhausted}
      else
        {:ok,
         %{
           intent_id: intent_id,
           id: id,
           delivery_id: entry["delivery_id"],
           coordinator_session: session,
           lease_session: lease_session,
           entry: entry
         }}
      end
    else
      false -> {:error, :stale_cancellation_intent}
      error -> error
    end
  end

  defp apply_action({operation, token, value}, job, runtime, recovery, authority)
       when operation in [:bind, :finish] do
    with :ok <- token_matches(recovery, token, authority),
         {:ok, bucket, _entry} <- apply(Intent, operation, [recovery[Intent.key()], token, value]),
         :ok <- persist(job, runtime, recovery, bucket) do
      :ok
    end
  end

  defp apply_action({:authorize, token}, _job, _runtime, recovery, authority) do
    with :ok <- token_matches(recovery, token, authority),
         {:ok, %{"execution_target" => %{} = target}} <-
           Intent.owned_entry(recovery[Intent.key()], token) do
      {:ok, target}
    else
      {:error, _} = error -> error
      _ -> {:error, :cancellation_target_not_bound}
    end
  end

  defp apply_action(_, _, _, _, _), do: {:error, :invalid_cancellation_action}

  defp token_matches(
         recovery,
         %{intent_id: id, coordinator_session: session, lease_session: lease_session},
         {session, lease_session}
       )
       when is_reference(lease_session) do
    if recovery[Intent.key()]["intent_id"] == id,
      do: :ok,
      else: {:error, :stale_cancellation_intent}
  end

  defp token_matches(_, _, _), do: {:error, :stale_cancellation_delivery}

  defp persist(job, runtime, recovery, bucket) do
    updated = Map.put(recovery, Intent.key(), bucket)

    with :ok <- Intent.verify(updated, job),
         {:ok, _} <-
           Store.apply_progress_with_result(
             %{
               job_id: job.job_id,
               stage: job.status,
               progress: job.progress,
               iteration: job.iteration
             },
             job,
             runtime,
             Map.put(runtime, "_workflow_recovery", updated)
           ),
         do: :ok
  end
end
