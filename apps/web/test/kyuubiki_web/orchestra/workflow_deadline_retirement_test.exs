defmodule KyuubikiWeb.Orchestra.WorkflowDeadlineRetirementTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{
    Analysis,
    AnalysisResultStore,
    Persistence,
    Storage,
    WorkflowNativeOperatorRuntime
  }

  alias KyuubikiWeb.Jobs.{Store, Watchdog}

  alias KyuubikiWeb.Orchestra.{
    LeaseStore,
    OperatorTaskExecutionSummary,
    OperatorTaskReadiness,
    WorkflowCancellationDelivery,
    WorkflowJobRunner
  }

  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Playground.{AgentClient, AgentExecutionGate, AgentPool}
  alias KyuubikiWeb.TestSupport.{AnalysisCommitFault, ScopedCancellationAgent}
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage

  @key "_workflow_recovery"
  @intent "cancellation_intent"
  @registry KyuubikiWeb.Orchestra.WorkflowRunnerRegistry
  @sweep {WorkflowCancellationDelivery, :sweep}

  setup do
    Outage.setup()

    root =
      Path.join(System.tmp_dir!(), "deadline-retirement-#{System.unique_integer([:positive])}")

    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Journal)
    start_supervised!({Journal, root: root})
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])
    native = Application.get_env(:kyuubiki_web, WorkflowNativeOperatorRuntime, [])
    Application.put_env(:kyuubiki_web, WorkflowNativeOperatorRuntime, client: AgentClient)
    watchdog = Application.get_env(:kyuubiki_web, Watchdog)

    Application.put_env(
      :kyuubiki_web,
      Watchdog,
      Keyword.put(watchdog, :workflow_timeout_ms, 1_000)
    )

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, previous)
      Application.put_env(:kyuubiki_web, WorkflowNativeOperatorRuntime, native)
      AgentPool.reload()
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Journal)
      File.rm_rf!(root)
    end)

    start_supervised!({ScopedCancellationAgent, {self(), "deadline"}})
    assert_receive {:cancellable_agent_ready, "deadline", port}

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{
          id: "deadline",
          host: "127.0.0.1",
          port: port,
          capacity: 2,
          methods: ["run_operator_task_ir", "solve_bar_1d"],
          agent_session_id: "deadline-session"
        }
      ]
    )

    AgentPool.reload()
    :ok
  end

  test "runner deadline retains intent across loss before delivery and isolates later same-job work" do
    assert {:ok, _} = Registry.register(@registry, @sweep, nil)
    {id, handler, request} = submit()
    assert {:ok, runner} = WorkflowJobRunner.running(id)
    ref = Process.monitor(runner)
    runtime = terminal(id)
    assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
    assert runtime[@key]["state"] == "recovery_blocked"
    [entry] = runtime[@key][@intent]["entries"]
    assert entry["request_id"] == request["id"]
    assert entry["attempts"] == 0
    assert entry["status"] == "pending"
    refute_receive {:agent_request, _, _, _}, 20

    later =
      Task.async(fn -> AgentClient.request("solve_bar_1d", %{}, fn _ -> :ok end, job_id: id) end)

    assert_receive {:agent_request, "deadline", later_handler, later_request}, 2_000
    refute later_request["id"] == request["id"]
    Registry.unregister(@registry, @sweep)
    Fixture.restart_coordinator()
    assert_receive {:agent_request, "deadline", _, cancel}, 2_000
    assert cancel["method"] == "cancel_execution"
    assert cancel["params"]["request_id"] == request["id"]
    assert cancel["params"]["generation"] == 1
    acknowledged(id)
    send(later_handler, {:reply, %{"ok" => true, "result" => %{"unaffected" => true}}})
    assert {:ok, %{"unaffected" => true}} = Task.await(later, 2_000)
    send(handler, {:reply, receipt(request)})
    Fixture.restart_coordinator()
    assert Coordinator.recover_now()["recovered"] == 0
    refute_receive {:agent_request, _, _, _}, 50
  end

  for late_reply? <- [false, true] do
    @tag skip: Storage.postgres?()
    test "rejected deadline write preserves its waiter and cannot accept late success=#{late_reply?}" do
      {id, handler, request} = submit()
      assert {:ok, runner} = WorkflowJobRunner.running(id)
      ref = Process.monitor(runner)
      assert {:ok, before} = AgentExecutionGate.observe_job_targets(id)
      drop = reject_result_update(id)
      # Cross the configured runner deadline while the durable commit is rejected.
      Process.sleep(1_200)
      assert Process.alive?(runner)
      assert {:ok, %{status: :solving}} = Store.get(id)
      assert {:ok, pending} = AnalysisResultStore.get(id)
      assert pending[@key]["state"] == "running"
      refute Map.has_key?(pending[@key], @intent)
      assert {:ok, ^before} = AgentExecutionGate.observe_job_targets(id)
      refute_receive {:agent_request, _, _, _}, 20

      if unquote(late_reply?) do
        send(handler, {:reply, receipt(request)})

        Outage.eventually(fn ->
          match?({:ok, %{requests: []}}, AgentExecutionGate.observe_job_targets(id))
        end)

        assert Process.alive?(runner)
      end

      drop.()
      final = terminal(id)
      assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
      assert {:ok, %{status: :failed, message: message}} = Store.get(id)
      assert message =~ "workflow execution timed out"
      refute get_in(final, ["artifacts", "diagnose.payload"])

      unless unquote(late_reply?) do
        assert_receive {:agent_request, "deadline", _, cancel}, 2_000
        assert cancel["method"] == "cancel_execution"
        assert cancel["params"]["request_id"] == request["id"]
        acknowledged(id)
        send(handler, {:reply, receipt(request)})
      end

      Fixture.restart_coordinator()
      assert Coordinator.recover_now()["recovered"] == 0
      refute_receive {:agent_request, _, _, _}, 50
    end
  end

  for field <- ~w(generation attempt owner_session_id) do
    test "stale deadline #{field} cannot capture or cancel current work" do
      f = Fixture.claimed_job()
      task = raw_request(f.id)
      assert_receive {:agent_request, "deadline", handler, _}, 2_000
      value = if unquote(field) == "owner_session_id", do: "stale-session", else: 99
      stale = Map.put(f.claim, unquote(field), value)

      assert {:error, :stale_workflow_execution_claim} =
               Coordinator.expire(f.id, stale, "old deadline")

      Fixture.unchanged(f)
      refute_receive {:agent_request, _, _, _}, 20
      release_raw(task, handler)
    end
  end

  for status <- [:completed, :failed, :cancelled] do
    test "deadline cannot rewrite an already #{status} job" do
      f = Fixture.claimed_job()

      assert {:ok, job} =
               Store.apply_progress(%{job_id: f.id, stage: unquote(status), progress: 1.0})

      assert {:error, {:workflow_job_terminal, unquote(status)}} =
               Coordinator.expire(f.id, f.claim, "late deadline")

      Fixture.unchanged(%{f | job: job})
    end
  end

  test "an unavailable gate prevents expiry without converting a failed observation to no work" do
    f = Fixture.claimed_job()

    Outage.without(AgentExecutionGate, fn ->
      assert {:error, :execution_gate_unavailable} = Coordinator.expire(f.id, f.claim, "deadline")
      Fixture.unchanged(f)
    end)
  end

  test "revoked Orchestra authority cannot persist a runner deadline or cancel an Agent" do
    f = Fixture.claimed_job()
    task = raw_request(f.id)
    assert_receive {:agent_request, "deadline", handler, _}, 2_000
    name = Coordinator.snapshot()["lease"]["lease_name"]
    assert {:ok, lease} = LeaseStore.current(name)
    assert :ok = LeaseStore.release(lease)
    assert {:ok, takeover} = LeaseStore.acquire(name, "another-orchestra", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)
    assert {:error, :orchestra_lease_lost} = Coordinator.expire(f.id, f.claim, "deadline")
    Fixture.unchanged(f)
    refute_receive {:agent_request, _, _, _}, 20
    release_raw(task, handler)
  end

  test "stale runner only stops its own task after a deadline loses its claim" do
    {id, handler, request} = submit()
    assert {:ok, runner} = WorkflowJobRunner.running(id)
    ref = Process.monitor(runner)
    {:ok, runtime} = AnalysisResultStore.get(id)
    changed = update_in(runtime, [@key, "generation"], &(&1 + 1))
    assert :ok = AnalysisResultStore.compare_and_swap(id, runtime, changed)
    assert_receive {:DOWN, ^ref, :process, ^runner, :normal}, 2_000
    refute_receive {:agent_request, _, _, _}, 50
    after_loss = terminal(id)
    assert after_loss[@key]["state"] == "recovery_blocked"
    assert after_loss[@key]["generation"] == 2

    assert Map.drop(after_loss[@key], ~w(state history updated_at)) ==
             Map.drop(changed[@key], ~w(state history updated_at))

    assert List.last(after_loss[@key]["history"])["reason"] ==
             "workflow_dispatch_outcome_unconfirmed"

    refute Map.has_key?(after_loss[@key], @intent)
    assert {:ok, %{message: message}} = Store.get(id)
    refute message =~ "workflow execution timed out"
    send(handler, {:reply, receipt(request)})
  end

  test "unavailable coordinator returns a bounded expiry error without changing snapshots" do
    f = Fixture.claimed_job()

    Outage.without(Coordinator, fn ->
      assert {:error, :workflow_coordinator_unavailable} =
               Coordinator.expire(f.id, f.claim, "deadline")

      Fixture.unchanged(f)
      Store.reset()
      AnalysisResultStore.reset()
    end)
  end

  defp submit do
    assert {:ok, admitted} =
             Analysis.submit_workflow_graph(%{
               "graph" => graph(),
               "input_artifacts" => %{"input" => %{}}
             })

    id = admitted["job"]["job_id"]
    assert_receive {:agent_request, "deadline", handler, request}, 2_000
    assert request["method"] == "run_operator_task_ir"
    {id, handler, request}
  end

  defp raw_request(id),
    do:
      Task.async(fn ->
        AgentClient.request("solve_bar_1d", %{}, fn _ -> :ok end, job_id: id)
      end)

  defp release_raw(task, handler) do
    send(handler, {:reply, %{"ok" => true, "result" => %{}}})
    assert {:ok, %{}} = Task.await(task, 2_000)
  end

  defp receipt(request) do
    {:ok, summary} = OperatorTaskExecutionSummary.build(request["params"]["task_ir"])

    %{
      "ok" => true,
      "result" =>
        Map.merge(Map.take(summary, ~w(task_id task_digest operator_id program_id)), %{
          "operator_task_ir_status" => "executed",
          "execution_readiness" => OperatorTaskReadiness.local_executed(),
          "result" => %{"late" => true}
        })
    }
  end

  defp terminal(id) do
    Outage.eventually(fn -> match?({:ok, %{status: :failed}}, Store.get(id)) end, 500)
    {:ok, runtime} = AnalysisResultStore.get(id)
    runtime
  end

  defp acknowledged(id) do
    Outage.eventually(fn ->
      {:ok, runtime} = AnalysisResultStore.get(id)
      match?([%{"status" => "requested"}], get_in(runtime, [@key, @intent, "entries"]))
    end)

    Outage.eventually(fn -> Registry.lookup(@registry, @sweep) == [] end)
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

  defp graph do
    port = %{"id" => "payload", "artifact_type" => "payload/json"}

    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "deadline-native",
      "nodes" => [
        %{"id" => "input", "kind" => "input", "outputs" => [port]},
        %{
          "id" => "diagnose",
          "kind" => "extract",
          "operator_id" => "extract.thermal_result_diagnostics",
          "inputs" => [port],
          "outputs" => [port]
        }
      ],
      "edges" => [
        %{
          "id" => "input-diagnose",
          "artifact_type" => "payload/json",
          "from" => %{"node" => "input", "port" => "payload"},
          "to" => %{"node" => "diagnose", "port" => "payload"}
        }
      ]
    }
  end
end
