defmodule KyuubikiWeb.Api.OperatorDispatchStorageApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.TestSupport.CancellableSolverAgent

  @policy Path.expand("../../../../../schemas/agent-task-result-retention.schema.json", __DIR__)
          |> File.read!()
          |> Jason.decode!()
          |> get_in(["$defs", "policy", "properties"])
          |> Map.new(fn {key, spec} -> {key, spec["const"]} end)

  setup do
    root =
      Path.join(System.tmp_dir!(), "dispatch-api-storage-#{System.unique_integer([:positive])}")

    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Journal)
    start_supervised!({Journal, root: root})

    on_exit(fn ->
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Journal)
      File.rm_rf!(root)
    end)

    start_supervised!({CancellableSolverAgent, {self(), "owner"}})
    assert_receive {:cancellable_agent_ready, "owner", port}
    endpoint = %{id: "owner", host: "127.0.0.1", port: port}
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [endpoint])
    AgentPool.reload()
    digest = String.duplicate("a", 64)

    request = %{
      "id" => "original-rpc",
      "params" => %{
        "job_id" => "parent",
        "task_ir" => %{
          "task_id" => "task",
          "integrity" => %{"task_digest" => digest},
          "operator" => %{"id" => "op"},
          "execution_program" => %{"program_id" => "op"}
        }
      }
    }

    {:ok, attempt} = Journal.begin_dispatch(request, endpoint)
    query = %{"task_id" => "task", "task_digest" => digest, "attempt_id" => attempt}
    {:ok, [record]} = Journal.lookup_task("task")
    %{root: root, path: Path.join(root, attempt <> ".json"), query: query, record: record}
  end

  for fault <- [:deleted, :corrupt, :replacement, :transition, :restart_empty, :restart_same] do
    test "public receipt lookup revalidates after TCP reply across #{fault}", ctx do
      reader = Task.async(fn -> fetch_result(ctx.query) end)
      assert_receive {:agent_request, "owner", agent, request}, 2_000
      assert request["method"] == "fetch_operator_task_result"
      assert request["params"]["attempt_id"] == ctx.query["attempt_id"]

      change_during_fetch(ctx, unquote(fault))

      after_fault = File.read(ctx.path)
      send(agent, {:reply, %{"ok" => true, "result" => retained(ctx.record)}})
      response = Task.await(reader)
      body = Jason.decode!(response.resp_body)

      if unquote(fault) == :restart_same do
        assert response.status == 200
        assert body["status"] == "receipt_recovered"
        assert body["completion"]["result"]["result"] == %{"value" => 42}
        refute body["automatic_replay_authorized"]
        refute body["publication_performed"]
      else
        assert response.status == 422

        assert body["error_code"] in ~w(operator_task_dispatch_journal_invalid operator_task_dispatch_journal_unavailable operator_task_dispatch_observation_changed)

        refute Map.has_key?(body, "completion")
        refute response.resp_body =~ "SECRET"
        refute response.resp_body =~ ctx.root
      end

      assert File.read(ctx.path) == after_fault
      assert Store.list() == []
      refute_receive {:agent_request, _, _, _}, 20
    end
  end

  for status <- ~w(not_retained result_not_retained) do
    test "public #{status} after an Agent incarnation change remains unknown without execution",
         ctx do
      before = File.read!(ctx.path)
      reader = Task.async(fn -> fetch_result(ctx.query) end)
      assert_receive {:agent_request, "owner", agent, request}, 2_000
      assert request["method"] == "fetch_operator_task_result"

      result =
        retained(ctx.record)
        |> Map.put("process_instance_id", "new-agent-process")
        |> Map.put("generation", nil)
        |> Map.put("status", unquote(status))
        |> Map.put("response", nil)

      send(agent, {:reply, %{"ok" => true, "result" => result}})
      response = Task.await(reader)
      assert response.status == 200
      body = Jason.decode!(response.resp_body)
      assert body["status"] == unquote(status)
      assert body["outcome"] == "unknown"
      assert is_nil(body["completion"])
      refute body["automatic_replay_authorized"]
      refute body["publication_performed"]
      assert Store.list() == []
      assert File.read!(ctx.path) == before
      refute_receive {:agent_request, _, _, _}, 20
    end
  end

  for action <- ~w(inspect-dispatch fetch-dispatch-result cancel-dispatch),
      fault <- [:deleted, :corrupt, :replacement] do
    test "#{action} refuses live #{fault} journal evidence before any Agent RPC", ctx do
      case unquote(fault) do
        :deleted ->
          File.rm!(ctx.path)

        :corrupt ->
          File.write!(ctx.path, "SECRET corrupt input")

        :replacement ->
          {:ok, [record]} = Journal.lookup_task("task")
          assert :ok = Files.write(ctx.root, Map.put(record, "state", "not_dispatched"), record)
      end

      before = File.read(ctx.path)

      payload =
        case unquote(action) do
          "inspect-dispatch" ->
            Map.delete(ctx.query, "attempt_id")

          "fetch-dispatch-result" ->
            ctx.query

          "cancel-dispatch" ->
            Map.put(ctx.query, "execution_target", %{
              "job_id" => "parent",
              "request_id" => "original-rpc",
              "process_instance_id" => "original-process",
              "generation" => 1
            })
        end

      response =
        conn(:post, "/api/v1/operator-tasks/" <> unquote(action), Jason.encode!(payload))
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert response.status == 422

      assert Jason.decode!(response.resp_body)["error_code"] in ~w(operator_task_dispatch_journal_invalid operator_task_dispatch_journal_unavailable)

      refute response.resp_body =~ "SECRET"
      refute response.resp_body =~ ctx.root
      refute_receive {:agent_request, _, _, _}, 20
      assert File.read(ctx.path) == before
      assert Process.alive?(Process.whereis(Journal))
    end
  end

  defp change_during_fetch(ctx, fault) do
    case fault do
      :deleted ->
        File.rm!(ctx.path)

      :corrupt ->
        File.write!(ctx.path, "SECRET invalid observation")

      :replacement ->
        assert :ok =
                 Files.write(
                   ctx.root,
                   Map.put(ctx.record, "state", "outcome_unknown"),
                   ctx.record
                 )

      :transition ->
        assert :ok = Journal.finish_dispatch(ctx.query["attempt_id"], "outcome_unknown")

      restart ->
        assert :ok = stop_supervised(Journal)
        if restart == :restart_empty, do: File.rm_rf!(ctx.root)
        start_supervised!({Journal, root: ctx.root})
    end
  end

  defp fetch_result(query) do
    conn(:post, "/api/v1/operator-tasks/fetch-dispatch-result", Jason.encode!(query))
    |> put_req_header("content-type", "application/json")
    |> Router.call(@opts)
  end

  defp retained(record) do
    identity =
      Map.take(record, ~w(attempt_id request_id task_id task_digest operator_id program_id))

    receipt =
      Map.merge(Map.take(record, ~w(task_id task_digest operator_id program_id)), %{
        "operator_task_ir_status" => "executed",
        "execution_readiness" => %{"status" => "executed", "ready_to_dispatch" => true},
        "result" => %{"value" => 42}
      })

    Map.merge(identity, %{
      "schema_version" => "kyuubiki.agent-task-result-retention/v1",
      "process_instance_id" => "original-agent-process",
      "generation" => 1,
      "status" => "receipt_retained",
      "automatic_replay_authorized" => false,
      "retention_policy" => @policy,
      "response" => %{
        "rpc_version" => 1,
        "id" => record["request_id"],
        "ok" => true,
        "result" => receipt
      }
    })
  end
end
