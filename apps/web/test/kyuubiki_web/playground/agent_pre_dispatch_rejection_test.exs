defmodule KyuubikiWeb.Playground.AgentPreDispatchRejectionTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal

  alias KyuubikiWeb.Orchestra.{
    OperatorTaskExecutionSummary,
    OperatorTaskIR,
    OperatorTaskReadiness
  }

  alias KyuubikiWeb.Playground.{AgentClient, AgentExecutionGate, AgentPool}
  alias KyuubikiWeb.TestSupport.{ControlledSolverAgent, FakePlaygroundAgent}

  @lan_denied "agent_lan_control_requires_explicit_trust"
  @codes ["agent_at_capacity", @lan_denied]

  setup do
    root = Path.join(System.tmp_dir!(), "admission-reply-#{System.unique_integer([:positive])}")
    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Journal)
    start_supervised!({Journal, root: root})
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, previous)
      AgentPool.reload()
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Journal)
      File.rm_rf!(root)
    end)

    id = "admission-reply-#{System.unique_integer([:positive])}"

    {:ok, task} =
      OperatorTaskIR.build("extract.thermal_result_diagnostics", %{}, %{}, task_id: id)

    %{task: task, root: root}
  end

  for code <- @codes,
      event <- ~w(progress heartbeat),
      policy <- [:idempotent, :checkpointed, :checkpoint_required] do
    @tag code: code, event: event, policy: policy
    test "#{event} then #{code} keeps managed task unknown under #{policy}", ctx do
      configure(ctx.task, [progress(ctx.event), rejection(ctx.code)])

      assert execute(ctx.task, ctx.policy) == {:error, :operator_task_dispatch_outcome_unknown}
      assert_single_request()
      assert {:ok, [%{"state" => "outcome_unknown"}]} = Journal.lookup_task(ctx.task["task_id"])

      assert {:error, :workflow_dispatch_outcome_unconfirmed} =
               Journal.workflow_replay_fence(ctx.task["task_id"], [ctx.task["task_id"]])

      assert_released()
      [first, peer] = AgentPool.endpoints()
      assert first.consecutive_failures == 1
      assert peer.consecutive_failures == 0
    end
  end

  for code <- @codes,
      event <- ~w(progress heartbeat),
      policy <- [:idempotent, :checkpointed, :checkpoint_required] do
    @tag code: code, event: event, policy: policy
    test "#{event} then #{code} cannot replay a legacy solve under #{policy}", ctx do
      configure(ctx.task, [progress(ctx.event), rejection(ctx.code)])

      assert {:error,
              {:agent_retry_blocked,
               %{reason_code: "agent_non_admission_conflict", retryable: false}}} =
               AgentClient.request("solve_bar_1d", %{}, fn _ -> :ok end,
                 retry_safety: ctx.policy,
                 request_timeout_ms: 1_000,
                 replay_checkpoint: checkpoint()
               )

      assert_single_request()
      assert {:ok, []} = Journal.lookup_task(ctx.task["task_id"])
      assert_released()
    end
  end

  for policy <- [:idempotent, :checkpointed, :checkpoint_required] do
    @tag policy: policy
    test "plain LAN denial is not dispatched and does not automatically try a peer under #{policy}",
         ctx do
      configure(ctx.task, [rejection(@lan_denied)])

      assert execute(ctx.task, ctx.policy) == {:error, {:rpc_error, @lan_denied, "not admitted"}}
      assert_single_request()

      assert {:ok, [%{"state" => "not_dispatched"} = record]} =
               Journal.lookup_task(ctx.task["task_id"])

      assert :ok = Journal.verify_observation(record)
      assert_released()
      for endpoint <- AgentPool.endpoints(), do: assert(endpoint.consecutive_failures == 0)

      # A distinct new task remains usable; the rejected task is not replayed.
      {:ok, other} =
        OperatorTaskIR.build("extract.thermal_result_diagnostics", %{}, %{},
          task_id: ctx.task["task_id"] <> "-next"
        )

      configure(other, [%{"ok" => true, "result" => receipt(other)}])
      assert {:ok, _} = execute(other, ctx.policy)
      assert_single_request()
      assert {:ok, [^record]} = Journal.lookup_task(ctx.task["task_id"])
    end
  end

  test "plain capacity rejection still permits one healthy peer without execution progress",
       ctx do
    configure(ctx.task, [rejection("agent_at_capacity")])
    assert {:ok, _} = execute(ctx.task, :checkpoint_required)
    assert_receive {:fake_agent_request, _}, 1_000
    assert_receive {:fake_agent_request, _}, 1_000
    refute_receive {:fake_agent_request, _}, 20
    assert {:ok, records} = Journal.lookup_task(ctx.task["task_id"])
    assert Enum.sort(Enum.map(records, & &1["state"])) == ["not_dispatched", "observed_executed"]
    assert_released()
  end

  for code <- @codes do
    @tag code: code
    test "#{code} cannot also carry a task failure receipt", ctx do
      failure = receipt(ctx.task)

      frame =
        put_in(rejection(ctx.code), ["error", "details"], %{
          "operator_task_failure_receipt" => failure
        })

      configure(ctx.task, [frame])
      assert execute(ctx.task, :idempotent) == {:error, :operator_task_dispatch_outcome_unknown}
      assert_single_request()
      assert {:ok, [%{"state" => "outcome_unknown"}]} = Journal.lookup_task(ctx.task["task_id"])
      [first, _] = AgentPool.endpoints()
      assert first.consecutive_failures == 1
      assert_released()
    end
  end

  test "an arbitrary RPC error cannot claim non-admission through message or details", ctx do
    frame = %{
      "ok" => false,
      "error" => %{
        "code" => "unknown_rejection",
        "message" => @lan_denied,
        "details" => %{"dispatched" => false}
      }
    }

    configure(ctx.task, [frame])
    assert execute(ctx.task, :idempotent) == {:error, :operator_task_dispatch_outcome_unknown}
    assert_single_request()
    assert {:ok, [%{"state" => "outcome_unknown"}]} = Journal.lookup_task(ctx.task["task_id"])
    assert_released()
  end

  test "contradictory admission remains unknown after journal reopen and healthy new work", ctx do
    configure(ctx.task, [progress("heartbeat"), rejection("agent_at_capacity")])
    assert execute(ctx.task, :idempotent) == {:error, :operator_task_dispatch_outcome_unknown}
    assert_single_request()
    assert {:ok, [record]} = Journal.lookup_task(ctx.task["task_id"])
    path = Path.join(ctx.root, record["attempt_id"] <> ".json")
    before = File.read!(path)

    stop_supervised!(Journal)
    start_supervised!({Journal, root: ctx.root})
    assert {:ok, [^record]} = Journal.lookup_task(ctx.task["task_id"])

    assert {:error, :workflow_dispatch_outcome_unconfirmed} =
             Journal.workflow_replay_fence(ctx.task["task_id"], [ctx.task["task_id"]])

    {:ok, other} =
      OperatorTaskIR.build("extract.thermal_result_diagnostics", %{}, %{},
        task_id: ctx.task["task_id"] <> "-after-reopen"
      )

    configure(other, [%{"ok" => true, "result" => receipt(other)}])
    assert {:ok, _} = execute(other, :checkpoint_required)
    assert_single_request()
    assert File.read!(path) == before
    assert {:ok, [^record]} = Journal.lookup_task(ctx.task["task_id"])
    assert_released()
  end

  test "late LAN denial cannot replace an already sealed unknown journal observation", ctx do
    start_supervised!({ControlledSolverAgent, {self(), :no_progress}})
    assert_receive {:fake_agent_ready, port}, 1_000
    endpoint = %{id: "delayed-denial", host: "127.0.0.1", port: port, capacity: 1}
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [endpoint])
    AgentPool.reload()
    dispatch = Task.async(fn -> execute(ctx.task, :idempotent) end)
    assert_receive {:solver_request, agent, request}, 1_000

    assert :ok =
             Journal.finish_dispatch(request["params"]["dispatch_attempt_id"], "outcome_unknown")

    assert {:ok, [record]} = Journal.lookup_task(ctx.task["task_id"])
    send(agent, {:reply, rejection(@lan_denied)})

    assert Task.await(dispatch) == {:error, :operator_task_dispatch_outcome_unknown}
    assert {:ok, [^record]} = Journal.lookup_task(ctx.task["task_id"])

    assert {:error, :workflow_dispatch_outcome_unconfirmed} =
             Journal.workflow_replay_fence(ctx.task["task_id"], [ctx.task["task_id"]])

    assert_released()
  end

  defp configure(task, frames) do
    endpoints = [agent(frames), agent([%{"ok" => true, "result" => receipt(task)}])]
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: endpoints)
    AgentPool.reload()
  end

  defp agent(frames) do
    id = "admission-peer-#{System.unique_integer([:positive])}"
    {:ok, pid} = FakePlaygroundAgent.start_link({:capture, self(), frames})
    on_exit(fn -> if Process.alive?(pid), do: Process.exit(pid, :kill) end)
    assert_receive {:fake_agent_ready, port}, 1_000
    %{id: id, host: "127.0.0.1", port: port, capacity: 1}
  end

  defp execute(task, policy) do
    AgentClient.run_operator_task_ir(task,
      mode: :execute,
      retry_safety: policy,
      request_timeout_ms: 1_000,
      on_dispatch_reserved: fn _ -> :ok end,
      replay_checkpoint: checkpoint()
    )
  end

  defp checkpoint,
    do: %{
      "operator_task_batch_checkpoint_verification_contract" =>
        "kyuubiki.operator_task_batch_checkpoint_verification/v1",
      "status" => "verified",
      "checkpoint_digest" => String.duplicate("a", 64)
    }

  defp assert_single_request do
    assert_receive {:fake_agent_request, _}, 1_000
    refute_receive {:fake_agent_request, _}, 20
  end

  defp assert_released do
    assert %{active_lease_count: 0, queued_request_count: 0} = AgentExecutionGate.snapshot()
  end

  defp progress(event), do: %{"event" => event, "progress" => %{"stage" => "agent-received"}}

  defp rejection(code),
    do: %{"ok" => false, "error" => %{"code" => code, "message" => "not admitted"}}

  defp receipt(task) do
    {:ok, summary} = OperatorTaskExecutionSummary.build(task)

    Map.merge(Map.take(summary, ~w(task_id task_digest operator_id program_id)), %{
      "operator_task_ir_status" => "executed",
      "execution_readiness" => OperatorTaskReadiness.local_executed(),
      "result" => %{"value" => 42}
    })
  end
end
