defmodule KyuubikiWeb.Orchestra.WorkflowCancellationSelectionTest do
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
    start_supervised!({CancellableSolverAgent, {self(), "selection-agent"}})
    assert_receive {:cancellable_agent_ready, "selection-agent", port}

    endpoint = %{
      id: "selection-agent",
      host: "127.0.0.1",
      port: port,
      agent_session_id: "session"
    }

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

  for route <- [:busy, :internal], kind <- [:integer, :map, :oversized] do
    test "#{route} selection cannot let a distinct #{kind} prefix hide valid work", ctx do
      holder = hold_slot()
      f = saved(ctx.endpoint)
      ids = invalid_prefix(unquote(kind)) ++ [f.id, f.id]

      case unquote(route) do
        :busy ->
          {:ok, pid} = Delivery.kick(ids)
          wait_worker(pid)

        :internal ->
          authority = Store.authority(:sys.get_state(Coordinator))

          GenServer.cast(
            Coordinator,
            {:cancellation_handoff, holder, ids, authority, now() + 30_000}
          )
      end

      await_error("invalid_cancellation_handoff_jobs")
      assert %{jobs: [id]} = pending()
      assert id == f.id
      Fixture.unchanged(f)

      if unquote(kind) == :oversized,
        do: Process.exit(holder, :kill),
        else: send(holder, :release)

      complete(f)
      drained()
      assert entry(f)["attempts"] == 1
    end
  end

  test "selection stops at the valid overflow sentinel and bounds invalid metadata" do
    unique = Enum.map(1..129, &"selected-#{&1}")

    ids =
      Stream.concat([
        invalid_prefix(:map),
        ["visited", "visited"],
        Enum.flat_map(unique, &[&1, &1]),
        Stream.map([:after_sentinel], fn _ -> flunk("selection scanned past valid sentinel") end)
      ])

    selected = Handoff.continuation_jobs(ids, MapSet.new(["visited"]))
    assert Enum.filter(selected, &Handoff.valid_job_id?/1) == unique
    assert Enum.reject(selected, &Handoff.valid_job_id?/1) == [nil]
    assert length(selected) == 130
  end

  for overflow <- [false, true] do
    test "invalid members do not consume valid capacity with overflow #{overflow}", ctx do
      holder = hold_slot()
      retained = saved(ctx.endpoint)
      omitted = saved(ctx.endpoint)
      expected = Enum.map(1..127, &"selection-missing-#{&1}") ++ [retained.id]
      valid = if unquote(overflow), do: expected ++ [omitted.id], else: expected
      ids = invalid_prefix(:integer) ++ Enum.flat_map(valid, &[&1, %{"invalid" => &1}, &1])
      {:ok, pid} = Delivery.kick(ids)
      wait_worker(pid)

      await_error(
        if(unquote(overflow),
          do: "cancellation_handoff_full",
          else: "invalid_cancellation_handoff_jobs"
        )
      )

      assert %{jobs: ^expected} = pending()
      Fixture.unchanged(retained)
      Fixture.unchanged(omitted)
      send(holder, :release)
      complete(retained)
      drained()
      assert entry(retained)["attempts"] == 1
      Fixture.unchanged(omitted)
      {:ok, _} = Delivery.kick([omitted.id])
      complete(omitted)
      drained()
      assert entry(omitted)["attempts"] == 1
    end
  end

  for outcome <- [:acknowledged, :unknown] do
    test "idle continuation skips an invalid prefix after #{outcome} without replay", ctx do
      [first | tail] = batch = for _ <- 1..16, do: saved(ctx.endpoint)
      later = saved(ctx.endpoint)

      ids =
        Enum.map(batch, & &1.id) ++ invalid_prefix(:map) ++ [first.id, later.id, later.id]

      {:ok, _} = Delivery.kick(ids)
      complete(first, unquote(outcome))
      for f <- tail, do: complete(f)
      complete(later)
      drained()
      await_error("invalid_cancellation_handoff_jobs")
      assert Enum.all?(batch ++ [later], &(entry(&1)["attempts"] == 1))

      if unquote(outcome) == :unknown do
        assert entry(first)["status"] == "pending"
        assert entry(first)["reason"] == "cancellation_outcome_unknown"
        assert entry(first)["execution_target"] == target(first)
      end
    end
  end

  test "large invalid-only hints retain the live queue without claiming capacity overflow", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    {:ok, pid} = Delivery.kick([f.id])
    wait_worker(pid)
    Outage.eventually(fn -> not is_nil(pending()) end)
    previous = pending()

    {:ok, pid} =
      Delivery.kick(invalid_prefix(:oversized), previous.authority, previous.deadline - 1_000)

    wait_worker(pid)
    await_error("invalid_cancellation_handoff_jobs")
    assert pending() == previous
    Fixture.unchanged(f)
    send(holder, :release)
    complete(f)
    drained()
    assert entry(f)["attempts"] == 1
  end

  test "selection cannot turn stale authority into a valid handoff", ctx do
    holder = hold_slot()
    f = saved(ctx.endpoint)
    ids = invalid_prefix(:integer) ++ [f.id]
    {:ok, pid} = Delivery.kick(ids, {"retired-selection", make_ref()})
    wait_worker(pid)
    Coordinator.snapshot()
    assert pending() == nil
    Fixture.unchanged(f)
    send(holder, :release)
    drained()
  end

  defp invalid_prefix(kind) do
    Enum.map(1..160, fn index ->
      case kind do
        :integer -> index
        :map -> %{"invalid" => index}
        :oversized -> String.duplicate("x", 1025) <> Integer.to_string(index)
      end
    end)
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

  defp complete(f, outcome \\ :acknowledged) do
    {handler, _} = request("describe_agent")

    send(
      handler,
      {:reply,
       %{
         "ok" => true,
         "result" => %{
           "lifecycle" => %{"process_instance_id" => "selection-process"},
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

    {handler, request} = request("cancel_execution")
    assert request["params"] == target(f)

    if outcome == :unknown do
      send(handler, :disconnect)
      Outage.eventually(fn -> entry(f)["reason"] == "cancellation_outcome_unknown" end)
    else
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
  end

  defp target(f),
    do: %{
      "job_id" => f.id,
      "request_id" => f.id <> "-rpc",
      "generation" => 7,
      "process_instance_id" => "selection-process"
    }

  defp entry(f) do
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    hd(runtime[@key][Intent.key()]["entries"])
  end

  defp request(method) do
    assert_receive {:agent_request, "selection-agent", handler, %{"method" => ^method} = req},
                   2_000

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

  defp pending, do: :sys.get_state(Coordinator).cancellation_handoff

  defp await_error(reason) do
    Outage.eventually(fn ->
      Coordinator.snapshot()["cancellation_handoff"]["last_error"] == reason
    end)
  end

  defp drained do
    Outage.eventually(fn -> is_nil(pending()) end)
    Outage.eventually(fn -> Registry.lookup(@registry, {Delivery, :sweep}) == [] end)
    refute_receive {:agent_request, _, _, _}, 30
  end

  defp now, do: System.monotonic_time(:millisecond)
end
