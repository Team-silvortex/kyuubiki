defmodule KyuubikiWeb.Playground.AgentTaskDispatch do
  @moduledoc false
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorTaskCompletion
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutionSummary
  alias KyuubikiWeb.Orchestra.OperatorTaskFailure
  alias KyuubikiWeb.Playground.AgentExecutionGate
  alias KyuubikiWeb.Playground.AgentRpcTransport

  def request(endpoint, id, request, on_progress, opts) do
    with :ok <- AgentExecutionGate.authorize_dispatch(id) do
      dispatch(endpoint, id, request, on_progress, opts)
    end
  end

  defp dispatch(
         endpoint,
         id,
         %{"method" => "run_operator_task_ir", "params" => params} = request,
         progress,
         opts
       ) do
    if params["mode"] == "execute" do
      with {:ok, attempt} <- Journal.begin_dispatch(request, endpoint),
           :ok <- persist_attempt(opts, attempt) do
        request = put_in(request, ["params", "dispatch_attempt_id"], attempt)
        result = AgentRpcTransport.request(endpoint, id, request, progress, opts)
        state = observed_state(params["task_ir"], result)

        case Journal.finish_dispatch(attempt, state) do
          :ok when state == "outcome_unknown" -> unconfirmed(result, opts)
          :ok -> result
          {:error, _} -> {:error, :operator_task_dispatch_outcome_unknown}
        end
      end
    else
      AgentRpcTransport.request(endpoint, id, request, progress, opts)
    end
  end

  defp dispatch(endpoint, id, request, progress, opts),
    do: AgentRpcTransport.request(endpoint, id, request, progress, opts)

  # Managed parents bind recovery to each reserved attempt. Their uncertainty must
  # not escape as a skippable error or a retry hint. Standalone routing is unchanged.
  defp unconfirmed(result, opts) do
    if is_function(Keyword.get(opts, :on_dispatch_reserved), 1),
      do: unconfirmed(result),
      else: result
  end

  # Preserve the transport cause for health accounting without permitting peer replay.
  defp unconfirmed({:error, {:operator_task_rpc_error, _, _, _}}),
    do: unconfirmed({:ok, nil})

  defp unconfirmed({:error, reason}),
    do: {:error, {:operator_task_dispatch_outcome_unknown, reason}}

  defp unconfirmed({:ok, _receipt}),
    do:
      {:error,
       {:operator_task_dispatch_outcome_unknown,
        {:agent_transport_failure, :protocol, :unverified_operator_task_receipt}}}

  # Keep an unconfirmed reservation on any callback failure. No RPC has been sent,
  # but a failed/aborted storage acknowledgement is not permission to erase evidence.
  defp persist_attempt(opts, attempt) do
    case Keyword.get(opts, :on_dispatch_reserved) do
      nil ->
        :ok

      callback when is_function(callback, 1) ->
        case callback.(attempt) do
          :ok -> :ok
          _ -> {:error, :operator_task_dispatch_evidence_unavailable}
        end

      _ ->
        {:error, :operator_task_dispatch_evidence_unavailable}
    end
  rescue
    _ -> {:error, :operator_task_dispatch_evidence_unavailable}
  catch
    _, _ -> {:error, :operator_task_dispatch_evidence_unavailable}
  end

  defp observed_state(_task, {:error, {:agent_transport_failure, :connect, _}}),
    do: "not_dispatched"

  defp observed_state(_task, {:error, {:rpc_error, "agent_at_capacity", _}}),
    do: "not_dispatched"

  defp observed_state(task, result) do
    with {:ok, summary} <- OperatorTaskExecutionSummary.build(task),
         {:ok, receipt} <- verified_receipt(summary, result),
         {:ok, completed} <- OperatorTaskCompletion.agent_receipt(summary, receipt) do
      "observed_" <> completed["status"]
    else
      _ -> "outcome_unknown"
    end
  end

  defp verified_receipt(_summary, {:ok, receipt}), do: {:ok, receipt}

  defp verified_receipt(summary, {:error, {:operator_task_rpc_error, code, message, failure}}),
    do: OperatorTaskFailure.agent_rpc_result(summary, code, message, failure)

  defp verified_receipt(_summary, _result), do: {:error, :unverified_completion}
end
