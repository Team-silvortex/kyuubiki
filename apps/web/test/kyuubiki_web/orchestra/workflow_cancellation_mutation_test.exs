defmodule KyuubikiWeb.Orchestra.WorkflowCancellationMutationTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store, as: Jobs
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
    start_supervised!({CancellableSolverAgent, {self(), "mutation-agent"}})
    assert_receive {:cancellable_agent_ready, "mutation-agent", port}
    endpoint = %{id: "mutation-agent", host: "127.0.0.1", port: port, agent_session_id: "session"}
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

  for phase <- [:reserve, :bind, :authorize, :finish],
      change <- [:job_metadata, :result_metadata] do
    test "#{change} before #{phase} survives delivery without replay or lost updates", ctx do
      f = saved(ctx.endpoint)
      next = saved(ctx.endpoint)
      {sweep, probe} = paused(f, next, unquote(phase))
      retained = mutate(f, unquote(change))
      Probe.resume(probe)
      complete_remaining(f, unquote(phase))
      complete(next)
      assert_done(sweep)

      assert snapshot(f).job == retained.job
      assert Map.delete(snapshot(f).runtime, @key) == Map.delete(retained.runtime, @key)
      assert_settled(f)
      assert_settled(next)
      refute Intent.summary(snapshot(f).runtime[@key])["execution_terminal_confirmed"]
    end
  end

  for phase <- [:reserve, :bind, :authorize, :finish],
      change <- [:replace_intent, :delete_result, :delete_job] do
    test "#{change} before #{phase} fences the old delivery but not a healthy sibling", ctx do
      f = saved(ctx.endpoint)
      next = saved(ctx.endpoint)
      {sweep, probe} = paused(f, next, unquote(phase))
      retained = mutate(f, unquote(change))
      Probe.resume(probe)
      complete(next)
      assert_done(sweep)
      assert snapshot(f) == retained
      assert_settled(next)

      # A new intent requires a new trigger; missing records must never be recreated.
      sweep = kick([f.id])

      if unquote(change) == :replace_intent do
        replacement = %{f | request: "replacement"}
        complete(replacement)
        assert_done(sweep)
        assert_settled(replacement)
      else
        assert_done(sweep)
        assert snapshot(f) == retained
      end
    end
  end

  for phase <- [:reserve, :bind, :finish], change <- [:job_metadata, :result_metadata] do
    test "paired #{phase} persistence rejects a stale #{change} snapshot", ctx do
      f = saved(ctx.endpoint)
      {before, replacement} = proposed_write(f, unquote(phase))
      retained = mutate(f, unquote(change))
      id = f.id

      expected =
        if unquote(change) == :job_metadata,
          do: {:error, {:stale_job_snapshot, id}},
          else: {:error, :stale_analysis_result}

      assert ^expected =
               Jobs.apply_progress_with_result(
                 %{job_id: id, stage: :failed, progress: before.job.progress},
                 before.job,
                 before.runtime,
                 replacement
               )

      assert snapshot(f) == retained
      refute_receive {:agent_request, _, _, _}, 10
      sweep = kick([id])

      if unquote(phase) == :finish,
        do: acknowledge_next(f),
        else: complete(f)

      assert_done(sweep)
      expected_attempts = if unquote(phase) == :reserve, do: 1, else: 2
      assert_settled(f, expected_attempts)
      assert snapshot(f).job == retained.job
      assert Map.delete(snapshot(f).runtime, @key) == Map.delete(retained.runtime, @key)
    end
  end

  defp saved(endpoint) do
    f = Fixture.terminal_job(:failed)
    recovery = attach(f.runtime[@key], f.job, endpoint, "original")
    runtime = Map.put(f.runtime, @key, recovery)
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, runtime)
    Map.merge(f, %{runtime: runtime, endpoint: endpoint, request: "original"})
  end

  defp attach(recovery, job, endpoint, request) do
    snapshot = %{
      requests: [%{endpoint: endpoint, request_id: request, job_id: job.job_id}],
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

    assert {:ok, updated} = Intent.attach(Map.delete(recovery, Intent.key()), job, snapshot)
    updated
  end

  defp paused(f, next, phase) do
    # Pause before the handler re-reads storage, not inside its atomic commit.
    probe =
      {_pid, marker} =
      Probe.install(Coordinator, fn
        {:cancellation_intent, id, {:sweep, _, action}} when id == f.id and is_tuple(action) ->
          elem(action, 0) == phase

        _ ->
          false
      end)

    sweep = kick([f.id, next.id])
    if phase != :reserve, do: describe(f)
    if phase == :finish, do: acknowledge_next(f)
    assert_receive {:cancellation_paused, ^marker, _}, 2_000
    {sweep, probe}
  end

  defp mutate(f, change) do
    before = snapshot(f)

    case change do
      :job_metadata ->
        assert {:ok, _} = Jobs.update_metadata(f.id, %{"message" => "concurrent annotation"})

      :result_metadata ->
        updated = Map.put(before.runtime, "annotation", %{"owner" => "concurrent editor"})
        assert :ok = AnalysisResultStore.compare_and_swap(f.id, before.runtime, updated)

      :replace_intent ->
        recovery = attach(before.runtime[@key], before.job, f.endpoint, "replacement")
        updated = Map.put(before.runtime, @key, recovery)
        assert :ok = AnalysisResultStore.compare_and_swap(f.id, before.runtime, updated)

      :delete_result ->
        assert {:ok, _} = AnalysisResultStore.delete(f.id)

      :delete_job ->
        assert {:ok, _} = Jobs.delete(f.id)
    end

    snapshot(f)
  end

  defp proposed_write(f, phase) do
    assert {:ok, bucket} = Store.access(f.id, :load)
    identity = Intent.identity(hd(bucket["entries"]))

    token =
      if phase != :reserve do
        assert {:ok, token} = Store.access(f.id, {:reserve, bucket["intent_id"], identity})
        if phase == :finish, do: assert(:ok = Store.access(f.id, {:bind, token, target(f)}))
        token
      end

    before = snapshot(f)
    bucket = before.runtime[@key][Intent.key()]

    result =
      case phase do
        :reserve -> Intent.reserve(bucket, identity)
        :bind -> Intent.bind(bucket, token, target(f))
        :finish -> Intent.finish(bucket, token, "requested")
      end

    assert {:ok, proposed, _} = result
    {before, put_in(before.runtime, [@key, Intent.key()], proposed)}
  end

  defp snapshot(f) do
    %{job: value(Jobs.get(f.id)), runtime: value(AnalysisResultStore.get(f.id))}
  end

  defp value({:ok, record}), do: record
  defp value(:error), do: nil

  defp kick(ids) do
    Registry.unregister(@registry, {Delivery, :sweep})
    assert {:ok, pid} = Delivery.kick(ids)
    {pid, Process.monitor(pid)}
  end

  defp assert_done({pid, ref}) do
    assert_receive {:DOWN, ^ref, :process, ^pid, :normal}, 2_000
    refute_receive {:agent_request, _, _, _}, 20
  end

  defp complete_remaining(f, :reserve), do: complete(f)
  defp complete_remaining(_f, :finish), do: :ok
  defp complete_remaining(f, _), do: acknowledge_next(f)

  defp complete(f) do
    describe(f)
    acknowledge_next(f)
  end

  defp describe(f) do
    {handler, _} = receive_request("describe_agent")

    send(
      handler,
      {:reply,
       %{
         "ok" => true,
         "result" => %{
           "lifecycle" => %{"process_instance_id" => "mutation-process"},
           "solver_control" => %{
             "schema_version" => "kyuubiki.agent-solver-control/v1",
             "available" => true,
             "active" => [
               %{
                 "job_id" => f.id,
                 "request_id" => f.request,
                 "generation" => 7,
                 "cancel_requested" => false
               }
             ]
           }
         }
       }}
    )
  end

  defp acknowledge_next(f) do
    {handler, request} = receive_request("cancel_execution")
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
  end

  defp receive_request(method) do
    assert_receive {:agent_request, "mutation-agent", handler, %{"method" => ^method} = request},
                   2_000

    {handler, request}
  end

  defp target(f),
    do: %{
      "job_id" => f.id,
      "request_id" => f.request,
      "process_instance_id" => "mutation-process",
      "generation" => 7
    }

  defp assert_settled(f, attempts \\ 1) do
    assert {:ok, bucket} = Store.access(f.id, :load)
    assert [entry] = bucket["entries"]
    assert entry["request_id"] == f.request
    assert entry["attempts"] == attempts
    assert entry["status"] == "requested"
    assert entry["execution_target"] == target(f)
  end
end
