defmodule KyuubikiWeb.Orchestra.WorkflowCancellationInputTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
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
  @moduletag capture_log: true

  setup do
    Outage.setup()
    Outage.eventually(fn -> Registry.lookup(@registry, {Delivery, :sweep}) == [] end)
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])
    start_supervised!({CancellableSolverAgent, {self(), "input-agent"}})
    assert_receive {:cancellable_agent_ready, "input-agent", port}
    endpoint = %{id: "input-agent", host: "127.0.0.1", port: port, agent_session_id: "session"}
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

  for mode <- [:idle, :busy],
      {label, invalid} <- [
        nil_value: nil,
        boolean: false,
        integer: 42,
        map: %{},
        list: [],
        empty: "",
        oversized: String.duplicate("x", 1025)
      ] do
    test "#{mode} sweep isolates #{label} identifiers from later valid work", ctx do
      holder = if unquote(mode) == :busy, do: hold_slot()
      f = saved(ctx.endpoint)
      coordinator = Process.whereis(Coordinator)
      {:ok, pid} = Delivery.kick([unquote(Macro.escape(invalid)), f.id])

      if unquote(mode) == :busy do
        wait_worker(pid)

        Outage.eventually(fn ->
          Coordinator.snapshot()["cancellation_handoff"]["pending_jobs"] == 1
        end)

        assert :sys.get_state(Coordinator).cancellation_handoff.jobs == [f.id]
        send(holder, :release)
      end

      complete(f)
      if unquote(mode) == :idle, do: wait_worker(pid)
      assert Process.whereis(Coordinator) == coordinator
      assert entry(f)["attempts"] == 1
      quiet()
    end
  end

  test "invalid middle identifier cannot abandon the rest of an idle sweep", ctx do
    first = saved(ctx.endpoint)
    later = saved(ctx.endpoint)
    {:ok, pid} = Delivery.kick([first.id, nil, later.id])
    complete(first)
    complete(later)
    wait_worker(pid)
    assert entry(first)["attempts"] == 1
    assert entry(later)["attempts"] == 1
    quiet()
  end

  test "invalid-only idle input exits normally without reserving unrelated work", ctx do
    f = saved(ctx.endpoint)
    {:ok, pid} = Delivery.kick([nil, false, 42, %{}, [], "", String.duplicate("x", 1025)])
    wait_worker(pid)
    Fixture.unchanged(f)
    quiet()
  end

  for budget <- [:shorter, :longer] do
    @tag :handoff_admission
    test "#{budget} invalid-only trigger cannot rearm an existing handoff", ctx do
      holder = hold_slot()
      f = saved(ctx.endpoint)
      {:ok, pid} = Delivery.kick([f.id])
      wait_worker(pid)
      retained = pending_for(f.id)

      deadline =
        if unquote(budget) == :shorter,
          do: System.monotonic_time(:millisecond) + 150,
          else: retained.deadline + 30_000

      {:ok, pid} = Delivery.kick([nil, false, %{}, ""], retained.authority, deadline)
      wait_worker(pid)

      Outage.eventually(fn ->
        Coordinator.snapshot()["cancellation_handoff"]["last_error"] ==
          "invalid_cancellation_handoff_jobs"
      end)

      if unquote(budget) == :shorter,
        do: Process.sleep(max(deadline - System.monotonic_time(:millisecond) + 10, 0))

      assert :sys.get_state(Coordinator).cancellation_handoff == retained
      Fixture.unchanged(f)
      send(holder, :release)
      complete(f)
      assert entry(f)["attempts"] == 1
      quiet()
    end
  end

  @tag :handoff_admission
  test "empty internal handoff is a no-op for an existing queue", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    {:ok, pid} = Delivery.kick([f.id])
    wait_worker(pid)
    retained = pending_for(f.id)

    GenServer.cast(
      Coordinator,
      {:cancellation_handoff, holder, [], retained.authority, retained.deadline - 1_000}
    )

    assert :sys.get_state(Coordinator).cancellation_handoff == retained
    send(holder, :release)
    complete(f)
    quiet()
  end

  @tag :handoff_admission
  test "mixed valid input still merges into an unexpired queue with the earliest deadline", ctx do
    holder = hold_slot()
    first = saved(ctx.endpoint)
    later = saved(ctx.endpoint)
    {:ok, pid} = Delivery.kick([first.id])
    wait_worker(pid)
    retained = pending_for(first.id)
    deadline = retained.deadline - 1_000
    {:ok, pid} = Delivery.kick([nil, later.id], retained.authority, deadline)
    wait_worker(pid)

    Outage.eventually(fn ->
      Coordinator.snapshot()["cancellation_handoff"]["pending_jobs"] == 2
    end)

    pending = :sys.get_state(Coordinator).cancellation_handoff
    assert pending.jobs == [first.id, later.id]
    assert pending.deadline == deadline
    send(holder, :release)
    complete(first)
    complete(later)
    assert entry(first)["attempts"] == 1
    assert entry(later)["attempts"] == 1
    quiet()
  end

  for order <- [:trigger_first, :timer_first], job <- [:new_job, :same_job] do
    @tag :handoff_admission
    test "#{order} fresh #{job} trigger does not inherit an expired handoff", ctx do
      holder = hold_slot()
      old = saved(ctx.endpoint)
      fresh = if unquote(job) == :same_job, do: old, else: saved(ctx.endpoint)
      authority = Store.authority(:sys.get_state(Coordinator))
      deadline = System.monotonic_time(:millisecond) + 300
      {:ok, pid} = Delivery.kick([old.id], authority, deadline)
      wait_worker(pid)
      expired = pending_for(old.id)
      coordinator = Process.whereis(Coordinator)
      :ok = :sys.suspend(coordinator)

      try do
        if unquote(order) == :timer_first, do: wait_expiry_message(coordinator, expired.token)
        {:ok, pid} = Delivery.kick([fresh.id], authority)
        wait_worker(pid)
        wait_expiry_message(coordinator, expired.token)
        {:messages, messages} = Process.info(coordinator, :messages)

        timer_index =
          Enum.find_index(messages, &(&1 == {:cancellation_handoff_expired, expired.token}))

        trigger_index =
          Enum.find_index(messages, fn
            {:"$gen_cast", {:cancellation_handoff, ^holder, _, ^authority, _}} -> true
            _ -> false
          end)

        assert is_integer(timer_index) and is_integer(trigger_index)
        assert trigger_index < timer_index == (unquote(order) == :trigger_first)
      after
        :ok = :sys.resume(coordinator)
      end

      pending = :sys.get_state(coordinator).cancellation_handoff
      assert pending != nil
      assert pending.jobs == [fresh.id]
      assert pending.deadline > expired.deadline
      assert pending.authority == authority
      assert pending.token != expired.token

      assert Coordinator.snapshot()["cancellation_handoff"]["last_error"] ==
               "cancellation_handoff_expired"

      send(coordinator, {:cancellation_handoff_expired, expired.token})
      send(coordinator, {:cancellation_handoff_ready, expired.token})
      send(coordinator, {:DOWN, expired.ref, :process, holder, :normal})
      assert :sys.get_state(coordinator).cancellation_handoff == pending
      Fixture.unchanged(old)
      send(holder, :release)
      complete(fresh)
      assert entry(fresh)["attempts"] == 1
      quiet()

      if unquote(job) == :new_job do
        Fixture.unchanged(old)
        {:ok, _} = Delivery.kick([old.id])
        complete(old)
        assert entry(old)["attempts"] == 1
        quiet()
      end
    end
  end

  for rejected <- [:stale_authority, :expired_trigger, :invalid_only] do
    @tag :handoff_admission
    test "#{rejected} trigger cannot revive expired waiting work", ctx do
      holder = hold_slot()
      old = saved(ctx.endpoint)
      later = saved(ctx.endpoint)
      authority = Store.authority(:sys.get_state(Coordinator))
      deadline = System.monotonic_time(:millisecond) + 300
      {:ok, pid} = Delivery.kick([old.id], authority, deadline)
      wait_worker(pid)
      expired = pending_for(old.id)
      coordinator = Process.whereis(Coordinator)
      :ok = :sys.suspend(coordinator)

      try do
        {ids, incoming_authority, incoming_deadline} =
          case unquote(rejected) do
            :stale_authority -> {[later.id], {"retired-session", make_ref()}, deadline + 30_000}
            :expired_trigger -> {[later.id], authority, deadline}
            :invalid_only -> {[nil, %{}, ""], authority, deadline + 30_000}
          end

        {:ok, pid} = Delivery.kick(ids, incoming_authority, incoming_deadline)
        wait_worker(pid)
        wait_expiry_message(coordinator, expired.token)
      after
        :ok = :sys.resume(coordinator)
      end

      Outage.eventually(fn -> :sys.get_state(coordinator).cancellation_handoff == nil end)
      Fixture.unchanged(old)
      Fixture.unchanged(later)
      send(holder, :release)
      quiet()
      {:ok, _} = Delivery.kick([later.id])
      complete(later)
      assert entry(later)["attempts"] == 1
      Fixture.unchanged(old)
      quiet()
    end
  end

  for budget <- [:shorter, :longer], position <- [:head, :tail] do
    @tag :capacity_admission
    test "#{budget} rejected overflow cannot rearm retained #{position} work", ctx do
      holder = hold_slot()
      f = saved(ctx.endpoint)
      omitted = saved(ctx.endpoint)
      padding = Enum.map(1..127, &"missing-capacity-#{&1}")
      ids = if unquote(position) == :head, do: [f.id | padding], else: padding ++ [f.id]
      {:ok, pid} = Delivery.kick(ids)
      wait_worker(pid)
      retained = pending_ids(ids)

      deadline =
        if unquote(budget) == :shorter,
          do: System.monotonic_time(:millisecond) + 150,
          else: retained.deadline + 30_000

      {:ok, pid} = Delivery.kick([nil, omitted.id, omitted.id], retained.authority, deadline)
      wait_worker(pid)
      await_full_error()

      if unquote(budget) == :shorter,
        do: Process.sleep(max(deadline - System.monotonic_time(:millisecond) + 10, 0))

      assert :sys.get_state(Coordinator).cancellation_handoff == retained
      Fixture.unchanged(f)
      Fixture.unchanged(omitted)
      send(holder, :release)
      complete(f)
      drained()
      assert entry(f)["attempts"] == 1
      Fixture.unchanged(omitted)
      {:ok, _} = Delivery.kick([omitted.id])
      complete(omitted)
      assert entry(omitted)["attempts"] == 1
      drained()
    end
  end

  for outcome <- [:acknowledged, :unknown] do
    @tag :capacity_admission
    test "full queue still accepts an overlapping trigger with #{outcome} delivery", ctx do
      holder = hold_slot()
      f = saved(ctx.endpoint)
      omitted = saved(ctx.endpoint)
      ids = [f.id | Enum.map(1..127, &"missing-overlap-#{&1}")]
      {:ok, pid} = Delivery.kick(ids)
      wait_worker(pid)
      retained = pending_ids(ids)
      deadline = retained.deadline - 1_000
      {:ok, pid} = Delivery.kick([omitted.id, f.id], retained.authority, deadline)
      wait_worker(pid)
      await_full_error()
      current = pending_ids(ids)
      assert current.token != retained.token
      assert current.deadline == deadline
      send(holder, :release)

      if unquote(outcome) == :unknown do
        observe(f)
        {handler, cancel} = request("cancel_execution")
        assert cancel["params"] == target(f)
        send(handler, :disconnect)
      else
        complete(f)
      end

      drained()
      assert entry(f)["attempts"] == 1
      Fixture.unchanged(omitted)

      if unquote(outcome) == :unknown do
        assert entry(f)["status"] == "pending"
        assert entry(f)["reason"] == "cancellation_outcome_unknown"
        assert entry(f)["execution_target"] == target(f)
      end
    end
  end

  @tag :capacity_admission
  test "partially admitted overflow keeps accepted work and leaves the rest unattempted", ctx do
    holder = hold_slot()
    first = saved(ctx.endpoint)
    accepted = saved(ctx.endpoint)
    omitted = saved(ctx.endpoint)
    ids = [first.id | Enum.map(1..126, &"missing-partial-#{&1}")]
    {:ok, pid} = Delivery.kick(ids)
    wait_worker(pid)
    retained = pending_ids(ids)
    deadline = retained.deadline - 1_000
    {:ok, pid} = Delivery.kick([accepted.id, omitted.id], retained.authority, deadline)
    wait_worker(pid)
    await_full_error()
    current = pending_ids(ids ++ [accepted.id])
    assert current.deadline == deadline
    assert current.token != retained.token
    send(holder, :release)
    complete(first)
    complete(accepted)
    drained()
    assert entry(first)["attempts"] == 1
    assert entry(accepted)["attempts"] == 1
    Fixture.unchanged(omitted)
    {:ok, _} = Delivery.kick([omitted.id])
    complete(omitted)
    assert entry(omitted)["attempts"] == 1
    drained()
  end

  @tag :capacity_admission
  test "expired full queue does not consume capacity of a fresh explicit trigger", ctx do
    holder = hold_slot()
    old = saved(ctx.endpoint)
    fresh = saved(ctx.endpoint)
    authority = Store.authority(:sys.get_state(Coordinator))
    ids = [old.id | Enum.map(1..127, &"missing-expired-#{&1}")]
    {:ok, pid} = Delivery.kick(ids, authority, System.monotonic_time(:millisecond) + 300)
    wait_worker(pid)
    expired = pending_ids(ids)
    coordinator = Process.whereis(Coordinator)
    :ok = :sys.suspend(coordinator)

    try do
      {:ok, pid} = Delivery.kick([fresh.id], authority)
      wait_worker(pid)
      wait_expiry_message(coordinator, expired.token)
    after
      :ok = :sys.resume(coordinator)
    end

    current = pending_ids([fresh.id])
    assert current.deadline > expired.deadline
    send(holder, :release)
    complete(fresh)
    drained()
    assert entry(fresh)["attempts"] == 1
    Fixture.unchanged(old)
  end

  for malformed <- [:legacy, :corrupt] do
    test "#{malformed} stored record is skipped without mutation or blocking its sibling", ctx do
      rejected = saved(ctx.endpoint)
      later = saved(ctx.endpoint)

      runtime =
        case unquote(malformed) do
          :legacy -> Map.delete(rejected.runtime, @key)
          :corrupt -> put_in(rejected.runtime, [@key, Intent.key(), "sha256"], "damaged")
        end

      assert :ok = AnalysisResultStore.put(rejected.id, runtime)
      {:ok, pid} = Delivery.kick([rejected.id, "missing-input-job", later.id])
      complete(later)
      wait_worker(pid)
      assert {:ok, ^runtime} = AnalysisResultStore.get(rejected.id)
      assert entry(later)["attempts"] == 1
      quiet()
    end
  end

  test "invalid identifiers after a full batch still allow bounded continuation", ctx do
    fixtures = for _ <- 1..17, do: saved(ctx.endpoint)
    {batch, [later]} = Enum.split(fixtures, 16)
    {:ok, _} = Delivery.kick(Enum.map(batch, & &1.id) ++ [nil, %{}, later.id])
    for f <- fixtures, do: complete(f)
    assert Enum.all?(fixtures, &(entry(&1)["attempts"] == 1))
    quiet()
  end

  test "invalid input does not authorize retry of an earlier unknown delivery", ctx do
    first = saved(ctx.endpoint)
    later = saved(ctx.endpoint)
    {:ok, pid} = Delivery.kick([first.id, nil, first.id, later.id])
    observe(first)
    {handler, cancel} = request("cancel_execution")
    assert cancel["params"] == target(first)
    send(handler, :disconnect)
    complete(later)
    wait_worker(pid)
    assert entry(first)["status"] == "pending"
    assert entry(first)["reason"] == "cancellation_outcome_unknown"
    assert entry(first)["attempts"] == 1
    quiet()
  end

  test "skipping invalid input cannot revive stale sweep authority", ctx do
    f = saved(ctx.endpoint)
    {:ok, pid} = Delivery.kick([nil, f.id], {"retired-session", make_ref()})
    wait_worker(pid)
    Fixture.unchanged(f)
    quiet()
  end

  test "storage outage is not treated as an invalid identifier to skip", ctx do
    f = saved(ctx.endpoint)
    authority = Store.authority(:sys.get_state(Coordinator))

    Outage.without(Outage.store_child(), fn ->
      {:ok, pid} = Delivery.kick([nil, f.id], authority)
      wait_worker(pid)
      quiet()
    end)

    Fixture.unchanged(f)
  end

  defp saved(endpoint) do
    f = Fixture.terminal_job(:failed)

    snapshot = %{
      requests: [%{endpoint: endpoint, request_id: f.id <> "-rpc", job_id: f.id}],
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

    {:ok, recovery} = Intent.attach(f.runtime[@key], f.job, snapshot)
    runtime = Map.put(f.runtime, @key, recovery)
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, runtime)
    %{f | runtime: runtime}
  end

  defp complete(f) do
    observe(f)
    {handler, request} = request("cancel_execution")
    assert request["params"] == target(f)

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

    Outage.eventually(fn -> entry(f)["status"] == "requested" end)
  end

  defp observe(f) do
    {handler, _} = request("describe_agent")

    send(
      handler,
      {:reply,
       %{
         "ok" => true,
         "result" => %{
           "lifecycle" => %{"process_instance_id" => "input-process"},
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

  defp target(f),
    do: %{
      "job_id" => f.id,
      "request_id" => f.id <> "-rpc",
      "generation" => 7,
      "process_instance_id" => "input-process"
    }

  defp entry(f) do
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    hd(runtime[@key][Intent.key()]["entries"])
  end

  defp request(method) do
    assert_receive {:agent_request, "input-agent", handler, %{"method" => ^method} = req}, 2_000
    {handler, req}
  end

  defp hold_slot do
    owner = self()

    pid =
      spawn(fn ->
        {:ok, _} = Registry.register(@registry, {Delivery, :sweep}, nil)
        send(owner, :slot_held)
        receive do: (:release -> :ok)
      end)

    on_exit(fn ->
      if Process.alive?(pid), do: Process.exit(pid, :kill)
    end)

    assert_receive :slot_held
    pid
  end

  defp wait_worker(pid) do
    ref = Process.monitor(pid)
    assert_receive {:DOWN, ^ref, :process, ^pid, reason}, 2_000
    assert reason in [:normal, :noproc]
  end

  defp pending_for(id) do
    pending_ids([id])
  end

  defp pending_ids(ids) do
    Outage.eventually(fn ->
      case :sys.get_state(Coordinator).cancellation_handoff do
        %{jobs: ^ids} -> true
        _ -> false
      end
    end)

    :sys.get_state(Coordinator).cancellation_handoff
  end

  defp await_full_error do
    Outage.eventually(fn ->
      Coordinator.snapshot()["cancellation_handoff"]["last_error"] == "cancellation_handoff_full"
    end)
  end

  defp drained do
    Outage.eventually(fn -> :sys.get_state(Coordinator).cancellation_handoff == nil end)
    quiet()
  end

  defp wait_expiry_message(coordinator, token) do
    Outage.eventually(fn ->
      {:messages, messages} = Process.info(coordinator, :messages)
      {:cancellation_handoff_expired, token} in messages
    end)
  end

  defp quiet do
    Outage.eventually(fn -> Registry.lookup(@registry, {Delivery, :sweep}) == [] end)
    refute_receive {:agent_request, _, _, _}, 30
  end
end
