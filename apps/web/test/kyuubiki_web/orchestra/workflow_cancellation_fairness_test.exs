defmodule KyuubikiWeb.Orchestra.WorkflowCancellationFairnessTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.Orchestra.WorkflowCancellationDelivery, as: Delivery
  alias KyuubikiWeb.Orchestra.WorkflowCancellationHandoff, as: Handoff
  alias KyuubikiWeb.Orchestra.WorkflowCancellationIntent, as: Intent
  alias KyuubikiWeb.Orchestra.WorkflowCancellationStore, as: Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.TestSupport.CancellableSolverAgent
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @key "_workflow_recovery"
  @registry KyuubikiWeb.Orchestra.WorkflowRunnerRegistry

  setup do
    Outage.setup()
    Outage.eventually(fn -> :sys.get_state(Coordinator).recovery_runs > 0 end)
    Outage.eventually(fn -> Registry.lookup(@registry, {Delivery, :sweep}) == [] end)
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])
    start_supervised!({CancellableSolverAgent, {self(), "fairness-agent"}})
    assert_receive {:cancellable_agent_ready, "fairness-agent", port}, 2_000
    endpoint = %{id: "fairness-agent", host: "127.0.0.1", port: port, agent_session_id: "session"}
    Application.put_env(:kyuubiki_web, AgentPool, discovery: :static, endpoints: [endpoint])
    assert :ok = AgentPool.reload()

    on_exit(fn ->
      :sys.replace_state(Coordinator, &Handoff.clear/1)
      for {pid, _} <- Registry.lookup(@registry, {Delivery, :sweep}), do: Process.exit(pid, :kill)
      Application.put_env(:kyuubiki_web, AgentPool, previous)
      :ok = AgentPool.reload()
    end)

    %{endpoint: endpoint}
  end

  for status <- ["pending", "dispatching"], bound <- [false, true] do
    test "fresh entries precede #{status} retries with bound target #{bound}", ctx do
      f = saved(ctx.endpoint, ["old", "new"])
      seed(f, "old", 2, unquote(status), unquote(bound))
      {:ok, pid} = Delivery.kick([f.id])
      reply_observation(f)
      cancel(f, "new", 9)
      cancel(f, "old", if(unquote(bound), do: 7, else: 9))
      finished(pid)
      assert Enum.map(entries(f), & &1["request_id"]) == f.ids
      assert entry(f, "new")["attempts"] == 1
      assert entry(f, "old")["attempts"] == 3
      assert Enum.all?(entries(f), &(&1["status"] == "requested"))
    end
  end

  for attempts <- 0..2 do
    test "equal attempt count #{attempts} retains captured order rather than lexical order",
         ctx do
      f = saved(ctx.endpoint, ["z", "a", "m"])
      for id <- f.ids, do: seed(f, id, unquote(attempts), "pending")
      {:ok, pid} = Delivery.kick([f.id])
      if unquote(attempts) == 0, do: reply_observation(f)
      for id <- f.ids, do: cancel(f, id, if(unquote(attempts) == 0, do: 9, else: 7))
      finished(pid)
      assert Enum.all?(entries(f), &(&1["attempts"] == unquote(attempts) + 1))
    end
  end

  test "lower nonzero attempts precede older retries and ties remain stable", ctx do
    f = saved(ctx.endpoint, ["many", "few", "new", "also-few"])
    seed(f, "many", 2, "pending")
    seed(f, "few", 1, "dispatching")
    seed(f, "also-few", 1, "pending")
    {:ok, pid} = Delivery.kick([f.id])
    reply_observation(f)
    cancel(f, "new", 9)
    for id <- ["few", "also-few", "many"], do: cancel(f, id, 7)
    finished(pid)
    assert Enum.map(entries(f), & &1["attempts"]) == [3, 2, 1, 2]
    assert Enum.map(entries(f), & &1["request_id"]) == f.ids
  end

  test "explicit recovery after worker kill visits untouched entries before unknown deliveries",
       ctx do
    f = saved(ctx.endpoint, ["lost", "crashed", "untouched"])
    {:ok, old} = Delivery.kick([f.id])
    reply_observation(f, 7)
    {handler, request} = request("cancel_execution")
    assert request["params"] == target(f, "lost", 7)
    send(handler, :disconnect)
    {handler, request} = request("cancel_execution")
    assert request["params"] == target(f, "crashed", 7)
    ref = Process.monitor(old)
    Process.exit(old, :kill)
    assert_receive {:DOWN, ^ref, :process, ^old, :killed}, 2_000
    send(handler, :disconnect)
    Outage.eventually(fn -> Registry.lookup(@registry, {Delivery, :sweep}) == [] end)
    assert entry(f, "lost")["status"] == "pending"
    assert entry(f, "crashed")["status"] == "dispatching"
    assert entry(f, "untouched")["attempts"] == 0
    refute_receive {:agent_request, _, _, _}, 50

    {:ok, fresh} = Delivery.kick([f.id])
    reply_observation(f)
    cancel(f, "untouched", 9)
    for id <- ["lost", "crashed"], do: cancel(f, id, 7)
    finished(fresh)
    assert Enum.map(entries(f), & &1["attempts"]) == [2, 2, 1]
    refute Intent.summary(runtime(f)[@key])["execution_terminal_confirmed"]
  end

  for transition <- [:restart, :reacquire] do
    test "#{transition} recovery derives fair order from durable attempts without a cursor",
         ctx do
      f = saved(ctx.endpoint, ["old", "new"])
      seed(f, "old", 1, "dispatching")
      holder = hold_slot()
      before = :sys.get_state(Coordinator)

      if unquote(transition) == :restart do
        Fixture.restart_coordinator()
      else
        assert :ok = LeaseStore.release(before.lease)
        send(Coordinator, {:renew_lease, before.lease.fencing_token, before.lease.expires_at_ms})
        assert Coordinator.snapshot()["lease"]["status"] == "standby"
        send(Coordinator, :acquire_lease)
      end

      Outage.eventually(fn ->
        Coordinator.snapshot()["cancellation_handoff"]["pending_jobs"] == 1
      end)

      refute :sys.get_state(Coordinator).lease_session == before.lease_session
      send(holder, :release)
      reply_observation(f)
      cancel(f, "new", 9)
      cancel(f, "old", 7)
      Outage.eventually(fn -> Enum.all?(entries(f), &(&1["status"] == "requested")) end)
      assert Enum.map(entries(f), & &1["attempts"]) == [2, 1]
      refute Map.has_key?(runtime(f)[@key][Intent.key()], "cursor")
      refute_receive {:agent_request, _, _, _}, 50
    end
  end

  test "exhausted crash reservation cannot block untouched work or authorize another send", ctx do
    f = saved(ctx.endpoint, ["crashed", "new"])
    seed(f, "crashed", 3, "dispatching")
    retained_target = entry(f, "crashed")["execution_target"]
    {:ok, pid} = Delivery.kick([f.id])
    reply_observation(f)
    cancel(f, "new", 9)
    finished(pid)
    assert entry(f, "crashed")["status"] == "exhausted"
    assert entry(f, "crashed")["attempts"] == 3
    assert entry(f, "crashed")["execution_target"] == retained_target
    before = runtime(f)
    {:ok, pid} = Delivery.kick([f.id])
    finished(pid)
    assert runtime(f) == before
  end

  test "settled entries retain their receipts while pending entries are processed", ctx do
    f = saved(ctx.endpoint, ["done", "gone", "spent", "new"])
    seed(f, "done", 1, "requested")
    seed(f, "gone", 1, "target_not_observed")
    seed(f, "spent", 3, "pending")
    retained = Enum.take(entries(f), 3)
    {:ok, pid} = Delivery.kick([f.id])
    reply_observation(f)
    cancel(f, "new", 9)
    finished(pid)
    assert Enum.take(entries(f), 3) == retained
  end

  test "corrupt attempt metadata is rejected before ordering or delivery", ctx do
    f = saved(ctx.endpoint, ["old", "new"])
    before = runtime(f)
    bucket = before[@key][Intent.key()]
    changed_entries = [Map.put(hd(bucket["entries"]), "attempts", -1) | tl(bucket["entries"])]
    changed = put_in(before, [@key, Intent.key(), "entries"], changed_entries)
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, before, changed)
    {:ok, pid} = Delivery.kick([f.id])
    finished(pid)
    assert runtime(f) == changed
  end

  test "fresh work completes before slow retries exhaust the real sweep budget", ctx do
    f = saved(ctx.endpoint, ["slow-a", "slow-b", "slow-c", "fresh"])
    for id <- Enum.take(f.ids, 3), do: seed(f, id, 1, "pending")
    {:ok, pid} = Delivery.kick([f.id])
    reply_observation(f)
    cancel(f, "fresh", 9)
    Outage.eventually(fn -> entry(f, "fresh")["status"] == "requested" end)

    for id <- Enum.take(f.ids, 3) do
      {_handler, request} = request("cancel_execution", 7_000)
      assert request["params"] == target(f, id, 7)
    end

    finished(pid, 7_000)
    assert entry(f, "fresh")["attempts"] == 1

    for id <- Enum.take(f.ids, 3) do
      assert entry(f, id)["attempts"] == 2
      assert entry(f, id)["reason"] == "cancellation_outcome_unknown"
    end
  end

  defp saved(endpoint, ids) do
    f = Fixture.terminal_job(:failed)

    snapshot = %{
      requests: Enum.map(ids, &%{endpoint: endpoint, request_id: &1, job_id: f.id}),
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

    assert {:ok, recovery} = Intent.attach(f.runtime[@key], f.job, snapshot)

    assert :ok =
             AnalysisResultStore.compare_and_swap(
               f.id,
               f.runtime,
               Map.put(f.runtime, @key, recovery)
             )

    Map.put(f, :ids, ids)
  end

  defp seed(f, id, count, state, bound \\ true) do
    for attempt <- Enum.take(1..3, count) do
      {:ok, bucket} = Store.access(f.id, :load)
      identity = Intent.identity(Enum.find(bucket["entries"], &(&1["request_id"] == id)))
      assert {:ok, token} = Store.access(f.id, {:reserve, bucket["intent_id"], identity})
      if bound, do: assert(:ok = Store.access(f.id, {:bind, token, target(f, id, 7)}))

      unless attempt == count and state == "dispatching" do
        outcome =
          if attempt == count and state in ~w(requested target_not_observed),
            do: state,
            else: "cancellation_outcome_unknown"

        assert :ok = Store.access(f.id, {:finish, token, outcome})
      end
    end
  end

  defp hold_slot do
    owner = self()

    pid =
      spawn(fn ->
        {:ok, _} = Registry.register(@registry, {Delivery, :sweep}, nil)
        send(owner, :slot_held)
        receive do: (:release -> :ok)
      end)

    on_exit(fn -> if Process.alive?(pid), do: Process.exit(pid, :kill) end)
    assert_receive :slot_held, 2_000
    pid
  end

  defp reply_observation(f, generation \\ 9) do
    {handler, _} = request("describe_agent")

    active =
      for id <- f.ids ++ ["bystander"],
          do: %{
            "request_id" => id,
            "job_id" => f.id,
            "generation" => generation,
            "cancel_requested" => false
          }

    send(
      handler,
      {:reply,
       %{
         "ok" => true,
         "result" => %{
           "lifecycle" => %{"process_instance_id" => "fairness-process"},
           "solver_control" => %{
             "schema_version" => "kyuubiki.agent-solver-control/v1",
             "available" => true,
             "active" => active
           }
         }
       }}
    )
  end

  defp cancel(f, id, generation) do
    {handler, request} = request("cancel_execution")
    assert request["params"] == target(f, id, generation)

    send(
      handler,
      {:reply,
       %{
         "ok" => true,
         "result" => %{
           "schema_version" => "kyuubiki.agent-execution-cancellation/v1",
           "execution_target" => request["params"],
           "status" => "requested",
           "cancel_registered" => true,
           "execution_terminal_confirmed" => false,
           "pending_cancellation_created" => false,
           "operator_package_cleanup_performed" => false,
           "automatic_replay_authorized" => false
         }
       }}
    )
  end

  defp request(method, timeout \\ 2_000) do
    assert_receive {:agent_request, "fairness-agent", handler, %{"method" => ^method} = request},
                   timeout

    {handler, request}
  end

  defp finished(pid, timeout \\ 2_000) do
    ref = Process.monitor(pid)
    assert_receive {:DOWN, ^ref, :process, ^pid, reason}, timeout
    assert reason in [:normal, :noproc]
    refute_receive {:agent_request, _, _, _}, 50
  end

  defp target(f, id, generation),
    do: %{
      "job_id" => f.id,
      "request_id" => id,
      "generation" => generation,
      "process_instance_id" => "fairness-process"
    }

  defp runtime(f) do
    assert {:ok, runtime} = AnalysisResultStore.get(f.id)
    runtime
  end

  defp entries(f), do: runtime(f)[@key][Intent.key()]["entries"]
  defp entry(f, id), do: Enum.find(entries(f), &(&1["request_id"] == id))
end
