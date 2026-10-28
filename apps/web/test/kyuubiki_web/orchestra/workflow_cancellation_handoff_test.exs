defmodule KyuubikiWeb.Orchestra.WorkflowCancellationHandoffTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.Orchestra.WorkflowCancellationDelivery, as: Delivery
  alias KyuubikiWeb.Orchestra.WorkflowCancellationIntent, as: Intent
  alias KyuubikiWeb.Orchestra.WorkflowCancellationHandoff, as: Handoff
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
    Outage.eventually(fn -> Registry.lookup(@registry, {Delivery, :sweep}) == [] end)
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])
    start_supervised!({CancellableSolverAgent, {self(), "handoff-agent"}})
    assert_receive {:cancellable_agent_ready, "handoff-agent", port}
    endpoint = %{id: "handoff-agent", host: "127.0.0.1", port: port, agent_session_id: "session"}
    Application.put_env(:kyuubiki_web, AgentPool, discovery: :static, endpoints: [endpoint])
    assert :ok = AgentPool.reload()

    on_exit(fn ->
      :sys.replace_state(Coordinator, &Handoff.clear/1)

      for {pid, _} <- Registry.lookup(@registry, {Delivery, :sweep}),
          do: Process.exit(pid, :kill)

      Application.put_env(:kyuubiki_web, AgentPool, previous)
      :ok = AgentPool.reload()
    end)

    %{endpoint: endpoint}
  end

  test "busy registration retains a trigger until the incumbent exits", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    kick_and_wait([f.id])
    refute_receive {:agent_request, _, _, _}, 30
    send(holder, :release)
    complete(f)
  end

  test "another job triggered during a live sweep runs without another manual kick", ctx do
    first = saved(ctx.endpoint)
    second = saved(ctx.endpoint)
    {:ok, _} = Delivery.kick([first.id])
    {handler, _} = request("describe_agent")
    kick_and_wait([second.id])
    reply_description(handler, first)
    {handler, cancel} = request("cancel_execution")
    assert cancel["params"] == target(first)
    acknowledge(handler, cancel)
    complete(second)
    assert entry(first)["status"] == "requested"
  end

  test "restart kickoff takes over after the old observer exits", ctx do
    f = saved(ctx.endpoint)
    {:ok, old} = Delivery.kick([f.id])
    {handler, _} = request("describe_agent")
    Fixture.restart_coordinator()
    Outage.eventually(fn -> :sys.get_state(Coordinator).recovery_runs > 0 end)
    reply_description(handler, f)
    complete(f)
    refute Process.alive?(old)
    assert entry(f)["attempts"] == 2
  end

  test "lease reacquisition kickoff preserves the bound target across a late reply", ctx do
    f = saved(ctx.endpoint)
    {:ok, old} = Delivery.kick([f.id])
    {handler, _} = request("describe_agent")
    reply_description(handler, f)
    {handler, cancel} = request("cancel_execution")
    before = :sys.get_state(Coordinator)
    assert :ok = LeaseStore.release(before.lease)
    send(Coordinator, {:renew_lease, before.lease.fencing_token, before.lease.expires_at_ms})
    assert Coordinator.snapshot()["lease"]["status"] == "standby"
    send(Coordinator, :acquire_lease)
    Outage.eventually(fn -> :sys.get_state(Coordinator).recovery_runs > before.recovery_runs end)
    acknowledge(handler, cancel)
    {handler, retry} = request("cancel_execution")
    assert retry["params"] == target(f)
    acknowledge(handler, retry)
    settled(f)
    refute Process.alive?(old)
    assert entry(f)["attempts"] == 2
    refute_receive {:agent_request, _, _, _}, 30
  end

  test "healthy single trigger still performs one exact cancellation", ctx do
    f = saved(ctx.endpoint)
    {:ok, _} = Delivery.kick([f.id])
    complete(f)
    assert entry(f)["attempts"] == 1
  end

  test "duplicate triggers coalesce without renewing the wait budget", ctx do
    holder = hold_slot()
    first = saved(ctx.endpoint)
    second = saved(ctx.endpoint)
    kick_and_wait([first.id])
    pending_jobs(1)
    deadline = pending().deadline
    for _ <- 1..10, do: kick_and_wait([first.id, second.id, first.id])
    pending_jobs(2)
    assert pending().deadline == deadline
    summary = Coordinator.snapshot()["cancellation_handoff"]
    assert summary["max_pending_jobs"] == 128
    assert summary["max_wait_ms"] == 30_000
    assert summary["batch_jobs"] == 16
    refute Jason.encode!(summary) =~ first.id
    send(holder, :release)
    complete(first, false)
    complete(second)
    assert entry(first)["attempts"] == 1
    assert entry(second)["attempts"] == 1
    pending_jobs(0)
  end

  test "more than one delivery batch drains without dropping remaining jobs", ctx do
    holder = hold_slot()
    fixtures = for _ <- 1..18, do: saved(ctx.endpoint)
    kick_and_wait(Enum.map(fixtures, & &1.id))
    pending_jobs(18)
    send(holder, :release)
    for f <- fixtures, do: complete(f, false)
    pending_jobs(0)
    assert Enum.all?(fixtures, &(entry(&1)["attempts"] == 1))
    refute_receive {:agent_request, _, _, _}, 50
  end

  test "queue overflow is visible and does not consume omitted work", ctx do
    holder = hold_slot()
    first = saved(ctx.endpoint)
    omitted = saved(ctx.endpoint)
    kick_and_wait([first.id | Enum.map(1..127, &"absent-#{&1}")])
    pending_jobs(128)
    kick_and_wait([omitted.id])
    Outage.eventually(fn -> handoff_error() == "cancellation_handoff_full" end)
    assert length(pending().jobs) == 128
    refute omitted.id in pending().jobs
    assert entry(omitted)["attempts"] == 0
    send(holder, :release)
    complete(first)
    pending_jobs(0)
    {:ok, _} = Delivery.kick([omitted.id])
    complete(omitted)
    assert entry(omitted)["attempts"] == 1
  end

  test "invalid queued identifiers are not retained", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    kick_and_wait([nil, String.duplicate("x", 1025), f.id])
    pending_jobs(1)
    assert handoff_error() == "invalid_cancellation_handoff_jobs"
    assert pending().jobs == [f.id]
    send(holder, :release)
    complete(f)
  end

  test "expired waiting handoff preserves intent for a new explicit trigger", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    authority = Store.authority(:sys.get_state(Coordinator))
    {:ok, pid} = Delivery.kick([f.id], authority, System.monotonic_time(:millisecond) + 150)
    wait_worker(pid)
    pending_jobs(1)
    Outage.eventually(fn -> handoff_error() == "cancellation_handoff_expired" end)
    pending_jobs(0)
    assert entry(f)["attempts"] == 0
    send(holder, :release)
    Outage.eventually(fn -> not Process.alive?(holder) end)
    refute_receive {:agent_request, _, _, _}, 30
    {:ok, _} = Delivery.kick([f.id])
    complete(f)
  end

  test "old timer and monitor messages cannot erase a refreshed handoff", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    kick_and_wait([f.id])
    pending_jobs(1)
    old = pending()
    kick_and_wait([f.id])
    Outage.eventually(fn -> pending().token != old.token end)
    current = pending()
    send(Coordinator, {:cancellation_handoff_expired, old.token})
    send(Coordinator, {:cancellation_handoff_ready, old.token})
    send(Coordinator, {:DOWN, old.ref, :process, holder, :normal})
    Coordinator.snapshot()
    assert pending().token == current.token
    assert entry(f)["attempts"] == 0
    send(holder, :release)
    complete(f)
  end

  test "stale authority cannot replace a pending current-owner handoff", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    kick_and_wait([f.id])
    pending_jobs(1)
    retained = pending()
    stale = {"former-session", make_ref()}
    {:ok, pid} = Delivery.kick(["other-job"], stale)
    wait_worker(pid)
    Coordinator.snapshot()
    assert pending() == retained
    send(holder, :release)
    complete(f)
  end

  test "ownership loss clears the queue and ignores late exit and timer signals", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    kick_and_wait([f.id])
    pending_jobs(1)
    old = pending()
    state = :sys.get_state(Coordinator)
    :ok = LeaseStore.release(state.lease)
    {:ok, foreign} = LeaseStore.acquire(state.lease_name, "other-handoff-owner", 120_000)
    on_exit(fn -> LeaseStore.release(foreign) end)
    send(Coordinator, {:renew_lease, state.lease.fencing_token, state.lease.expires_at_ms})
    assert Coordinator.snapshot()["lease"]["status"] == "standby"
    pending_jobs(0)
    send(holder, :release)
    send(Coordinator, {:cancellation_handoff_ready, old.token})
    send(Coordinator, {:cancellation_handoff_expired, old.token})
    Coordinator.snapshot()
    assert entry(f)["attempts"] == 0
    refute_receive {:agent_request, _, _, _}, 100
  end

  test "unavailable worker supervisor leaves visible unresolved work", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    kick_and_wait([f.id])
    pending_jobs(1)

    Outage.without(KyuubikiWeb.TaskSupervisor, fn ->
      send(holder, :release)
      Outage.eventually(fn -> handoff_error() == "cancellation_worker_unavailable" end)
      pending_jobs(0)
      assert entry(f)["attempts"] == 0
    end)

    {:ok, _} = Delivery.kick([f.id])
    complete(f)
  end

  test "registry loss clears a waiting handoff without consuming its intent", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    kick_and_wait([f.id])
    pending_jobs(1)

    Outage.without(@registry, fn ->
      send(holder, :release)
      Outage.eventually(fn -> handoff_error() == "workflow_runner_registry_unavailable" end)
      pending_jobs(0)
      assert entry(f)["attempts"] == 0
    end)

    {:ok, _} = Delivery.kick([f.id])
    complete(f)
  end

  test "unknown delivery does not start another sweep without a retained trigger", ctx do
    f = saved(ctx.endpoint)
    {:ok, pid} = Delivery.kick([f.id])
    {handler, _} = request("describe_agent")
    reply_description(handler, f)
    {handler, _} = request("cancel_execution")
    send(handler, :disconnect)
    wait_worker(pid)
    assert entry(f)["status"] == "pending"
    assert entry(f)["reason"] == "cancellation_outcome_unknown"
    assert entry(f)["attempts"] == 1
    pending_jobs(0)
    refute_receive {:agent_request, _, _, _}, 100
  end

  test "many triggers during an unknown delivery produce only one follow-up attempt", ctx do
    f = saved(ctx.endpoint)
    {:ok, _} = Delivery.kick([f.id])
    {handler, _} = request("describe_agent")
    reply_description(handler, f)
    {handler, _} = request("cancel_execution")
    for _ <- 1..20, do: kick_and_wait([f.id])
    pending_jobs(1)
    send(handler, :disconnect)
    {handler, retry} = request("cancel_execution")
    assert retry["params"] == target(f)
    acknowledge(handler, retry)
    settled(f)
    pending_jobs(0)
    assert entry(f)["attempts"] == 2
    refute_receive {:agent_request, _, _, _}, 100
  end

  for exit_kind <- [:normal, :killed] do
    test "#{exit_kind} incumbent exit before the deferred message is handled is not lost", ctx do
      holder = hold_slot()
      f = saved(ctx.endpoint)
      authority = Store.authority(:sys.get_state(Coordinator))
      :ok = :sys.suspend(Coordinator)

      try do
        {:ok, pid} = Delivery.kick([f.id], authority)
        wait_worker(pid)

        if unquote(exit_kind) == :normal,
          do: send(holder, :release),
          else: Process.exit(holder, :kill)

        Outage.eventually(fn -> not Process.alive?(holder) end)
      after
        :ok = :sys.resume(Coordinator)
      end

      complete(f)
      assert entry(f)["attempts"] == 1
    end
  end

  test "coordinator restart rebuilds waiting work from intent rather than reviving the queue",
       ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    kick_and_wait([f.id])
    pending_jobs(1)
    old = pending()
    Fixture.restart_coordinator()
    pending_jobs(1)
    refute pending().authority == old.authority
    send(Coordinator, {:cancellation_handoff_expired, old.token})
    send(Coordinator, {:cancellation_handoff_ready, old.token})
    Coordinator.snapshot()
    assert entry(f)["attempts"] == 0
    send(holder, :release)
    complete(f)
    assert entry(f)["attempts"] == 1
  end

  test "handoff wait deadline is not renewed while earlier chunks execute", ctx do
    holder = hold_slot()
    first = saved(ctx.endpoint)
    later = saved(ctx.endpoint)
    authority = Store.authority(:sys.get_state(Coordinator))
    ids = [first.id] ++ Enum.map(1..15, &"chunk-absent-#{&1}") ++ [later.id]
    {:ok, pid} = Delivery.kick(ids, authority, System.monotonic_time(:millisecond) + 500)
    wait_worker(pid)
    pending_jobs(17)
    send(holder, :release)
    {handler, _} = request("describe_agent")
    reply_description(handler, first)
    {handler, cancel} = request("cancel_execution")
    Outage.eventually(fn -> handoff_error() == "cancellation_handoff_expired" end)
    pending_jobs(0)
    assert entry(later)["attempts"] == 0
    acknowledge(handler, cancel)
    settled(first)
    Outage.eventually(fn -> Registry.lookup(@registry, {Delivery, :sweep}) == [] end)
    refute_receive {:agent_request, _, _, _}, 30
    {:ok, _} = Delivery.kick([later.id])
    complete(later)
  end

  test "killed active delivery hands off another job without inventing its own receipt", ctx do
    first = saved(ctx.endpoint)
    second = saved(ctx.endpoint)
    {:ok, worker} = Delivery.kick([first.id])
    {handler, _} = request("describe_agent")
    reply_description(handler, first)
    {handler, _} = request("cancel_execution")
    kick_and_wait([second.id])
    pending_jobs(1)
    Process.exit(worker, :kill)
    send(handler, :disconnect)
    complete(second)
    assert entry(first)["attempts"] == 1
    assert entry(first)["status"] == "dispatching"
    assert entry(first)["execution_target"] == target(first)
    assert entry(second)["attempts"] == 1
  end

  @tag :batch_continuation
  test "continuation selection is bounded and excludes already visited duplicate IDs" do
    unique = Enum.map(1..129, &"continuation-#{&1}")

    ids =
      Stream.concat([
        ["visited", "visited"],
        Enum.flat_map(unique, &[&1, &1]),
        Stream.map(["beyond-overflow-sentinel"], fn _ -> flunk("unbounded continuation scan") end)
      ])

    assert Handoff.continuation_jobs(ids, MapSet.new(["visited"])) == unique
  end

  @tag :batch_continuation
  test "idle sweep hands off jobs beyond its first batch without another trigger", ctx do
    fixtures = for _ <- 1..18, do: saved(ctx.endpoint)
    {:ok, _} = Delivery.kick(Enum.map(fixtures, & &1.id))
    for f <- fixtures, do: complete(f, false)
    assert Enum.all?(fixtures, &(entry(&1)["attempts"] == 1))
    pending_jobs(0)
    refute_receive {:agent_request, _, _, _}, 50
  end

  @tag :batch_continuation
  test "duplicate IDs in one trigger do not retry an unknown delivery", ctx do
    first = saved(ctx.endpoint)
    second = saved(ctx.endpoint)
    {:ok, _} = Delivery.kick([first.id, first.id, second.id, first.id])
    {handler, _} = request("describe_agent")
    reply_description(handler, first)
    {handler, cancel} = request("cancel_execution")
    assert cancel["params"] == target(first)
    send(handler, :disconnect)
    complete(second)
    assert entry(first)["status"] == "pending"
    assert entry(first)["reason"] == "cancellation_outcome_unknown"
    assert entry(first)["attempts"] == 1
    assert entry(second)["attempts"] == 1
    pending_jobs(0)
  end

  @tag :batch_continuation
  test "delivery budget yields untouched jobs but does not retry the partially visited job",
       ctx do
    first = saved(ctx.endpoint)
    second = saved(ctx.endpoint)
    partial = saved(ctx.endpoint, 2)
    later = saved(ctx.endpoint)
    visited = [first, second, partial]
    {:ok, _} = Delivery.kick(Enum.map(visited ++ [later], & &1.id))

    # Three withheld acknowledgements exhaust the real 15-second sweep budget.
    for f <- visited do
      {handler, _} = request("describe_agent", 7_000)
      reply_description(handler, f)
      {_handler, cancel} = request("cancel_execution")
      assert cancel["params"] == target(f)
    end

    {handler, _} = request("describe_agent", 7_000)
    reply_description(handler, later)
    {handler, cancel} = request("cancel_execution")
    assert cancel["params"] == target(later)
    acknowledge(handler, cancel)
    settled(later)

    for f <- visited do
      assert entry(f)["attempts"] == 1
      assert entry(f)["status"] == "pending"
      assert entry(f)["reason"] == "cancellation_outcome_unknown"
    end

    {:ok, runtime} = AnalysisResultStore.get(partial.id)
    assert Enum.at(runtime[@key][Intent.key()]["entries"], 1)["attempts"] == 0
    pending_jobs(0)
    refute_receive {:agent_request, _, _, _}, 100
  end

  @tag :batch_continuation
  test "idle continuation cannot renew an expired original handoff deadline", ctx do
    fixtures = for _ <- 1..17, do: saved(ctx.endpoint)
    [first | tail] = fixtures
    authority = Store.authority(:sys.get_state(Coordinator))
    {:ok, _} = Delivery.kick(Enum.map(fixtures, & &1.id), authority, now() + 500)
    {handler, _} = request("describe_agent")
    reply_description(handler, first)
    {handler, cancel} = request("cancel_execution")
    Process.sleep(550)
    acknowledge(handler, cancel)
    settled(first)
    for f <- Enum.take(tail, 15), do: complete(f, false)
    Outage.eventually(fn -> handoff_error() == "cancellation_handoff_expired" end)
    pending_jobs(0)
    later = List.last(fixtures)
    assert entry(later)["attempts"] == 0
    refute_receive {:agent_request, _, _, _}, 30
    {:ok, _} = Delivery.kick([later.id])
    complete(later)
  end

  @tag :batch_continuation
  test "idle continuation uses the same bounded queue and leaves overflow unattempted", ctx do
    fixtures = for _ <- 1..16, do: saved(ctx.endpoint)
    omitted = saved(ctx.endpoint)
    ids = Enum.map(fixtures, & &1.id) ++ Enum.map(1..128, &"idle-absent-#{&1}") ++ [omitted.id]
    {:ok, _} = Delivery.kick(ids)
    for f <- fixtures, do: complete(f, false)
    Outage.eventually(fn -> handoff_error() == "cancellation_handoff_full" end)
    pending_jobs(0)
    assert entry(omitted)["attempts"] == 0
    refute_receive {:agent_request, _, _, _}, 30
    {:ok, _} = Delivery.kick([omitted.id])
    complete(omitted)
  end

  for prequeued <- [false, true] do
    @tag :duplicate_handoff
    test "duplicate prefix preserves later work with existing queue #{prequeued}", ctx do
      holder = hold_slot()
      first = saved(ctx.endpoint)
      second = saved(ctx.endpoint)

      deadline =
        if unquote(prequeued) do
          kick_and_wait([first.id])
          pending_jobs(1)
          pending().deadline
        end

      kick_and_wait(List.duplicate(first.id, 256) ++ [second.id, first.id])
      pending_jobs(2)
      assert pending().jobs == [first.id, second.id]
      if deadline, do: assert(pending().deadline == deadline)
      assert is_nil(handoff_error())
      send(holder, :release)
      complete(first, false)
      complete(second)
      assert entry(first)["attempts"] == 1
      assert entry(second)["attempts"] == 1
      pending_jobs(0)
    end
  end

  @tag :duplicate_handoff
  test "duplicate-only busy trigger does not claim overflow or retry an unknown outcome", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    kick_and_wait(List.duplicate(f.id, 256))
    pending_jobs(1)
    assert is_nil(handoff_error())
    send(holder, :release)
    {handler, _} = request("describe_agent")
    reply_description(handler, f)
    {handler, cancel} = request("cancel_execution")
    assert cancel["params"] == target(f)
    send(handler, :disconnect)
    Outage.eventually(fn -> entry(f)["status"] == "pending" end)
    Outage.eventually(fn -> Registry.lookup(@registry, {Delivery, :sweep}) == [] end)
    assert entry(f)["attempts"] == 1
    assert entry(f)["reason"] == "cancellation_outcome_unknown"
    pending_jobs(0)
    refute_receive {:agent_request, _, _, _}, 50
  end

  for overflow <- [false, true] do
    @tag :duplicate_handoff
    test "distinct-job capacity with repeated IDs and overflow #{overflow}", ctx do
      holder = hold_slot()
      retained = saved(ctx.endpoint)
      omitted = saved(ctx.endpoint)
      expected = Enum.map(1..127, &"duplicate-absent-#{&1}") ++ [retained.id]
      ids = if unquote(overflow), do: expected ++ [omitted.id], else: expected
      kick_and_wait(Enum.flat_map(ids, &List.duplicate(&1, 3)))
      pending_jobs(128)
      assert pending().jobs == expected

      assert handoff_error() ==
               if(unquote(overflow), do: "cancellation_handoff_full", else: nil)

      assert entry(retained)["attempts"] == 0
      assert entry(omitted)["attempts"] == 0
      send(holder, :release)
      complete(retained)
      pending_jobs(0)
      assert entry(retained)["attempts"] == 1
      assert entry(omitted)["attempts"] == 0

      if unquote(overflow) do
        {:ok, _} = Delivery.kick([omitted.id])
        complete(omitted)
        assert entry(omitted)["attempts"] == 1
      end
    end
  end

  @tag :duplicate_handoff
  test "duplicate-heavy handoff still rejects invalid identifiers without losing later work",
       ctx do
    holder = hold_slot()
    first = saved(ctx.endpoint)
    second = saved(ctx.endpoint)
    ids = List.duplicate(first.id, 256) ++ [nil, String.duplicate("x", 1025), second.id]
    kick_and_wait(ids)
    pending_jobs(2)
    assert pending().jobs == [first.id, second.id]
    assert handoff_error() == "invalid_cancellation_handoff_jobs"
    send(holder, :release)
    complete(first, false)
    complete(second)
    assert entry(first)["attempts"] == 1
    assert entry(second)["attempts"] == 1
  end

  defp now, do: System.monotonic_time(:millisecond)
  defp pending, do: :sys.get_state(Coordinator).cancellation_handoff
  defp handoff_error, do: Coordinator.snapshot()["cancellation_handoff"]["last_error"]

  defp pending_jobs(count),
    do:
      Outage.eventually(fn ->
        Coordinator.snapshot()["cancellation_handoff"]["pending_jobs"] == count
      end)

  defp hold_slot do
    owner = self()

    pid =
      spawn(fn ->
        {:ok, _} = Registry.register(@registry, {Delivery, :sweep}, nil)
        send(owner, :slot_held)
        receive do: (:release -> :ok)
      end)

    on_exit(fn ->
      :sys.replace_state(Coordinator, &Handoff.clear/1)
      if Process.alive?(pid), do: Process.exit(pid, :kill)
    end)

    assert_receive :slot_held
    pid
  end

  defp saved(endpoint, requests \\ 1) do
    f = Fixture.terminal_job(:failed)

    snapshot = %{
      requests:
        for index <- 1..requests do
          suffix = if index == 1, do: "-rpc", else: "-rpc-#{index}"
          %{endpoint: endpoint, request_id: f.id <> suffix, job_id: f.id}
        end,
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

    {:ok, recovery} = Intent.attach(f.runtime[@key], f.job, snapshot)
    runtime = Map.put(f.runtime, @key, recovery)
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, runtime)
    %{f | runtime: runtime}
  end

  defp kick_and_wait(ids) do
    {:ok, pid} = Delivery.kick(ids)
    wait_worker(pid)
  end

  defp wait_worker(pid) do
    ref = Process.monitor(pid)
    assert_receive {:DOWN, ^ref, :process, ^pid, reason}, 2_000
    assert reason in [:normal, :noproc]
  end

  defp complete(f, quiet \\ true) do
    {handler, _} = request("describe_agent")
    reply_description(handler, f)
    {handler, cancel} = request("cancel_execution")
    assert cancel["params"] == target(f)
    acknowledge(handler, cancel)
    settled(f)
    if quiet, do: refute_receive({:agent_request, _, _, _}, 30)
  end

  defp request(method, timeout \\ 2_000) do
    assert_receive {:agent_request, "handoff-agent", handler, %{"method" => ^method} = req},
                   timeout

    {handler, req}
  end

  defp target(f),
    do: %{
      "job_id" => f.id,
      "request_id" => f.id <> "-rpc",
      "generation" => 7,
      "process_instance_id" => "handoff-process"
    }

  defp reply_description(handler, f) do
    send(
      handler,
      {:reply,
       %{
         "ok" => true,
         "result" => %{
           "lifecycle" => %{"process_instance_id" => "handoff-process"},
           "solver_control" => %{
             "schema_version" => "kyuubiki.agent-solver-control/v1",
             "available" => true,
             "active" => [
               target(f)
               |> Map.delete("process_instance_id")
               |> Map.put("cancel_requested", false)
             ]
           }
         }
       }}
    )
  end

  defp acknowledge(handler, request) do
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

  defp entry(f) do
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    hd(runtime[@key][Intent.key()]["entries"])
  end

  defp settled(f), do: Outage.eventually(fn -> entry(f)["status"] == "requested" end)
end
