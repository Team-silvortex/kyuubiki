defmodule KyuubikiWeb.Playground.AgentTaskDispatchObservationTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal

  alias KyuubikiWeb.Orchestra.{
    OperatorTaskExecutionSummary,
    OperatorTaskIR,
    OperatorTaskReadiness
  }

  alias KyuubikiWeb.Playground.{AgentExecutionGate, AgentRpcTransport, AgentTaskDispatch}
  alias KyuubikiWeb.TestSupport.ControlledSolverAgent

  for state <- ~w(outcome_unknown observed_executed) do
    test "a delayed TCP success respects an already sealed #{state} observation" do
      id = "dispatch-observation-#{System.unique_integer([:positive])}"

      {:ok, task_ir} =
        OperatorTaskIR.build("extract.thermal_result_diagnostics", %{}, %{}, task_id: id)

      {:ok, summary} = OperatorTaskExecutionSummary.build(task_ir)
      start_supervised!({ControlledSolverAgent, {self(), %{"job_id" => id}}})
      assert_receive {:fake_agent_ready, port}
      endpoint = %{id: id, host: "127.0.0.1", port: port, capacity: 1}

      request = %{
        "rpc_version" => 1,
        "id" => id,
        "method" => "run_operator_task_ir",
        "params" => %{"task_ir" => task_ir, "mode" => "execute", "job_id" => id}
      }

      dispatch =
        Task.async(fn ->
          {:ok, _, _} = AgentExecutionGate.acquire([endpoint], id, 1_000, id)

          try do
            AgentTaskDispatch.request(endpoint, id, request, fn _ -> :ok end,
              request_timeout_ms: 3_000
            )
          after
            AgentExecutionGate.release(id)
          end
        end)

      assert_receive {:solver_request, agent, sent}, 2_000
      attempt = sent["params"]["dispatch_attempt_id"]
      assert :ok = Journal.finish_dispatch(attempt, unquote(state))
      assert {:ok, [before]} = Journal.lookup_task(id)
      path = Path.join(Journal.root(), attempt <> ".json")
      bytes = File.read!(path)

      result =
        Map.merge(Map.take(summary, ~w(task_id task_digest operator_id program_id)), %{
          "operator_task_ir_status" => "executed",
          "execution_readiness" => OperatorTaskReadiness.local_executed(),
          "result" => %{"controlled" => 42}
        })

      send(agent, {:reply, %{"ok" => true, "result" => result}})
      outcome = Task.await(dispatch)

      if unquote(state) == "outcome_unknown" do
        assert outcome == {:error, :operator_task_dispatch_outcome_unknown}
        assert AgentRpcTransport.local_failure?(:operator_task_dispatch_outcome_unknown)

        assert {:error, :workflow_dispatch_outcome_unconfirmed} =
                 Journal.workflow_replay_fence(id, [id])
      else
        assert outcome == {:ok, result}
      end

      assert {:ok, [^before]} = Journal.lookup_task(id)
      assert File.read!(path) == bytes
      assert AgentExecutionGate.snapshot([endpoint]).active_lease_count == 0
      refute_receive {:solver_request, _, _}, 0

      assert {:ok, other} = Journal.begin_dispatch(request, endpoint)
      assert :ok = Journal.finish_dispatch(other, "not_dispatched")
    end
  end
end
