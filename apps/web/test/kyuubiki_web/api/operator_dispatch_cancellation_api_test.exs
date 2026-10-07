defmodule KyuubikiWeb.Api.OperatorDispatchCancellationApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.TestSupport.CancellableSolverAgent

  test "public cancellation sends one exact RPC to the original owner not its peer" do
    owner = start_agent("owner")
    peer = start_agent("peer")
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [owner, peer])
    AgentPool.reload()
    query = retained_query(owner)
    {:ok, before} = Journal.lookup(query["task_id"], query["task_digest"])
    call = Task.async(fn -> cancel(query) end)
    assert_receive {:agent_request, "owner", handler, request}, 1_000
    assert request["method"] == "cancel_execution"
    assert request["params"] == query["execution_target"]
    refute request["id"] == query["execution_target"]["request_id"]
    send(handler, {:reply, %{"ok" => true, "result" => ack(query["execution_target"])}})
    response = Task.await(call)
    assert response.status == 200
    report = Jason.decode!(response.resp_body)
    assert report["status"] == "requested"
    assert report["cancel_registered"]
    refute report["execution_terminal_confirmed"]
    refute report["automatic_replay_authorized"]
    refute report["job_wide_fallback_performed"]
    assert {:ok, ^before} = Journal.lookup(query["task_id"], query["task_digest"])
    refute_receive {:agent_request, "peer", _, _}, 20
    refute_receive {:agent_request, "owner", _, _}, 20
  end

  test "write authentication and target validation precede all Agent effects" do
    endpoint = start_agent("owner")
    query = retained_query(endpoint)

    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "test-secret",
      protect_reads?: false
    )

    assert cancel(query).status == 401
    assert cancel(query, "wrong").status == 401

    for invalid <- [
          Map.put(query, "host", "untrusted"),
          put_in(query, ~w(execution_target generation), 0),
          put_in(query, ~w(execution_target request_id), "foreign-request")
        ] do
      assert cancel(invalid, "test-secret").status == 422
    end

    refute_receive {:agent_request, _, _, _}, 20
  end

  test "empty discovery registry does not fall back to static Agents and malformed ack stays unknown" do
    endpoint = start_agent("owner")
    query = retained_query(endpoint)
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [endpoint], discovery: :registry)
    AgentPool.reload()
    report = cancel(query) |> Map.fetch!(:resp_body) |> Jason.decode!()
    assert report["status"] == "original_endpoint_not_configured"
    refute report["delivery_attempted"]
    refute_receive {:agent_request, _, _, _}, 20
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [endpoint])
    AgentPool.reload()
    call = Task.async(fn -> cancel(query) end)
    assert_receive {:agent_request, "owner", handler, _}, 1_000
    invalid = ack(query["execution_target"]) |> Map.put("execution_terminal_confirmed", true)
    send(handler, {:reply, %{"ok" => true, "result" => invalid}})
    report = Task.await(call) |> Map.fetch!(:resp_body) |> Jason.decode!()
    assert report["status"] == "cancellation_outcome_unknown"
    assert report["uncertainty_reason"] == "agent_acknowledgement_invalid"
    assert is_nil(report["cancel_registered"])
    assert is_nil(report["agent_acknowledgement"])
    refute report["execution_terminal_confirmed"]
    refute_receive {:agent_request, _, _, _}, 20
  end

  test "oversized cancellation acknowledgement stays unknown without diagnostic echo or fallback" do
    endpoint = start_agent("owner")
    query = retained_query(endpoint)
    call = Task.async(fn -> cancel(query) end)
    assert_receive {:agent_request, "owner", handler, _}, 1_000
    oversized = Map.put(ack(query["execution_target"]), "SECRET", String.duplicate("x", 65_537))
    send(handler, {:reply, %{"ok" => true, "result" => oversized}})
    response = Task.await(call)
    assert response.status == 200
    report = Jason.decode!(response.resp_body)
    assert report["status"] == "cancellation_outcome_unknown"
    assert report["uncertainty_reason"] == "original_endpoint_unreachable"
    assert is_nil(report["cancel_registered"])
    assert is_nil(report["agent_acknowledgement"])
    refute response.resp_body =~ "SECRET"
    refute_receive {:agent_request, _, _, _}, 20
  end

  defp cancel(query, token \\ nil) do
    connection =
      conn(:post, "/api/v1/operator-tasks/cancel-dispatch", Jason.encode!(query))
      |> put_req_header("content-type", "application/json")

    connection =
      if token,
        do: put_req_header(connection, "authorization", "Bearer " <> token),
        else: connection

    Router.call(connection, @opts)
  end

  defp retained_query(endpoint) do
    id = "cancel-task-#{Base.encode16(:crypto.strong_rand_bytes(8))}"
    digest = String.duplicate("a", 64)

    request = %{
      "id" => "original-#{id}",
      "params" => %{
        "task_ir" => %{
          "task_id" => id,
          "integrity" => %{"task_digest" => digest},
          "operator" => %{"id" => "op"},
          "execution_program" => %{"program_id" => "op"}
        }
      }
    }

    {:ok, attempt} = Journal.begin_dispatch(request, endpoint)

    %{
      "task_id" => id,
      "task_digest" => digest,
      "attempt_id" => attempt,
      "execution_target" => %{
        "process_instance_id" => "original-process",
        "request_id" => request["id"],
        "generation" => 3,
        "job_id" => id
      }
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

  defp ack(target),
    do: %{
      "schema_version" => "kyuubiki.agent-execution-cancellation/v1",
      "execution_target" => target,
      "status" => "requested",
      "cancel_registered" => true,
      "execution_terminal_confirmed" => false,
      "pending_cancellation_created" => false,
      "operator_package_cleanup_performed" => false,
      "automatic_replay_authorized" => false
    }
end
