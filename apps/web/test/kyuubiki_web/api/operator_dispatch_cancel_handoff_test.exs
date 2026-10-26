defmodule KyuubikiWeb.Api.OperatorDispatchCancelHandoffTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.TestSupport.{CancellableSolverAgent, ScopedCancellationAgent}

  setup tags do
    root = Path.join(System.tmp_dir!(), "cancel-handoff-#{System.unique_integer([:positive])}")
    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Journal)
    start_supervised!({Journal, root: root})

    on_exit(fn ->
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Journal)
      File.rm_rf!(root)
    end)

    agent = if tags[:manual_ack], do: CancellableSolverAgent, else: ScopedCancellationAgent
    start_supervised!({agent, {self(), "owner"}})
    assert_receive {:cancellable_agent_ready, "owner", port}
    endpoint = %{id: "owner", host: "127.0.0.1", port: port, agent_session_id: "session"}
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [endpoint])
    :ok = AgentPool.reload()
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
    {:ok, [record]} = Journal.lookup_task("task")

    query = %{
      "task_id" => "task",
      "task_digest" => digest,
      "attempt_id" => attempt,
      "execution_target" => %{
        "process_instance_id" => "owner-process",
        "request_id" => "original-rpc",
        "job_id" => "parent",
        "generation" => 1
      }
    }

    %{
      root: root,
      path: Path.join(root, attempt <> ".json"),
      record: record,
      request: request,
      query: query,
      endpoint: endpoint
    }
  end

  for fault <- [:deleted, :corrupt, :replacement, :partial, :missing_root] do
    test "#{fault} during endpoint lookup blocks cancellation before any TCP send", ctx do
      response = during_lookup(ctx, fn -> damage(ctx, unquote(fault)) end)
      assert response.status == 422
      body = Jason.decode!(response.resp_body)

      assert body["error_code"] in ~w(operator_task_dispatch_journal_invalid operator_task_dispatch_journal_unavailable)

      refute Map.has_key?(body, "cancel_registered")
      refute response.resp_body =~ "SECRET"
      refute response.resp_body =~ ctx.root
      assert_fault_preserved(ctx)
      refute_receive {:agent_request, _, _, _}, 20
      assert Store.list() == []
    end
  end

  for state <-
        ~w(observed_executed observed_failed observed_blocked not_dispatched outcome_unknown) do
    test "concurrent #{state} prevents the stale cancel and leaves a fresh explicit request usable",
         ctx do
      response =
        during_lookup(ctx, fn ->
          assert :ok = Journal.finish_dispatch(ctx.query["attempt_id"], unquote(state))
        end)

      assert response.status == 422

      assert Jason.decode!(response.resp_body)["error_code"] ==
               "operator_task_dispatch_observation_changed"

      assert_fault_preserved(ctx)
      refute_receive {:agent_request, _, _, _}, 20
      assert {:ok, [current]} = Journal.lookup_task("task")
      assert current["state"] == unquote(state)

      fresh = cancel(ctx.query)
      assert fresh.status == 200
      receipt = Jason.decode!(fresh.resp_body)

      if unquote(state) == "outcome_unknown" do
        assert receipt["status"] == "target_not_observed"
        assert_exact_send(ctx)
      else
        expected =
          if unquote(state) == "not_dispatched",
            do: "not_dispatched",
            else: "retained_terminal_dispatch"

        assert receipt["status"] == expected
        refute receipt["delivery_attempted"]
      end

      refute receipt["cancel_registered"]
      refute receipt["execution_terminal_confirmed"]
      refute receipt["automatic_replay_authorized"]
      refute_receive {:agent_request, _, _, _}, 20
    end
  end

  for mode <- [:empty, :same, :stopped] do
    test "#{mode} journal restart during endpoint lookup never substitutes missing evidence",
         ctx do
      response =
        during_lookup(ctx, fn ->
          assert :ok = stop_supervised(Journal)
          if unquote(mode) == :empty, do: File.rm_rf!(ctx.root)
          if unquote(mode) != :stopped, do: start_supervised!({Journal, root: ctx.root})
        end)

      if unquote(mode) == :same do
        assert response.status == 200
        assert Jason.decode!(response.resp_body)["status"] == "target_not_observed"
        assert_exact_send(ctx)
      else
        assert response.status == 422

        expected =
          if unquote(mode) == :empty,
            do: "operator_task_dispatch_observation_changed",
            else: "operator_task_dispatch_journal_unavailable"

        assert Jason.decode!(response.resp_body)["error_code"] == expected
      end

      assert_fault_preserved(ctx)
      refute_receive {:agent_request, _, _, _}, 20
    end
  end

  for control <- [:new_attempt, :unrelated_damage, :same_observation] do
    test "#{control} preserves cancellation of the exact selected attempt", ctx do
      assert :ok = Journal.finish_dispatch(ctx.query["attempt_id"], "outcome_unknown")
      other = put_in(ctx.request, ~w(params task_ir task_id), "unrelated")
      {:ok, other_id} = Journal.begin_dispatch(other, ctx.endpoint)

      response =
        during_lookup(ctx, fn ->
          case unquote(control) do
            :new_attempt ->
              assert {:ok, _} = Journal.begin_dispatch(ctx.request, ctx.endpoint)

            :unrelated_damage ->
              File.write!(Path.join(ctx.root, other_id <> ".json"), "SECRET unrelated payload")

            :same_observation ->
              assert :ok = Journal.finish_dispatch(ctx.query["attempt_id"], "outcome_unknown")
          end
        end)

      assert response.status == 200
      receipt = Jason.decode!(response.resp_body)
      assert receipt["status"] == "target_not_observed"
      assert receipt["delivery_attempted"]
      refute receipt["automatic_replay_authorized"]
      assert_exact_send(ctx)
      assert_fault_preserved(ctx)
      refute_receive {:agent_request, _, _, _}, 20
    end
  end

  for change <- [:removed, :new_session] do
    test "#{change} endpoint while lookup is queued does not retarget cancellation", ctx do
      response =
        during_lookup(ctx, fn ->
          endpoints =
            if unquote(change) == :removed,
              do: [],
              else: [%{ctx.endpoint | agent_session_id: "new-session"}]

          :sys.replace_state(AgentPool, &%{&1 | endpoints: endpoints})
        end)

      assert response.status == 200
      receipt = Jason.decode!(response.resp_body)
      assert receipt["status"] == "original_endpoint_not_configured"
      refute receipt["delivery_attempted"]
      assert_fault_preserved(ctx)
      refute_receive {:agent_request, _, _, _}, 20
    end
  end

  for outcome <- [:registered, :not_observed, :disconnected] do
    @tag manual_ack: true
    test "storage loss after sending retains #{outcome} acknowledgement semantics without retry",
         ctx do
      call = Task.async(fn -> cancel(ctx.query) end)
      assert_receive {:agent_request, "owner", handler, request}, 2_000
      assert request["method"] == "cancel_execution"
      assert request["params"] === ctx.query["execution_target"]
      File.rm!(ctx.path)

      if unquote(outcome) == :disconnected do
        Process.exit(handler, :kill)
      else
        send(
          handler,
          {:reply,
           %{
             "ok" => true,
             "result" => ack(ctx.query["execution_target"], unquote(outcome) == :registered)
           }}
        )
      end

      response = Task.await(call)
      assert response.status == 200
      receipt = Jason.decode!(response.resp_body)
      assert receipt["delivery_attempted"]

      case unquote(outcome) do
        :registered ->
          assert receipt["status"] == "requested"
          assert receipt["cancel_registered"]

        :not_observed ->
          assert receipt["status"] == "target_not_observed"
          refute receipt["cancel_registered"]

        :disconnected ->
          assert receipt["status"] == "cancellation_outcome_unknown"
          assert is_nil(receipt["cancel_registered"])
      end

      for flag <-
            ~w(execution_terminal_confirmed automatic_replay_authorized journal_mutation_performed publication_performed job_wide_fallback_performed),
          do: refute(receipt[flag])

      refute File.exists?(ctx.path)
      assert {:error, :operator_task_dispatch_journal_invalid} = Journal.lookup_task("task")
      refute_receive {:agent_request, _, _, _}, 20
      assert Store.list() == []
    end
  end

  defp during_lookup(ctx, change) do
    pool = Process.whereis(AgentPool)
    :ok = :sys.suspend(pool)

    call =
      try do
        task = Task.async(fn -> cancel(ctx.query) end)
        await_lookup(pool, task.pid, 100)
        change.()
        send(self(), {:fault_bytes, File.read(ctx.path)})
        task
      after
        :sys.resume(pool)
      end

    Task.await(call)
  end

  defp await_lookup(_, _, 0), do: flunk("cancellation did not reach endpoint lookup")

  defp await_lookup(pool, caller, attempts) do
    {:messages, messages} = Process.info(pool, :messages)

    pending =
      Enum.any?(messages, fn
        {:"$gen_call", {^caller, _}, :inspection_endpoints} -> true
        _ -> false
      end)

    unless pending do
      receive do
      after
        10 -> await_lookup(pool, caller, attempts - 1)
      end
    end
  end

  defp assert_fault_preserved(ctx) do
    assert_receive {:fault_bytes, bytes}
    assert File.read(ctx.path) == bytes
  end

  defp assert_exact_send(ctx) do
    assert_receive {:agent_request, "owner", _, request}
    assert request["method"] == "cancel_execution"
    assert request["params"] === ctx.query["execution_target"]
  end

  defp cancel(query) do
    conn(:post, "/api/v1/operator-tasks/cancel-dispatch", Jason.encode!(query))
    |> put_req_header("content-type", "application/json")
    |> Router.call(@opts)
  end

  defp damage(ctx, :deleted), do: File.rm!(ctx.path)
  defp damage(ctx, :corrupt), do: File.write!(ctx.path, "SECRET damaged observation")

  defp damage(ctx, :partial),
    do: File.write!(ctx.path <> ".next", "SECRET incomplete replacement")

  defp damage(ctx, :missing_root), do: File.rm_rf!(ctx.root)

  defp damage(ctx, :replacement),
    do:
      assert(
        :ok = Files.write(ctx.root, Map.put(ctx.record, "state", "outcome_unknown"), ctx.record)
      )

  defp ack(target, matched),
    do: %{
      "schema_version" => "kyuubiki.agent-execution-cancellation/v1",
      "execution_target" => target,
      "status" => if(matched, do: "requested", else: "target_not_observed"),
      "cancel_registered" => matched,
      "execution_terminal_confirmed" => false,
      "pending_cancellation_created" => false,
      "operator_package_cleanup_performed" => false,
      "automatic_replay_authorized" => false
    }
end
