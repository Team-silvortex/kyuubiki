defmodule KyuubikiWeb.Api.OperatorTaskBudgetApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.Orchestra.OperatorTaskReadiness
  alias KyuubikiWeb.Orchestra.OperatorTaskRequestBudget
  alias KyuubikiWeb.Playground.AgentExecutionGate
  alias KyuubikiWeb.TestSupport.CancellableSolverAgent

  test "invalid waiting budgets fail before dispatch and never silently fall back" do
    start_agent("owner")
    task = native_task()
    valid = budget(120_000, 120_000)

    invalid = [nil, %{}, Map.delete(valid, "request_timeout_ms")]

    invalid =
      invalid ++
        Enum.map(
          [
            {"schema_version", "other/v1"},
            {"queue_timeout_ms", 0},
            {"request_timeout_ms", 600_001},
            {"request_timeout_ms", -1},
            {"request_timeout_ms", 1.0},
            {"request_timeout_ms", true},
            {"request_timeout_ms", "120000"},
            {"unexpected", 1}
          ],
          fn {key, value} -> Map.put(valid, key, value) end
        )

    for invalid_budget <- invalid do
      response = request(task, invalid_budget)
      assert response.status == 422

      assert Jason.decode!(response.resp_body)["error_code"] ==
               "operator_task_execution_budget_invalid"
    end

    refute_receive {:agent_request, _, _, _}, 50
    assert AgentExecutionGate.snapshot().active_lease_count == 0
  end

  test "waiting budget is echoed separately without changing the signed task or Agent payload" do
    start_agent("owner")
    task = native_task()
    execution_budget = budget(500, 2_000)
    call = Task.async(fn -> request(task, execution_budget) end)

    assert_receive {:agent_request, "owner", handler, captured}, 1_000

    assert Regex.match?(~r/\A[0-9a-f]{32}\z/, captured["params"]["dispatch_attempt_id"])

    assert Map.delete(captured["params"], "dispatch_attempt_id") == %{
             "task_ir" => task,
             "mode" => "execute",
             "job_id" => task["task_id"]
           }

    Process.sleep(150)
    send(handler, {:reply, %{"ok" => true, "result" => receipt(task)}})
    response = Task.await(call, 2_000)
    assert response.status == 200
    result = Jason.decode!(response.resp_body)
    assert result["execution_budget"] == execution_budget
    assert result["task_digest"] == get_in(task, ["integrity", "task_digest"])
    assert result["result"] == receipt(task)
  end

  test "short execution budget reaches the transport and does not replay on an idle peer" do
    owner = start_agent("owner")
    peer = start_agent("peer")
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [owner, peer])
    AgentPool.reload()
    task = native_task()
    started = System.monotonic_time(:millisecond)
    response = request(task, budget(500, 75))
    elapsed = System.monotonic_time(:millisecond) - started

    assert_receive {:agent_request, "owner", handler, %{"method" => "run_operator_task_ir"}}
    assert response.status == 422
    assert response.resp_body =~ "agent_transport_timeout"
    assert response.resp_body =~ "checkpoint_required"
    assert elapsed >= 50 and elapsed < 1_500
    refute_receive {:agent_request, "peer", _, _}, 50
    send(handler, {:reply, %{"ok" => true, "result" => receipt(task)}})
    assert AgentExecutionGate.snapshot().active_lease_count == 0
  end

  test "queue budget expires before dispatch and releases only its own waiter" do
    endpoint = start_agent("owner")

    assert {:ok, _, _} =
             AgentExecutionGate.acquire([endpoint], "held-capacity", 1_000, "held-job")

    try do
      started = System.monotonic_time(:millisecond)
      response = request(native_task(), budget(75, 2_000))
      elapsed = System.monotonic_time(:millisecond) - started
      assert response.status == 422
      assert response.resp_body =~ "agent_queue_timeout"
      assert elapsed >= 50 and elapsed < 1_500
      assert AgentExecutionGate.snapshot().active_lease_count == 1
      assert AgentExecutionGate.snapshot().queued_request_count == 0
      refute_receive {:agent_request, _, _, _}, 50
    after
      AgentExecutionGate.release("held-capacity")
    end
  end

  test "absent budget preserves service defaults and maximum phases remain bounded" do
    assert {:ok, []} = OperatorTaskRequestBudget.options(%{})

    assert {:ok,
            [queue_timeout_ms: 1, request_timeout_ms: 600_000, retry_safety: :checkpoint_required]} =
             OperatorTaskRequestBudget.options(%{"execution_budget" => budget(1, 600_000)})
  end

  defp request(task, execution_budget) do
    conn(
      :post,
      "/api/v1/operator-tasks/execute",
      Jason.encode!(%{"task" => task, "execution_budget" => execution_budget})
    )
    |> put_req_header("content-type", "application/json")
    |> Router.call(@opts)
  end

  defp budget(queue, execution) do
    %{
      "schema_version" => "kyuubiki.operator-task-request-budget/v1",
      "queue_timeout_ms" => queue,
      "request_timeout_ms" => execution
    }
  end

  defp start_agent(name) do
    start_supervised!({CancellableSolverAgent, {self(), name}})
    assert_receive {:cancellable_agent_ready, ^name, port}

    endpoint = %{
      id: name,
      host: "127.0.0.1",
      port: port,
      capacity: 1,
      methods: ["run_operator_task_ir"],
      capabilities: ["solver_rpc"]
    }

    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [endpoint])
    AgentPool.reload()
    endpoint
  end

  defp native_task do
    {:ok, task} =
      OperatorTaskIR.build(
        "solve.bar_1d",
        %{
          "length" => 1.0,
          "area" => 0.01,
          "youngs_modulus" => 210.0e9,
          "elements" => 4,
          "tip_force" => 1000.0
        },
        %{},
        task_id: "budget-native-bar"
      )

    task
    |> put_in(["execution_program", "package_ref"], nil)
    |> put_in(["operator", "execution", "package_ref"], nil)
    |> Map.put("runtime_hints", %{
      "authority_mode" => "agent_local",
      "execution_mode" => "agent_native",
      "agent_fetchable" => false,
      "cache_scope" => "none",
      "operator_kind" => "solver",
      "required_capabilities" => ["solver_rpc"]
    })
    |> then(&put_in(&1, ["integrity", "task_digest"], OperatorTaskIR.compute_task_digest(&1)))
  end

  defp receipt(task) do
    %{
      "task_id" => task["task_id"],
      "task_digest" => get_in(task, ["integrity", "task_digest"]),
      "operator_id" => get_in(task, ["operator", "id"]),
      "program_id" => get_in(task, ["execution_program", "program_id"]),
      "operator_task_ir_status" => "executed",
      "execution_readiness" => OperatorTaskReadiness.local_executed(),
      "result" => %{"tip_displacement" => 4.761904761904762e-7}
    }
  end
end
