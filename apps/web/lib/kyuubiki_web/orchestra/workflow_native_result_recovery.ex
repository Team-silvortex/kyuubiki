defmodule KyuubikiWeb.Orchestra.WorkflowNativeResultRecovery do
  @moduledoc "Reuses an observed successful native dispatch only through its original gated receipt."

  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorDispatchResult, as: Result
  alias KyuubikiWeb.Orchestra.OperatorTaskCompletion

  @reasons ~w(journal_unavailable dispatch_unconfirmed input_changed dispatch_identity_invalid receipt_unavailable receipt_invalid)a

  def blocking_reason?(reason), do: reason in @reasons

  def resolve(task, summary, context, opts) do
    if Keyword.get(opts, :recover_native_results, false) do
      recover(task, summary, context, opts)
    else
      :execute
    end
  end

  defp recover(task, summary, %{"job_id" => job}, opts)
       when is_binary(job) and job != "" do
    with :ok <- gate(opts),
         {:ok, records} <- lookup(task, opts),
         {:ok, selected} <- select(records, summary, job) do
      case selected do
        nil -> :execute
        record -> retrieve(record, summary, opts)
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
        {:ok, Enum.find(owned, &(&1["state"] == "observed_executed"))}
    end
  end

  defp retrieve(record, summary, opts) do
    query = Map.take(record, ~w(task_id task_digest attempt_id))
    result_opts = Keyword.take(opts, [:journal, :endpoints, :probe])

    case Result.fetch(query, result_opts) do
      {:ok,
       %{"status" => "receipt_recovered", "outcome" => "executed", "completion" => completion}} ->
        with {:ok, %{"status" => "executed", "result" => %{"result" => output}}} <-
               OperatorTaskCompletion.agent_receipt(summary, completion["result"]),
             :ok <- gate(opts) do
          {:ok, output}
        else
          {:error, {:operator_task_execution_receipt_invalid, _}} -> blocked(:receipt_invalid)
          {:error, _} = error -> error
          _ -> blocked(:receipt_invalid)
        end

      {:ok, %{"status" => "agent_receipt_invalid"}} ->
        blocked(:receipt_invalid)

      _ ->
        blocked(:receipt_unavailable)
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
