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
      with {:ok, attempt} <- Journal.begin_dispatch(request, endpoint) do
        request = put_in(request, ["params", "dispatch_attempt_id"], attempt)
        result = AgentRpcTransport.request(endpoint, id, request, progress, opts)

        case Journal.finish_dispatch(attempt, observed_state(params["task_ir"], result)) do
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
