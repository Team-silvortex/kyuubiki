defmodule KyuubikiWeb.Playground.AgentTaskUncertainDispatchTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal

  alias KyuubikiWeb.Orchestra.{
    OperatorTaskExecutionSummary,
    OperatorTaskIR,
    OperatorTaskReadiness
  }

  alias KyuubikiWeb.Playground.{AgentClient, AgentExecutionGate, AgentPool}
  alias KyuubikiWeb.TestSupport.ControlledSolverAgent

  setup do
    root =
      Path.join(System.tmp_dir!(), "uncertain-dispatch-#{System.unique_integer([:positive])}")

    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Journal)
    start_supervised!({Journal, root: root})
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, previous)
      AgentPool.reload()
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Journal)
      File.rm_rf!(root)
    end)

    first = agent("uncertain-first")
    peer = agent("unused-peer")
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [first, peer])
    AgentPool.reload()
    id = "uncertain-#{System.unique_integer([:positive])}"

    {:ok, task} =
      OperatorTaskIR.build("extract.thermal_result_diagnostics", %{}, %{}, task_id: id)

    %{task: task, first: first, peer: peer}
  end

  for policy <- [:idempotent, :checkpointed, :checkpoint_required],
      outcome <- [
        :disconnect,
        :timeout,
        :invalid_receipt,
        :invalid_failure,
        :rpc_error,
        :returned_error,
        :raise,
        :throw,
        :exit
      ] do
    @tag policy: policy, outcome: outcome
    test "#{outcome} stays unconfirmed despite #{policy} routing", ctx do
      opts =
        [
          mode: :execute,
          retry_safety: ctx.policy,
          request_timeout_ms: if(ctx.outcome == :timeout, do: 150, else: 2_000),
          replay_checkpoint: %{
            "operator_task_batch_checkpoint_verification_contract" =>
              "kyuubiki.operator_task_batch_checkpoint_verification/v1",
            "status" => "verified",
            "checkpoint_digest" => String.duplicate("a", 64)
          }
        ]

      dispatch =
        Task.async(fn ->
          execute_managed(ctx.task, opts, callback(ctx.outcome))
        end)

      assert_receive {:solver_request, agent, request}, 2_000
      respond(agent, ctx.outcome)
      assert Task.await(dispatch) == {:error, :operator_task_dispatch_outcome_unknown}
      send(agent, :disconnect)
      refute_receive {:solver_request, _, _}, 30

      assert {:ok, [record]} = Journal.lookup_task(ctx.task["task_id"])
      assert record["state"] == "outcome_unknown"
      assert record["attempt_id"] == request["params"]["dispatch_attempt_id"]
      assert %{active_lease_count: 0, queued_request_count: 0} = AgentExecutionGate.snapshot()

      [first, peer] = AgentPool.endpoints()
      health_failure = ctx.outcome in [:disconnect, :timeout, :invalid_receipt, :invalid_failure]
      assert first.consecutive_failures == if(health_failure, do: 1, else: 0)
      assert peer.consecutive_failures == 0
    end
  end

  for rejection <- [:connect, :capacity] do
    @tag rejection: rejection
    test "#{rejection} before execution still permits a different Agent", ctx do
      if ctx.rejection == :connect, do: stop_supervised!(ctx.first.id)
      dispatch = Task.async(fn -> execute_managed(ctx.task) end)

      if ctx.rejection == :capacity do
        assert_receive {:solver_request, first, _}, 2_000

        send(
          first,
          {:reply,
           %{
             "ok" => false,
             "error" => %{"code" => "agent_at_capacity", "message" => "not admitted"}
           }}
        )
      end

      assert_receive {:solver_request, peer, _}, 2_000
      receipt = receipt(ctx.task, "executed")
      send(peer, {:reply, %{"ok" => true, "result" => receipt}})
      assert Task.await(dispatch) == {:ok, receipt}
      assert {:ok, records} = Journal.lookup_task(ctx.task["task_id"])

      assert Enum.sort(Enum.map(records, & &1["state"])) == [
               "not_dispatched",
               "observed_executed"
             ]

      assert AgentExecutionGate.snapshot().active_lease_count == 0
      refute_receive {:solver_request, _, _}, 20
    end
  end

  for status <- ~w(executed blocked failed) do
    @tag receipt_status: status
    test "verified #{status} receipt is not misclassified as uncertain", ctx do
      dispatch = Task.async(fn -> execute_managed(ctx.task) end)
      assert_receive {:solver_request, agent, _}, 2_000
      receipt = receipt(ctx.task, ctx.receipt_status)
      send(agent, {:reply, %{"ok" => true, "result" => receipt}})
      assert Task.await(dispatch) == {:ok, receipt}
      assert {:ok, [record]} = Journal.lookup_task(ctx.task["task_id"])
      assert record["state"] == "observed_" <> ctx.receipt_status
      refute_receive {:solver_request, _, _}, 20
      for endpoint <- AgentPool.endpoints(), do: assert(endpoint.consecutive_failures == 0)
    end
  end

  test "concurrent cancellation does not hide a disconnected native attempt", ctx do
    dispatch = Task.async(fn -> execute_managed(ctx.task) end)
    assert_receive {:solver_request, agent, _}, 2_000
    assert {:ok, _} = AgentExecutionGate.cancel_job_targets(ctx.task["task_id"])
    send(agent, :disconnect)
    assert Task.await(dispatch) == {:error, :operator_task_dispatch_outcome_unknown}
    assert {:ok, [%{"state" => "outcome_unknown"}]} = Journal.lookup_task(ctx.task["task_id"])
    refute_receive {:solver_request, _, _}, 20
    assert AgentExecutionGate.snapshot().active_lease_count == 0
  end

  test "an uncertain task does not block unrelated work on a healthy peer", ctx do
    dispatch = Task.async(fn -> execute_managed(ctx.task) end)
    assert_receive {:solver_request, agent, _}, 2_000
    send(agent, :disconnect)
    assert Task.await(dispatch) == {:error, :operator_task_dispatch_outcome_unknown}
    refute_receive {:solver_request, _, _}, 20

    {:ok, other} =
      OperatorTaskIR.build("extract.thermal_result_diagnostics", %{}, %{}, task_id: "other-task")

    next = Task.async(fn -> execute_managed(other) end)
    assert_receive {:solver_request, peer, request}, 2_000
    assert request["params"]["task_ir"]["task_id"] == "other-task"
    receipt = receipt(other, "executed")
    send(peer, {:reply, %{"ok" => true, "result" => receipt}})
    assert Task.await(next) == {:ok, receipt}
    assert {:ok, [%{"state" => "outcome_unknown"}]} = Journal.lookup_task(ctx.task["task_id"])
  end

  defp execute_managed(task, opts \\ [], progress \\ fn _ -> :ok end) do
    opts = Keyword.merge(opts, mode: :execute, on_dispatch_reserved: fn _attempt -> :ok end)
    AgentClient.run_operator_task_ir(task, opts, progress)
  end

  defp agent(id) do
    start_supervised!(
      Supervisor.child_spec({ControlledSolverAgent, {self(), %{"stage" => "agent-received"}}},
        id: id
      )
    )

    assert_receive {:fake_agent_ready, port}

    %{
      id: id,
      host: "127.0.0.1",
      port: port,
      methods: ["run_operator_task_ir"],
      capabilities: ["solver_rpc"],
      operator_package_runtime: %{"ready" => false}
    }
  end

  defp callback(outcome) do
    fn
      %{"stage" => "agent-received"} ->
        case outcome do
          :returned_error -> {:error, "PRIVATE callback detail"}
          :raise -> raise "PRIVATE callback detail"
          :throw -> throw("PRIVATE callback detail")
          :exit -> exit("PRIVATE callback detail")
          _ -> :ok
        end

      _ ->
        :ok
    end
  end

  defp respond(agent, :invalid_receipt),
    do: send(agent, {:reply, %{"ok" => true, "result" => %{}}})

  defp respond(agent, :invalid_failure) do
    send(
      agent,
      {:reply,
       %{
         "ok" => false,
         "error" => %{
           "code" => "failed",
           "message" => "PRIVATE",
           "details" => %{
             "operator_task_failure_receipt" => %{"task_id" => "different-task"}
           }
         }
       }}
    )
  end

  defp respond(agent, :rpc_error),
    do:
      send(
        agent,
        {:reply,
         %{
           "ok" => false,
           "error" => %{"code" => "failed", "message" => "PRIVATE"}
         }}
      )

  defp respond(_agent, :timeout), do: :ok
  defp respond(agent, _outcome), do: send(agent, :disconnect)

  defp receipt(task, status) do
    {:ok, summary} = OperatorTaskExecutionSummary.build(task)

    Map.merge(Map.take(summary, ~w(task_id task_digest operator_id program_id)), %{
      "operator_task_ir_status" => status,
      "execution_readiness" =>
        if(status == "executed",
          do: OperatorTaskReadiness.local_executed(),
          else: OperatorTaskReadiness.local_failed(:test_rejection)
        ),
      "result" => %{"controlled" => 42}
    })
  end
end
