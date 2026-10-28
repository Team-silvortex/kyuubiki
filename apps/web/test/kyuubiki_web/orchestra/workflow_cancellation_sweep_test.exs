defmodule KyuubikiWeb.Orchestra.WorkflowCancellationSweepTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Storage}
  alias KyuubikiWeb.Orchestra.{LeaseMemoryBackend, LeaseStore}
  alias KyuubikiWeb.Orchestra.WorkflowCancellationDelivery, as: Delivery
  alias KyuubikiWeb.Orchestra.WorkflowCancellationIntent, as: Intent
  alias KyuubikiWeb.Orchestra.WorkflowCancellationStore, as: Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.TestSupport.CancellableSolverAgent
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCancellationProbe, as: Probe
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @key "_workflow_recovery"
  @registry KyuubikiWeb.Orchestra.WorkflowRunnerRegistry

  setup do
    Outage.setup()
    Outage.eventually(fn -> Registry.lookup(@registry, {Delivery, :sweep}) == [] end)
    assert {:ok, _} = Registry.register(@registry, {Delivery, :sweep}, nil)
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])
    start_supervised!({CancellableSolverAgent, {self(), "sweep-agent"}})
    assert_receive {:cancellable_agent_ready, "sweep-agent", port}
    endpoint = %{id: "sweep-agent", host: "127.0.0.1", port: port, agent_session_id: "session"}
    Application.put_env(:kyuubiki_web, AgentPool, discovery: :static, endpoints: [endpoint])
    assert :ok = AgentPool.reload()

    on_exit(fn ->
      for {pid, _} <- Registry.lookup(@registry, {Delivery, :sweep}),
          do: Process.exit(pid, :kill)

      Application.put_env(:kyuubiki_web, AgentPool, previous)
      :ok = AgentPool.reload()
    end)

    %{endpoint: endpoint}
  end

  for scope <- [:entries, :jobs],
      transition <- [:takeover, :restart, :outage],
      phase <- [:discovery, :observation, :directory] do
    test "#{transition} during #{phase} stops the entire #{scope} sweep", ctx do
      fixtures = batch(ctx.endpoint, unquote(scope))
      [first | _] = fixtures

      probe =
        case unquote(phase) do
          :discovery -> Probe.install(AgentPool, &(&1 == :inspection_endpoints))
          :directory -> Probe.install(AgentPool, &(&1 == :inspection_endpoints), 2)
          _ -> nil
        end

      sweep = kick(fixtures)

      handler =
        if unquote(phase) != :discovery do
          {handler, _} = receive_request("describe_agent")
          if probe, do: reply_description(handler, fixtures)
          handler
        end

      if probe do
        {_, token} = probe
        assert_receive {:cancellation_paused, ^token, _}, 2_000
      end

      transition(unquote(transition))
      retained = snapshots(fixtures)

      if probe, do: Probe.resume(probe), else: reply_description(handler, fixtures)
      assert_stopped(sweep)
      assert snapshots(fixtures) == retained
      assert hd(entries(first))["attempts"] == 1
      assert_untouched_tail(fixtures)
    end
  end

  for scope <- [:entries, :jobs],
      transition <- [:takeover, :restart, :outage],
      outcome <- [:requested, :not_observed, :lost_ack] do
    test "#{transition} before #{outcome} reply cannot reserve later #{scope}", ctx do
      fixtures = batch(ctx.endpoint, unquote(scope))
      [first | _] = fixtures
      sweep = kick(fixtures)
      {handler, _} = receive_request("describe_agent")
      reply_description(handler, fixtures)
      {handler, request} = receive_request("cancel_execution")
      assert request["params"] == target(first, "one")
      transition(unquote(transition))
      retained = snapshots(fixtures)
      acknowledge(handler, request, unquote(outcome))
      assert_stopped(sweep)
      assert snapshots(fixtures) == retained
      assert hd(entries(first))["status"] == "dispatching"
      assert_untouched_tail(fixtures)
    end
  end

  for scope <- [:entries, :jobs] do
    test "ordinary renewal keeps the healthy #{scope} batch usable", ctx do
      fixtures = batch(ctx.endpoint, unquote(scope))
      sweep = kick(fixtures)

      for f <- fixtures do
        {handler, _} = receive_request("describe_agent")
        transition(:renew)
        reply_description(handler, fixtures)

        for id <- f.ids do
          {handler, request} = receive_request("cancel_execution")
          assert request["params"] == target(f, id)
          acknowledge(handler, request)
        end
      end

      assert_stopped(sweep)

      for f <- fixtures, entry <- entries(f) do
        assert entry["attempts"] == 1
        assert entry["status"] == "requested"
      end
    end
  end

  for change <- [:takeover, :restart, :outage] do
    test "delayed startup under #{change} cannot consume any attempt", ctx do
      fixtures = batch(ctx.endpoint, :entries)
      authority = Store.authority(:sys.get_state(Coordinator))
      transition(unquote(change))
      retained = snapshots(fixtures)
      Registry.unregister(@registry, {Delivery, :sweep})
      assert {:ok, pid} = Delivery.kick(Enum.map(fixtures, & &1.id), authority)
      assert_stopped({pid, Process.monitor(pid)})
      assert snapshots(fixtures) == retained
      assert Enum.all?(Enum.flat_map(fixtures, &entries/1), &(&1["attempts"] == 0))
    end

    test "scoped reserve after #{change} rejects the old sweep before persistence", ctx do
      [f | _] = fixtures = batch(ctx.endpoint, :entries)
      assert {:ok, authority} = Store.access(nil, :sweep_authority)
      assert {:ok, bucket} = Store.access(f.id, :load, authority)
      transition(unquote(change))
      retained = snapshots(fixtures)
      identity = Intent.identity(hd(bucket["entries"]))

      assert {:error, :stale_cancellation_sweep} = Store.access(f.id, :load, authority)

      assert {:error, :stale_cancellation_sweep} =
               Store.access(f.id, {:reserve, bucket["intent_id"], identity}, authority)

      assert snapshots(fixtures) == retained
    end
  end

  test "malformed sweep authority cannot reserve or contact an Agent", ctx do
    fixtures = batch(ctx.endpoint, :entries)
    retained = snapshots(fixtures)
    state = :sys.get_state(Coordinator)
    Registry.unregister(@registry, {Delivery, :sweep})

    for authority <- [
          nil,
          %{},
          {state.session_id, make_ref()},
          {"other-session", state.lease_session}
        ] do
      assert {:ok, pid} = Delivery.kick(Enum.map(fixtures, & &1.id), authority)
      assert_stopped({pid, Process.monitor(pid)})
      assert snapshots(fixtures) == retained
    end
  end

  for scope <- [:entries, :jobs] do
    test "fresh recovery of #{scope} retains bound targets but discards the old observation cache",
         ctx do
      [first | _] = fixtures = batch(ctx.endpoint, unquote(scope))
      sweep = kick(fixtures)
      {handler, _} = receive_request("describe_agent")
      reply_description(handler, fixtures)
      {handler, request} = receive_request("cancel_execution")
      assert request["params"] == target(first, "one")
      transition(:takeover)
      acknowledge(handler, request)
      assert_stopped(sweep)

      fresh = kick(fixtures)

      for f <- fixtures do
        ids = if f.id == first.id and unquote(scope) == :entries, do: ["two", "one"], else: f.ids

        Enum.reduce(ids, false, fn id, described ->
          bound = f.id == first.id and id == "one"

          if not bound and not described do
            {handler, _} = receive_request("describe_agent")
            reply_description(handler, fixtures, 9)
          end

          {handler, request} = receive_request("cancel_execution")
          assert request["params"] == target(f, id, if(bound, do: 7, else: 9))
          acknowledge(handler, request)
          described or not bound
        end)
      end

      assert_stopped(fresh)
      [first_entry | tail] = Enum.flat_map(fixtures, &entries/1)
      assert first_entry["attempts"] == 2
      assert Enum.all?(tail, &(&1["attempts"] == 1))
      assert Enum.all?([first_entry | tail], &(&1["status"] == "requested"))
    end
  end

  defp batch(endpoint, :entries),
    do: [saved(endpoint, ["one", "two"]), saved(endpoint, ["three"])]

  defp batch(endpoint, :jobs), do: [saved(endpoint, ["one"]), saved(endpoint, ["two"])]

  defp saved(endpoint, ids) do
    f = Fixture.terminal_job(:failed)

    snapshot = %{
      requests: Enum.map(ids, &%{endpoint: endpoint, request_id: &1, job_id: f.id}),
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

    assert {:ok, recovery} = Intent.attach(f.runtime[@key], f.job, snapshot)
    updated = Map.put(f.runtime, @key, recovery)
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, updated)
    Map.merge(f, %{ids: ids, runtime: updated})
  end

  defp transition(:restart) do
    old = Process.whereis(Coordinator)
    Fixture.restart_coordinator()
    assert Process.whereis(Coordinator) != old
    Outage.eventually(fn -> :sys.get_state(Coordinator).recovery_runs > 0 end)
    Probe.discard_startup_handoff()
  end

  defp transition(:outage) do
    before = :sys.get_state(Coordinator)
    child = if Storage.memory?(), do: LeaseMemoryBackend, else: Outage.store_child()

    Outage.without(child, fn ->
      assert {:error, :orchestra_lease_store_unavailable} = Store.access(nil, :sweep_authority)
      assert :sys.get_state(Coordinator).lease_status == :standby
    end)

    send(Coordinator, :acquire_lease)
    Outage.eventually(fn -> :sys.get_state(Coordinator).recovery_runs > before.recovery_runs end)
    Probe.discard_startup_handoff()
    assert :sys.get_state(Coordinator).lease_session != before.lease_session
  end

  defp transition(change) do
    before = :sys.get_state(Coordinator)

    holder =
      if change == :takeover do
        assert :ok = LeaseStore.release(before.lease)
        assert {:ok, holder} = LeaseStore.acquire(before.lease_name, "sweep-takeover", 120_000)
        on_exit(fn -> LeaseStore.release(holder) end)
        holder
      end

    send(Coordinator, {:renew_lease, before.lease.fencing_token, before.lease.expires_at_ms})
    Coordinator.snapshot()

    if holder do
      assert :sys.get_state(Coordinator).lease_status == :standby
      assert :ok = LeaseStore.release(holder)
      send(Coordinator, :acquire_lease)

      Outage.eventually(fn ->
        :sys.get_state(Coordinator).recovery_runs > before.recovery_runs
      end)

      Probe.discard_startup_handoff()

      assert :sys.get_state(Coordinator).lease_session != before.lease_session
    else
      assert :sys.get_state(Coordinator).lease_session == before.lease_session
    end
  end

  defp kick(fixtures) do
    Registry.unregister(@registry, {Delivery, :sweep})
    assert {:ok, pid} = Delivery.kick(Enum.map(fixtures, & &1.id))
    {pid, Process.monitor(pid)}
  end

  defp assert_stopped({pid, ref}) do
    refute_receive {:agent_request, _, _, _}, 100
    assert_receive {:DOWN, ^ref, :process, ^pid, :normal}, 2_000
    refute_receive {:agent_request, _, _, _}, 20
  end

  defp snapshots(fixtures) do
    Enum.map(fixtures, fn f ->
      assert {:ok, runtime} = AnalysisResultStore.get(f.id)
      runtime
    end)
  end

  defp entries(f), do: hd(snapshots([f]))[@key][Intent.key()]["entries"]

  defp assert_untouched_tail(fixtures) do
    [_first | tail] = Enum.flat_map(fixtures, &entries/1)

    for entry <- tail do
      assert entry["attempts"] == 0
      assert entry["status"] == "pending"
      assert is_nil(entry["execution_target"])
    end
  end

  defp target(f, id, generation \\ 7),
    do: %{
      "job_id" => f.id,
      "request_id" => id,
      "process_instance_id" => "sweep-process",
      "generation" => generation
    }

  defp receive_request(method) do
    assert_receive {:agent_request, "sweep-agent", handler, %{"method" => ^method} = request},
                   2_000

    {handler, request}
  end

  defp reply_description(handler, fixtures, generation \\ 7) do
    active =
      for f <- fixtures, id <- f.ids ++ ["bystander"] do
        %{
          "request_id" => id,
          "job_id" => f.id,
          "generation" => generation,
          "cancel_requested" => false
        }
      end

    send(
      handler,
      {:reply,
       %{
         "ok" => true,
         "result" => %{
           "lifecycle" => %{"process_instance_id" => "sweep-process"},
           "solver_control" => %{
             "schema_version" => "kyuubiki.agent-solver-control/v1",
             "available" => true,
             "active" => active
           }
         }
       }}
    )
  end

  defp acknowledge(handler, request, outcome \\ :requested)
  defp acknowledge(handler, _, :lost_ack), do: send(handler, :disconnect)

  defp acknowledge(handler, request, outcome) do
    ack = %{
      "schema_version" => "kyuubiki.agent-execution-cancellation/v1",
      "execution_target" => request["params"],
      "status" => if(outcome == :requested, do: "requested", else: "target_not_observed"),
      "cancel_registered" => outcome == :requested,
      "execution_terminal_confirmed" => false,
      "pending_cancellation_created" => false,
      "operator_package_cleanup_performed" => false,
      "automatic_replay_authorized" => false
    }

    send(handler, {:reply, %{"ok" => true, "result" => ack}})
  end
end
