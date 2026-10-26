defmodule KyuubikiWeb.Orchestra.WorkflowCancellationLeaseTest do
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
  @endpoint %{id: "lease-agent", host: "127.0.0.1", port: 1, agent_session_id: "session"}

  setup do
    Outage.setup()
    Outage.eventually(fn -> Registry.lookup(@registry, {Delivery, :sweep}) == [] end)
    # Keep startup recovery from racing these explicit delivery reservations.
    assert {:ok, _} = Registry.register(@registry, {Delivery, :sweep}, nil)
    :ok
  end

  for transition <- [:release, :takeover, :outage], operation <- [:bind, :authorize, :finish] do
    test "#{transition} fences old #{operation} in the same coordinator process" do
      f = saved()
      token = reserve(f)
      assert :ok = Store.access(f.id, {:bind, token, target(f)})
      before = runtime(f)
      reacquire(unquote(transition), f.id)

      assert {:error, :stale_cancellation_delivery} =
               Store.access(f.id, action(unquote(operation), token, f))

      assert runtime(f) == before
    end
  end

  test "ordinary lease renewal preserves the active delivery authority" do
    f = saved()
    token = reserve(f)
    before = :sys.get_state(Coordinator)
    Process.sleep(10)
    renew(before)
    after_renewal = :sys.get_state(Coordinator)
    assert after_renewal.session_id == before.session_id
    assert after_renewal.lease.fencing_token == before.lease.fencing_token
    assert after_renewal.lease.expires_at_ms > before.lease.expires_at_ms
    assert :ok = Store.access(f.id, {:bind, token, target(f)})
    assert {:ok, actual} = Store.access(f.id, {:authorize, token})
    assert actual == target(f)
    assert :ok = Store.access(f.id, {:finish, token, "requested"})
    assert entry(f)["status"] == "requested"
  end

  test "new reservation after reacquisition preserves exact target and retry budget" do
    f = saved()
    old = reserve(f)
    assert :ok = Store.access(f.id, {:bind, old, target(f)})
    reacquire(:takeover, f.id)
    fresh = reserve(f)
    assert fresh.delivery_id != old.delivery_id
    assert fresh.entry["attempts"] == 2
    assert fresh.entry["execution_target"] == target(f)

    assert {:error, :cancellation_target_changed} =
             Store.access(f.id, {:bind, fresh, Map.put(target(f), "generation", 8)})

    assert {:ok, actual} = Store.access(f.id, {:authorize, fresh})
    assert actual == target(f)
    assert :ok = Store.access(f.id, {:finish, fresh, "requested"})
    assert entry(f)["status"] == "requested"
    refute Intent.summary(runtime(f)[@key])["execution_terminal_confirmed"]
  end

  for change <- [:missing, nil, :foreign], operation <- [:bind, :authorize, :finish] do
    test "#{inspect(change)} acquisition identity rejects #{operation} without mutating the intent" do
      f = saved()
      token = reserve(f)
      assert :ok = Store.access(f.id, {:bind, token, target(f)})
      before = runtime(f)

      changed =
        case unquote(change) do
          :missing -> Map.delete(token, :lease_session)
          nil -> Map.put(token, :lease_session, nil)
          :foreign -> Map.put(token, :lease_session, "different-acquisition")
        end

      assert {:error, :stale_cancellation_delivery} =
               Store.access(f.id, action(unquote(operation), changed, f))

      assert runtime(f) == before
    end
  end

  test "acquisition identity is not persisted or included in the public summary" do
    f = saved()
    token = reserve(f)
    assert is_reference(token.lease_session)
    refute Jason.encode!(runtime(f)) =~ "lease_session"
    refute Jason.encode!(Coordinator.snapshot()) =~ "lease_session"
    refute Jason.encode!(Intent.summary(runtime(f)[@key])) =~ "lease_session"
  end

  test "returning storage cannot revive a token even when the database fence is reused" do
    f = saved()

    # Memory lease storage deliberately restarts empty; normalize its first fence before reserving.
    reacquire(:outage, f.id)
    token = reserve(f)
    assert :ok = Store.access(f.id, {:bind, token, target(f)})
    before = :sys.get_state(Coordinator)
    retained = runtime(f)
    after_acquisition = reacquire(:outage, f.id)
    assert after_acquisition.lease.fencing_token == before.lease.fencing_token
    assert after_acquisition.lease_session != before.lease_session
    assert {:error, :stale_cancellation_delivery} = Store.access(f.id, {:authorize, token})
    assert runtime(f) == retained
  end

  test "coordinator restart still rejects an old delivery without altering its record" do
    f = saved()
    token = reserve(f)
    assert :ok = Store.access(f.id, {:bind, token, target(f)})
    retained = runtime(f)
    Fixture.restart_coordinator()
    assert {:error, :stale_cancellation_delivery} = Store.access(f.id, {:authorize, token})
    assert runtime(f) == retained
  end

  test "lease takeover while TCP observation waits cannot bind or cancel the old delivery" do
    f = saved(agent())
    sweep = kick(f)
    {handler, _} = receive_request("describe_agent")
    reacquire(:takeover, f.id)
    retained = runtime(f)
    reply_description(handler, f)
    assert_done(sweep)
    assert runtime(f) == retained
    assert entry(f)["status"] == "dispatching"
    assert is_nil(entry(f)["execution_target"])
  end

  test "lease takeover during final directory lookup rejects earlier authorization" do
    f = saved(agent())
    probe = {_pid, token} = Probe.install(AgentPool, &(&1 == :inspection_endpoints), 2)
    sweep = kick(f)
    {handler, _} = receive_request("describe_agent")
    reply_description(handler, f)
    assert_receive {:cancellation_paused, ^token, _}, 2_000
    reacquire(:takeover, f.id)
    retained = runtime(f)
    Probe.resume(probe)
    assert_done(sweep)
    assert runtime(f) == retained
    assert entry(f)["execution_target"] == target(f)
    retry_exact(f)
  end

  for outcome <- [:requested, :not_observed, :lost_ack] do
    test "delayed TCP #{outcome} cannot settle a delivery under a newly acquired lease" do
      f = saved(agent())
      sweep = kick(f)
      {handler, _} = receive_request("describe_agent")
      reply_description(handler, f)
      {handler, request} = receive_request("cancel_execution")
      assert request["params"] == target(f)
      reacquire(:takeover, f.id)
      retained = runtime(f)
      acknowledge(handler, request, unquote(outcome))
      assert_done(sweep)
      assert runtime(f) == retained
      assert entry(f)["status"] == "dispatching"
      refute Intent.summary(runtime(f)[@key])["execution_terminal_confirmed"]
      retry_exact(f)
    end
  end

  test "normal renewal during TCP observation still delivers only the exact target" do
    f = saved(agent())
    sweep = kick(f)
    {handler, _} = receive_request("describe_agent")
    before = :sys.get_state(Coordinator)
    renew(before)
    assert :sys.get_state(Coordinator).lease_session == before.lease_session
    reply_description(handler, f)
    {handler, request} = receive_request("cancel_execution")
    assert request["params"] == target(f)
    acknowledge(handler, request)
    assert_done(sweep)
    assert entry(f)["status"] == "requested"
    assert entry(f)["attempts"] == 1
  end

  defp agent do
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])
    start_supervised!({CancellableSolverAgent, {self(), "lease-agent"}})
    assert_receive {:cancellable_agent_ready, "lease-agent", port}
    endpoint = %{@endpoint | port: port}
    Application.put_env(:kyuubiki_web, AgentPool, discovery: :static, endpoints: [endpoint])
    assert :ok = AgentPool.reload()

    on_exit(fn ->
      for {pid, _} <- Registry.lookup(@registry, {Delivery, :sweep}),
          do: Process.exit(pid, :kill)

      Application.put_env(:kyuubiki_web, AgentPool, previous)
      :ok = AgentPool.reload()
    end)

    endpoint
  end

  defp kick(f) do
    Registry.unregister(@registry, {Delivery, :sweep})
    assert {:ok, pid} = Delivery.kick([f.id])
    {pid, Process.monitor(pid)}
  end

  defp assert_done({pid, ref}) do
    assert_receive {:DOWN, ^ref, :process, ^pid, :normal}, 2_000
    refute_receive {:agent_request, _, _, _}, 30
  end

  defp retry_exact(f) do
    sweep = kick(f)
    {handler, request} = receive_request("cancel_execution")
    assert request["params"] == target(f)
    acknowledge(handler, request)
    assert_done(sweep)
    assert entry(f)["status"] == "requested"
    assert entry(f)["attempts"] == 2
    refute Intent.summary(runtime(f)[@key])["execution_terminal_confirmed"]
  end

  defp receive_request(method) do
    assert_receive {:agent_request, "lease-agent", handler, %{"method" => ^method} = request},
                   2_000

    {handler, request}
  end

  defp reply_description(handler, f) do
    active =
      Enum.map(["request", "bystander"], fn id ->
        %{"request_id" => id, "job_id" => f.id, "generation" => 7, "cancel_requested" => false}
      end)

    send(
      handler,
      {:reply,
       %{
         "ok" => true,
         "result" => %{
           "lifecycle" => %{"process_instance_id" => "original-process"},
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

  defp saved(endpoint \\ @endpoint) do
    f = Fixture.terminal_job(:failed)

    snapshot = %{
      requests: [%{endpoint: endpoint, request_id: "request", job_id: f.id}],
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

    assert {:ok, recovery} = Intent.attach(f.runtime[@key], f.job, snapshot)
    updated = Map.put(f.runtime, @key, recovery)
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, updated)
    %{f | runtime: updated}
  end

  defp reserve(f) do
    assert {:ok, bucket} = Store.access(f.id, :load)
    identity = Intent.identity(hd(bucket["entries"]))
    assert {:ok, token} = Store.access(f.id, {:reserve, bucket["intent_id"], identity})
    token
  end

  defp target(f),
    do: %{
      "job_id" => f.id,
      "request_id" => "request",
      "process_instance_id" => "original-process",
      "generation" => 7
    }

  defp action(:bind, token, f), do: {:bind, token, target(f)}
  defp action(:authorize, token, _), do: {:authorize, token}
  defp action(:finish, token, _), do: {:finish, token, "requested"}

  defp runtime(f) do
    assert {:ok, runtime} = AnalysisResultStore.get(f.id)
    runtime
  end

  defp entry(f), do: hd(runtime(f)[@key][Intent.key()]["entries"])

  defp reacquire(transition, id) do
    pid = Process.whereis(Coordinator)
    before = :sys.get_state(Coordinator)

    case transition do
      :outage ->
        child = if Storage.memory?(), do: LeaseMemoryBackend, else: Outage.store_child()

        Outage.without(child, fn ->
          assert {:error, :orchestra_lease_store_unavailable} = Store.access(id, :load)
          assert :sys.get_state(Coordinator).lease_status == :standby
        end)

      change ->
        assert :ok = LeaseStore.release(before.lease)

        holder =
          if change == :takeover do
            assert {:ok, holder} = LeaseStore.acquire(before.lease_name, "other-owner", 120_000)
            holder
          end

        renew(before)
        assert :sys.get_state(Coordinator).lease_status == :standby
        if holder, do: assert(:ok = LeaseStore.release(holder))
    end

    send(Coordinator, :acquire_lease)
    Outage.eventually(fn -> :sys.get_state(Coordinator).lease_status == :owner end)
    # Drain the recovery scan while the test or TCP sweep owns the cancellation registry key.
    Outage.eventually(fn -> :sys.get_state(Coordinator).recovery_runs > before.recovery_runs end)
    after_acquisition = :sys.get_state(Coordinator)
    assert Process.whereis(Coordinator) == pid
    assert after_acquisition.session_id == before.session_id

    if transition != :outage do
      assert after_acquisition.lease.fencing_token > before.lease.fencing_token
    end

    after_acquisition
  end

  defp renew(state) do
    send(Coordinator, {:renew_lease, state.lease.fencing_token, state.lease.expires_at_ms})
    Coordinator.snapshot()
  end
end
