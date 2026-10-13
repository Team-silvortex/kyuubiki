defmodule KyuubikiWeb.Orchestra.WorkflowNativeResultRecovery do
  @moduledoc "Reuses an observed successful native dispatch only through its original gated receipt."

  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorDispatchResult, as: Result
  alias KyuubikiWeb.Orchestra.OperatorTaskCompletion
  alias KyuubikiWeb.Orchestra.WorkflowNativeDispatchEvidence
  alias KyuubikiWeb.Orchestra.WorkflowNativeResultCheckpoint, as: Checkpoint

  @reasons ~w(journal_unavailable dispatch_unconfirmed input_changed dispatch_identity_invalid dispatch_attempt_ambiguous dispatch_observation_changed dispatch_evidence_missing dispatch_evidence_invalid dispatch_evidence_commit_failed receipt_unavailable receipt_invalid result_checkpoint_invalid result_checkpoint_unavailable result_checkpoint_commit_failed)a

  def blocking_reason?(reason), do: reason in @reasons

  def retain(summary, context, receipt, opts) do
    if is_nil(Keyword.get(opts, :native_result_checkpoint)) do
      :ok
    else
      job = context["job_id"]

      with :ok <- gate(opts),
           :ok <- WorkflowNativeDispatchEvidence.check(summary, opts),
           {:ok, records} <- lookup(summary, opts),
           {:ok, %{} = record} <- select(records, summary, job),
           :ok <- Checkpoint.commit(record, receipt, opts),
           :ok <- revalidate_selection(record, summary, job, opts),
           :ok <- gate(opts) do
        :ok
      else
        {:ok, nil} -> blocked(:dispatch_observation_changed)
        {:error, _} = error -> error
      end
    end
  end

  def resolve(task, summary, context, opts) do
    with :ok <- WorkflowNativeDispatchEvidence.check(summary, opts) do
      if Keyword.get(opts, :recover_native_results, false) do
        recover(task, summary, context, opts)
      else
        :execute
      end
    end
  end

  defp recover(task, summary, %{"job_id" => job}, opts)
       when is_binary(job) and job != "" do
    with :ok <- gate(opts),
         {:ok, records} <- lookup(task, opts),
         {:ok, selected} <- select(records, summary, job) do
      case selected do
        nil -> :execute
        record -> retrieve(record, summary, job, opts)
      end
    end
  end

  defp recover(_task, _summary, _context, _opts), do: blocked(:dispatch_identity_invalid)

  defp lookup(task, opts) do
    case Journal.lookup_task(task["task_id"], Keyword.get(opts, :journal, Journal)) do
      {:ok, records} -> {:ok, records}
      {:error, _} -> blocked(:journal_unavailable)
    end
  end

  defp select(records, summary, job) do
    # Legacy task names are insufficient authority to attach an output to a parent job.
    owned = Enum.filter(records, &(Files.job_id(&1) == job))

    cond do
      Enum.any?(records, fn record ->
        record["state"] in ~w(dispatch_boundary_unconfirmed outcome_unknown) and
            (Files.job_id(record) == job or is_nil(Files.job_id(record)))
      end) ->
        blocked(:dispatch_unconfirmed)

      Enum.any?(owned, &(&1["task_digest"] != summary["task_digest"])) ->
        blocked(:input_changed)

      Enum.any?(owned, fn record ->
        Enum.any?(~w(task_id operator_id program_id), &(record[&1] != summary[&1]))
      end) ->
        blocked(:dispatch_identity_invalid)

      true ->
        select_success(owned)
    end
  end

  defp select_success(owned) do
    dispatched = Enum.reject(owned, &(&1["state"] == "not_dispatched"))

    if Enum.any?(dispatched, &(&1["state"] == "observed_executed")) do
      # Wall-clock ordering is not authority to choose between separate attempts.
      case dispatched do
        [record] -> {:ok, record}
        _ -> blocked(:dispatch_attempt_ambiguous)
      end
    else
      {:ok, nil}
    end
  end

  defp retrieve(record, summary, job, opts) do
    case Checkpoint.fetch(record, opts) do
      {:ok, receipt} -> accept(record, summary, job, receipt, opts, false)
      :missing -> retrieve_original(record, summary, job, opts)
      {:error, _} = error -> error
    end
  end

  defp retrieve_original(record, summary, job, opts) do
    query = Map.take(record, ~w(task_id task_digest attempt_id))
    result_opts = Keyword.take(opts, [:journal, :endpoints, :probe])

    case Result.fetch(query, result_opts) do
      {:ok,
       %{"status" => "receipt_recovered", "outcome" => "executed", "completion" => completion}} ->
        accept(record, summary, job, completion["result"], opts, true)

      {:ok, %{"status" => "agent_receipt_invalid"}} ->
        blocked(:receipt_invalid)

      {:error, :operator_task_dispatch_observation_changed} ->
        blocked(:dispatch_observation_changed)

      {:error, reason}
      when reason in [
             :operator_task_dispatch_journal_invalid,
             :operator_task_dispatch_journal_unavailable
           ] ->
        blocked(:journal_unavailable)

      _ ->
        blocked(:receipt_unavailable)
    end
  end

  defp accept(record, summary, job, receipt, opts, retain?) do
    with {:ok, %{"status" => "executed", "result" => %{"result" => output}}} <-
           OperatorTaskCompletion.agent_receipt(summary, receipt),
         :ok <- revalidate_selection(record, summary, job, opts),
         :ok <- gate(opts),
         :ok <- if(retain?, do: Checkpoint.commit(record, receipt, opts), else: :ok) do
      # A commit callback may take time; recheck the owner and journal before release.
      if retain? and not is_nil(Keyword.get(opts, :native_result_checkpoint)) do
        with :ok <- revalidate_selection(record, summary, job, opts),
             :ok <- gate(opts),
             do: {:ok, output}
      else
        {:ok, output}
      end
    else
      {:error, {:operator_task_execution_receipt_invalid, _}} -> blocked(:receipt_invalid)
      {:error, _} = error -> error
      _ -> blocked(:receipt_invalid)
    end
  end

  defp revalidate_selection(record, summary, job, opts) do
    with :ok <- WorkflowNativeDispatchEvidence.check(summary, opts),
         {:ok, records} <- lookup(record, opts),
         {:ok, selected} <- select(records, summary, job) do
      if selected == record, do: :ok, else: blocked(:dispatch_observation_changed)
    end
  end

  defp gate(opts) do
    case Keyword.get(opts, :before_dispatch, fn -> :ok end).() do
      :ok -> :ok
      {:error, _} = error -> error
      _ -> {:error, :invalid_native_workflow_operator_request}
    end
  end

  defp blocked(reason), do: {:error, {:native_workflow_recovery_blocked, reason}}
end
