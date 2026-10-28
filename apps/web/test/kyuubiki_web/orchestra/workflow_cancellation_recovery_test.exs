defmodule KyuubikiWeb.Orchestra.WorkflowCancellationRecoveryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.{Persistence, Storage}
  alias KyuubikiWeb.Orchestra.WorkflowCancellationStore, as: CancellationStore
  alias KyuubikiWeb.Orchestra.WorkflowCancellationDelivery, as: Delivery
  alias KyuubikiWeb.Orchestra.WorkflowCancellationIntent, as: Intent
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Storage.DurableJson

  alias KyuubikiWeb.Playground.{
    AgentClient,
    AgentPool,
    AgentExecutionGate,
    AgentExecutionCancellation
  }

  alias KyuubikiWeb.TestSupport.{
    ScopedCancellationAgent,
    WorkflowCommitFixture,
    AnalysisCommitFault
  }

  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage

  @key "_workflow_recovery"
  @intent "cancellation_intent"

  setup do
    Outage.setup()
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, previous)
      AgentPool.reload()
    end)

    start_supervised!({ScopedCancellationAgent, {self(), "durable-cancel"}})
    assert_receive {:cancellable_agent_ready, "durable-cancel", port}

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{
          id: "durable-cancel",
          host: "127.0.0.1",
          port: port,
          capacity: 2,
          methods: ["solve_bar_1d"],
          agent_session_id: "original-session"
        }
      ]
    )

    AgentPool.reload()
    :ok
  end

  @tag skip: Storage.postgres?()
  test "failed retirement cannot cancel or change the original gate reservation" do
    f = WorkflowCommitFixture.claimed_job()
    task = dispatch(f.id)
    assert_receive {:agent_request, "durable-cancel", handler, _}, 2_000
    assert {:ok, captured} = AgentExecutionGate.observe_job_targets(f.id)
    drop = reject_result_update(f.id)
    assert {:error, _} = Coordinator.fail_observed(f.job, "write rejected")
    WorkflowCommitFixture.unchanged(f)
    assert {:ok, ^captured} = AgentExecutionGate.observe_job_targets(f.id)
    assert Task.yield(task, 0) == nil
    refute_receive {:agent_request, _, _, _}, 20
    drop.()
    assert {:ok, _} = Coordinator.fail_observed(f.job, "retry after storage repair")
    assert_receive {:agent_request, "durable-cancel", _, %{"method" => "cancel_execution"}}, 2_000
    settled(f.id)
    send(handler, {:reply, %{"ok" => true, "result" => %{}}})
    assert {:error, {:rpc_error, "cancelled", _}} = Task.await(task, 2_000)
  end

  test "restart preserves the bound generation even when the Agent currently reports another" do
    f = active_request()
    target = target(f, 999)

    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      retire(f)
      token = reserve(f.id)
      assert :ok = CancellationStore.access(f.id, {:bind, token, target})
      assert {:ok, ^target} = CancellationStore.access(f.id, {:authorize, token})
    end)

    WorkflowCommitFixture.restart_coordinator()
    assert_receive {:agent_request, "durable-cancel", _, cancel}, 2_000
    assert cancel["method"] == "cancel_execution"
    assert cancel["params"] == target
    runtime = wait_entry(f.id, &(&1["status"] == "target_not_observed"))
    assert hd(runtime[@key][@intent]["entries"])["attempts"] == 2
    refute runtime[@key]["state"] == "completed"
    release(f)
  end

  @tag skip: Storage.postgres?()
  test "reservation and target writes must succeed before authorization; lost ack retries the same target" do
    f = active_request()
    exact = target(f, 1)

    old_token =
      Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
        retire(f)
        {:ok, bucket} = CancellationStore.access(f.id, :load)
        id = KyuubikiWeb.Orchestra.WorkflowCancellationIntent.identity(hd(bucket["entries"]))
        drop = reject_result_update(f.id)
        assert {:error, _} = CancellationStore.access(f.id, {:reserve, bucket["intent_id"], id})
        assert {:ok, ^bucket} = CancellationStore.access(f.id, :load)
        drop.()
        token = reserve(f.id)
        drop = reject_result_update(f.id)
        assert {:error, _} = CancellationStore.access(f.id, {:bind, token, exact})

        assert {:error, :cancellation_target_not_bound} =
                 CancellationStore.access(f.id, {:authorize, token})

        refute_receive {:agent_request, _, _, _}, 20
        drop.()
        assert :ok = CancellationStore.access(f.id, {:bind, token, exact})
        [endpoint] = AgentPool.inspection_endpoints()
        assert %{"status" => "requested"} = AgentExecutionCancellation.deliver(endpoint, exact)
        assert_receive {:agent_request, "durable-cancel", _, first_cancel}
        assert first_cancel["params"] == exact
        drop = reject_result_update(f.id)
        assert {:error, _} = CancellationStore.access(f.id, {:finish, token, "requested"})
        assert {:ok, saved} = CancellationStore.access(f.id, :load)
        assert hd(saved["entries"])["status"] == "dispatching"
        assert hd(saved["entries"])["execution_target"] == exact
        drop.()
        token
      end)

    WorkflowCommitFixture.restart_coordinator()
    assert_receive {:agent_request, "durable-cancel", _, retry}, 2_000
    assert retry["params"] == exact
    settled(f.id)

    assert {:error, :stale_cancellation_delivery} =
             CancellationStore.access(f.id, {:finish, old_token, "requested"})

    release(f)
  end

  test "malformed delivery token does not crash the coordinator" do
    f = active_request()

    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      retire(f)
      token = reserve(f.id)
      pid = Process.whereis(Coordinator)

      for malformed <- [nil, %{}, Map.delete(token, :id), Map.delete(token, :delivery_id)] do
        assert {:error, _} = CancellationStore.access(f.id, {:authorize, malformed})
        assert {:error, _} = CancellationStore.access(f.id, {:finish, malformed, "requested"})
        assert Process.whereis(Coordinator) == pid
      end
    end)

    release(f)
  end

  test "a modified durable intent is not sent on restart" do
    f = active_request()

    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      retire(f)
      {:ok, runtime} = AnalysisResultStore.get(f.id)
      changed = put_in(runtime, [@key, @intent, "sha256"], String.duplicate("0", 64))
      assert :ok = AnalysisResultStore.compare_and_swap(f.id, runtime, changed)
    end)

    WorkflowCommitFixture.restart_coordinator()
    Coordinator.recover_now()
    assert {:error, :invalid_workflow_cancellation_intent} = CancellationStore.access(f.id, :load)
    refute_receive {:agent_request, _, _, _}, 100
    release(f)
  end

  test "changed endpoint session is never contacted and bounded retries stay unresolved" do
    f = active_request()
    Outage.without(KyuubikiWeb.TaskSupervisor, fn -> retire(f) end)
    config = Application.get_env(:kyuubiki_web, AgentPool)
    [endpoint] = config[:endpoints]

    Application.put_env(
      :kyuubiki_web,
      AgentPool,
      Keyword.put(config, :endpoints, [Map.put(endpoint, :agent_session_id, "replacement")])
    )

    AgentPool.reload()

    for attempt <- 1..3 do
      Coordinator.recover_now()
      wait_entry(f.id, &(&1["attempts"] == attempt and &1["status"] != "dispatching"))
      wait_worker()
    end

    {:ok, runtime} = AnalysisResultStore.get(f.id)
    [entry] = runtime[@key][@intent]["entries"]
    assert entry["status"] == "exhausted"
    assert entry["reason"] == "original_endpoint_not_configured"
    summary = KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope.public_summary(runtime[@key])
    assert summary["cancellation"]["status"] == "unresolved"
    Coordinator.recover_now()
    wait_worker()
    assert {:ok, ^runtime} = AnalysisResultStore.get(f.id)
    refute_receive {:agent_request, _, _, _}, 50
    release(f)
  end

  for trigger <- [:explicit, :restart],
      phase <- [:fresh_failure, :spent_pending, :dispatch_with_reason] do
    @tag :state_consistency
    test "#{trigger} recovery leaves a resealed #{phase} record untouched" do
      f = active_request()

      changed =
        Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
          retire(f)
          corrupt_phase(f, unquote(phase))
        end)

      case unquote(trigger) do
        :explicit -> Coordinator.recover_now()
        :restart -> WorkflowCommitFixture.restart_coordinator()
      end

      assert {:error, :invalid_workflow_cancellation_intent} =
               CancellationStore.access(f.id, :load)

      wait_worker()
      assert {:ok, ^changed} = AnalysisResultStore.get(f.id)
      assert Intent.summary(changed[@key])["status"] == "invalid"
      refute_receive {:agent_request, _, _, _}, 50
      release(f)
    end
  end

  for phase <- [:fresh_failure, :spent_pending] do
    @tag :state_consistency
    test "invalid #{phase} sibling does not block a healthy cancellation" do
      bad = active_request()
      good = active_request()

      changed =
        Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
          retire(bad)
          retire(good)
          corrupt_phase(bad, unquote(phase))
        end)

      {:ok, worker} = Delivery.kick([bad.id, good.id])
      ref = Process.monitor(worker)
      assert_receive {:agent_request, "durable-cancel", _, cancel}, 2_000
      assert cancel["method"] == "cancel_execution"
      assert cancel["params"] == target(good, 2)
      runtime = settled(good.id)
      assert hd(runtime[@key][@intent]["entries"])["attempts"] == 1
      assert_receive {:DOWN, ^ref, :process, ^worker, reason}, 2_000
      assert reason in [:normal, :noproc]
      assert {:ok, ^changed} = AnalysisResultStore.get(bad.id)
      refute_receive {:agent_request, _, _, _}, 50
      release(bad)
      release(good)
    end
  end

  defp corrupt_phase(f, phase) do
    attempts =
      case phase do
        :fresh_failure -> 0
        :spent_pending -> 3
        :dispatch_with_reason -> 1
      end

    for _ <- List.duplicate(:reserve, attempts), do: reserve(f.id)
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    bucket = runtime[@key][@intent]
    [entry] = bucket["entries"]

    entry =
      entry
      |> Map.put("status", if(phase == :dispatch_with_reason, do: "dispatching", else: "pending"))
      |> Map.put("reason", "cancellation_outcome_unknown")

    bucket = Map.put(bucket, "entries", [entry])
    digest = :crypto.hash(:sha256, DurableJson.encode!(Map.delete(bucket, "sha256")))
    bucket = Map.put(bucket, "sha256", Base.encode16(digest, case: :lower))
    changed = put_in(runtime, [@key, @intent], bucket)
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, runtime, changed)
    changed
  end

  defp active_request do
    f = WorkflowCommitFixture.claimed_job()
    task = dispatch(f.id)
    assert_receive {:agent_request, "durable-cancel", handler, request}, 2_000
    Map.merge(f, %{task: task, handler: handler, request: request})
  end

  defp retire(f),
    do: assert({:ok, _} = Coordinator.fail_observed(f.job, "durable watchdog timeout"))

  defp reserve(id) do
    assert {:ok, bucket} = CancellationStore.access(id, :load)
    entry = hd(bucket["entries"])
    identity = {entry["endpoint_fingerprint"], entry["request_id"]}
    assert {:ok, token} = CancellationStore.access(id, {:reserve, bucket["intent_id"], identity})
    token
  end

  defp target(f, generation),
    do: %{
      "job_id" => f.id,
      "request_id" => f.request["id"],
      "generation" => generation,
      "process_instance_id" => "durable-cancel-process"
    }

  defp release(f) do
    send(f.handler, {:reply, %{"ok" => true, "result" => %{}}})
    assert {:error, {:rpc_error, "cancelled", _}} = Task.await(f.task, 2_000)
    wait_worker()
  end

  defp wait_worker do
    Outage.eventually(fn ->
      Registry.lookup(
        KyuubikiWeb.Orchestra.WorkflowRunnerRegistry,
        {KyuubikiWeb.Orchestra.WorkflowCancellationDelivery, :sweep}
      ) == []
    end)
  end

  defp reject_result_update(id) do
    if Storage.sqlite?() do
      AnalysisCommitFault.reject_result_update(id)
    else
      path = Persistence.analysis_state_path() <> ".next"
      File.mkdir_p!(path)
      File.write!(Path.join(path, "blocker"), "injected fault")
      cleanup = fn -> File.rm_rf!(path) end
      on_exit(cleanup)
      cleanup
    end
  end

  test "watchdog commit survives loss before cancellation delivery without selecting later same-job work" do
    f = WorkflowCommitFixture.claimed_job()
    original = dispatch(f.id)
    assert_receive {:agent_request, "durable-cancel", first, request}, 2_000

    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      assert {:ok, _} = Coordinator.fail_observed(f.job, "durable watchdog timeout")
      assert {:ok, runtime} = AnalysisResultStore.get(f.id)
      assert %{"entries" => [_]} = intent = runtime[@key][@intent]
      assert hd(intent["entries"])["request_id"] == request["id"]
      refute Jason.encode!(intent) =~ "127.0.0.1"
      refute_receive {:agent_request, _, _, _}, 20
    end)

    Task.shutdown(original, :brutal_kill)
    later = dispatch(f.id)
    assert_receive {:agent_request, "durable-cancel", second, later_request}, 2_000
    assert later_request["id"] != request["id"]
    WorkflowCommitFixture.restart_coordinator()
    assert_receive {:agent_request, "durable-cancel", _, cancel}, 2_000
    assert cancel["method"] == "cancel_execution"
    assert cancel["params"]["request_id"] == request["id"]
    assert cancel["params"]["generation"] == 1
    assert cancel["params"]["process_instance_id"] == "durable-cancel-process"
    settled(f.id)
    WorkflowCommitFixture.restart_coordinator()
    Coordinator.recover_now()
    refute_receive {:agent_request, _, _, _}, 50
    send(second, {:reply, %{"ok" => true, "result" => %{"unaffected" => true}}})
    assert {:ok, %{"unaffected" => true}} = Task.await(later, 2_000)
    send(first, {:reply, %{"ok" => true, "result" => %{}}})
  end

  defp dispatch(id),
    do:
      Task.async(fn ->
        AgentClient.request("solve_bar_1d", %{}, fn _ -> :ok end, job_id: id)
      end)

  defp settled(id), do: wait_entry(id, &(&1["status"] == "requested"))

  defp wait_entry(id, predicate, attempts \\ 200)
  defp wait_entry(_, _, 0), do: flunk("durable cancellation did not settle")

  defp wait_entry(id, predicate, attempts) do
    {:ok, runtime} = AnalysisResultStore.get(id)

    case get_in(runtime, [@key, @intent, "entries"]) do
      [entry] ->
        if predicate.(entry) do
          runtime
        else
          Process.sleep(5)
          wait_entry(id, predicate, attempts - 1)
        end

      _ ->
        Process.sleep(5)
        wait_entry(id, predicate, attempts - 1)
    end
  end
end
