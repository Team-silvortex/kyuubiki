defmodule KyuubikiWeb.Api.OperatorDispatchApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.Orchestra.OperatorTaskReadiness
  alias KyuubikiWeb.TestSupport.CancellableSolverAgent

  test "journal storage failure blocks before any solver RPC" do
    start_agent("owner")

    root =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-blocked-dispatch-#{Base.encode16(:crypto.strong_rand_bytes(16))}"
      )

    File.write!(root, "not a directory")

    on_exit(fn ->
      replace_journal([])
      File.rm(root)
    end)

    replace_journal(root: root)
    response = execute(native_task())
    assert response.status == 422

    assert Jason.decode!(response.resp_body)["error_code"] ==
             "operator_task_dispatch_journal_unavailable"

    refute_receive {:agent_request, _, _, _}, 50
  end

  test "timed out TaskIR stays queryable by a fresh caller, targeting only its owner" do
    owner = start_agent("owner")
    peer = start_agent("peer")
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [owner, peer])
    AgentPool.reload()
    task = native_task()
    response = execute(task)
    assert response.status == 422
    assert_receive {:agent_request, "owner", handler, request}
    assert {:ok, [record]} = Journal.lookup(task["task_id"], digest(task))
    assert record["request_id"] == request["id"]
    assert record["state"] == "outcome_unknown"

    call = Task.async(fn -> inspect_dispatch(task) end)
    assert_receive {:agent_request, "owner", observer, %{"method" => "describe_agent"}}, 1_000

    send(
      observer,
      {:reply,
       %{
         "ok" => true,
         "result" => %{
           "lifecycle" => %{"process_instance_id" => "owner-boot"},
           "solver_control" => %{
             "schema_version" => "kyuubiki.agent-solver-control/v1",
             "available" => true,
             "active" => [
               %{
                 "request_id" => request["id"],
                 "generation" => 3,
                 "job_id" => task["task_id"],
                 "cancel_requested" => false
               }
             ]
           }
         }
       }}
    )

    inspected = Task.await(call)
    assert inspected.status == 200
    report = Jason.decode!(inspected.resp_body)

    assert get_in(report, ["attempts", Access.at(0), "observation", "status"]) ==
             "original_endpoint_reports_active_request"

    assert get_in(report, ["attempts", Access.at(0), "observation", "execution_target"]) ==
             %{
               "process_instance_id" => "owner-boot",
               "request_id" => request["id"],
               "generation" => 3,
               "job_id" => task["task_id"]
             }

    refute report["automatic_replay_authorized"]
    refute report["terminal_result_available"]
    refute_receive {:agent_request, "peer", _, _}, 20
    send(handler, {:reply, %{"ok" => true, "result" => receipt(task)}})
  end

  test "verified completion is compact history and is not queried or promoted to a result cache" do
    start_agent("owner")
    task = native_task()
    call = Task.async(fn -> execute(task, 2_000) end)
    assert_receive {:agent_request, "owner", handler, _}, 1_000
    send(handler, {:reply, %{"ok" => true, "result" => receipt(task)}})
    assert Task.await(call).status == 200
    report = inspect_dispatch(task) |> Map.fetch!(:resp_body) |> Jason.decode!()
    assert [attempt] = report["attempts"]
    assert attempt["state"] == "observed_executed"
    assert attempt["observation"]["outcome"] == "observed_executed"
    refute report["terminal_result_available"]
    refute_receive {:agent_request, "owner", _, _}, 20
  end

  test "wrong task completion cannot become retained success" do
    start_agent("owner")
    task = native_task()
    call = Task.async(fn -> execute(task, 2_000) end)
    assert_receive {:agent_request, "owner", handler, _}, 1_000

    send(
      handler,
      {:reply,
       %{
         "ok" => true,
         "result" => Map.put(receipt(task), "task_digest", String.duplicate("b", 64))
       }}
    )

    assert Task.await(call).status == 422
    assert {:ok, [record]} = Journal.lookup(task["task_id"], digest(task))
    assert record["state"] == "outcome_unknown"
  end

  test "inspection honors read authentication and rejects caller supplied destinations" do
    task = native_task()

    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "test-secret",
      protect_reads?: true
    )

    assert inspect_dispatch(task).status == 401

    conn =
      conn(
        :post,
        "/api/v1/operator-tasks/inspect-dispatch",
        Jason.encode!(%{
          "task_id" => task["task_id"],
          "task_digest" => digest(task),
          "host" => "untrusted"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("authorization", "Bearer test-secret")
      |> Router.call(@opts)

    assert conn.status == 422
    assert Jason.decode!(conn.resp_body)["error_code"] == "operator_task_dispatch_query_invalid"
  end

  test "empty registry inspection never falls back to an unrelated static endpoint" do
    endpoint = start_agent("fallback")
    task = native_task()
    request = %{"id" => "not-sent", "params" => %{"task_ir" => task}}
    assert {:ok, _} = Journal.begin_dispatch(request, endpoint)
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [endpoint], discovery: :registry)
    AgentPool.reload()
    assert AgentPool.inspection_endpoints() == []
    response = inspect_dispatch(task)
    assert response.status == 200
    report = Jason.decode!(response.resp_body)

    assert get_in(report, ["attempts", Access.at(0), "observation", "status"]) ==
             "original_endpoint_not_configured"

    refute_receive {:agent_request, _, _, _}, 20
  end

  test "result retrieval requires read authorization and never accepts a caller target" do
    task = native_task()

    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "test-secret",
      protect_reads?: true
    )

    body = %{
      "task_id" => task["task_id"],
      "task_digest" => digest(task),
      "attempt_id" => String.duplicate("1", 32)
    }

    request = fn payload, token ->
      connection =
        conn(:post, "/api/v1/operator-tasks/fetch-dispatch-result", Jason.encode!(payload))
        |> put_req_header("content-type", "application/json")

      connection =
        if token,
          do: put_req_header(connection, "authorization", "Bearer " <> token),
          else: connection

      Router.call(connection, @opts)
    end

    assert request.(body, nil).status == 401
    assert request.(body, "wrong").status == 401
    assert request.(Map.put(body, "host", "untrusted"), "test-secret").status == 422
    authorized = request.(body, "test-secret")
    assert authorized.status == 200
    report = Jason.decode!(authorized.resp_body)
    assert report["status"] == "no_retained_dispatch"
    refute report["automatic_replay_authorized"]
    refute report["publication_performed"]
    refute_receive {:agent_request, _, _, _}, 20
  end

  test "inspection rejects oversized Agent frames without echoing raw diagnostics" do
    endpoint = start_agent("owner")
    task = native_task()

    assert {:ok, _} =
             Journal.begin_dispatch(
               %{"id" => "not-sent", "params" => %{"task_ir" => task}},
               endpoint
             )

    call = Task.async(fn -> inspect_dispatch(task) end)
    assert_receive {:agent_request, "owner", observer, %{"method" => "describe_agent"}}, 1_000

    send(
      observer,
      {:reply, %{"ok" => true, "result" => %{"SECRET" => String.duplicate("x", 1_048_577)}}}
    )

    response = Task.await(call)
    assert response.status == 200
    report = Jason.decode!(response.resp_body)

    assert get_in(report, ["attempts", Access.at(0), "observation", "status"]) ==
             "original_endpoint_unreachable"

    refute response.resp_body =~ "SECRET"
    refute report["automatic_replay_authorized"]
  end

  defp execute(task, timeout \\ 60) do
    conn(
      :post,
      "/api/v1/operator-tasks/execute",
      Jason.encode!(%{
        "task" => task,
        "execution_budget" => %{
          "schema_version" => "kyuubiki.operator-task-request-budget/v1",
          "queue_timeout_ms" => 500,
          "request_timeout_ms" => timeout
        }
      })
    )
    |> put_req_header("content-type", "application/json")
    |> Router.call(@opts)
  end

  defp replace_journal(opts) do
    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Journal)
    :ok = Supervisor.delete_child(KyuubikiWeb.Supervisor, Journal)
    {:ok, _} = Supervisor.start_child(KyuubikiWeb.Supervisor, {Journal, opts})
  end

  defp inspect_dispatch(task) do
    conn(
      :post,
      "/api/v1/operator-tasks/inspect-dispatch",
      Jason.encode!(%{"task_id" => task["task_id"], "task_digest" => digest(task)})
    )
    |> put_req_header("content-type", "application/json")
    |> Router.call(@opts)
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
        task_id: "dispatch-#{Base.encode16(:crypto.strong_rand_bytes(8))}"
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

  defp digest(task), do: get_in(task, ["integrity", "task_digest"])

  defp receipt(task) do
    %{
      "task_id" => task["task_id"],
      "task_digest" => digest(task),
      "operator_id" => get_in(task, ["operator", "id"]),
      "program_id" => get_in(task, ["execution_program", "program_id"]),
      "operator_task_ir_status" => "executed",
      "execution_readiness" => OperatorTaskReadiness.local_executed(),
      "result" => %{"value" => 1.0}
    }
  end
end
