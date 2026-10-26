defmodule KyuubikiWeb.Orchestra.WorkflowCancellationDirectoryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Orchestra.WorkflowCancellationDelivery, as: Delivery
  alias KyuubikiWeb.Orchestra.WorkflowCancellationIntent, as: Intent
  alias KyuubikiWeb.Orchestra.WorkflowCancellationStore, as: Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Playground.{AgentPool, AgentRegistry}
  alias KyuubikiWeb.TestSupport.CancellableSolverAgent
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCancellationProbe, as: Probe
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @key "_workflow_recovery"
  @registry KyuubikiWeb.Orchestra.WorkflowRunnerRegistry

  setup do
    Outage.setup()
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])
    endpoint = agent("durable-directory")
    Application.put_env(:kyuubiki_web, AgentPool, discovery: :static, endpoints: [endpoint])
    :ok = AgentPool.reload()

    on_exit(fn ->
      for {pid, _} <- Registry.lookup(@registry, {Delivery, :sweep}),
          do: Process.exit(pid, :kill)

      Application.put_env(:kyuubiki_web, AgentPool, previous)
      :ok = Outage.start(AgentPool)
      :ok = AgentPool.reload()
    end)

    %{endpoint: endpoint}
  end

  for change <- [:removed, :session, :host, :port, :id] do
    test "#{change} during TCP observation prevents durable cancellation", ctx do
      f = saved(ctx.endpoint)
      sweep = kick(f.id)
      {handler, _} = receive_request("describe_agent")
      configure(changed(ctx.endpoint, unquote(change)))
      reply_description(handler, f)
      assert_no_send(sweep)
      entry = only_entry(f.id)
      assert entry["status"] == "pending"
      assert entry["reason"] == "original_endpoint_not_configured"
      assert entry["attempts"] == 1
      assert entry["execution_target"] == exact(f, "one")
    end
  end

  for phase <- [:reserve, :bind, :authorize] do
    test "node removal while #{phase} waits cannot authorize a later send", ctx do
      f = saved(ctx.endpoint)
      probe = {_pid, token} = pause_store(f.id, unquote(phase))
      sweep = kick(f.id)

      if unquote(phase) != :reserve do
        {handler, _} = receive_request("describe_agent")
        reply_description(handler, f)
      end

      assert_receive {:cancellation_paused, ^token, _}, 2_000
      configure([])
      Probe.resume(probe)
      assert_no_send(sweep)
      assert only_entry(f.id)["reason"] == "original_endpoint_not_configured"
    end
  end

  test "saved target still rechecks membership after durable authorization", ctx do
    f = saved(ctx.endpoint)
    bind_saved(f)
    probe = {_pid, token} = pause_store(f.id, :authorize)
    sweep = kick(f.id)
    assert_receive {:cancellation_paused, ^token, _}, 2_000
    configure(changed(ctx.endpoint, :session))
    Probe.resume(probe)
    assert_no_send(sweep)
    entry = only_entry(f.id)
    assert entry["attempts"] == 2
    assert entry["reason"] == "original_endpoint_not_configured"
    assert entry["execution_target"] == exact(f, "one")
  end

  for outcome <- [:requested, :not_observed, :lost_ack], change <- [:removed, :unavailable] do
    test "#{outcome} remains truthful after #{change} before the next cached target",
         ctx do
      f = saved(ctx.endpoint, ["one", "two"])
      sweep = kick(f.id)
      {handler, _} = receive_request("describe_agent")
      reply_description(handler, f)
      {handler, request} = receive_request("cancel_execution")
      assert request["params"] == exact(f, "one")
      withdraw(unquote(change))
      acknowledge(handler, request, unquote(outcome))
      assert_no_send(sweep)
      [first, second] = entries(f.id)
      assert first["execution_target"] == exact(f, "one")
      assert_outcome(first, unquote(outcome))
      assert second["status"] == "pending"
      assert second["reason"] == directory_reason(unquote(change))
      assert second["attempts"] == 1
    end
  end

  test "directory loss during observation retains the bound target without sending", ctx do
    f = saved(ctx.endpoint)
    sweep = kick(f.id)
    {handler, _} = receive_request("describe_agent")
    assert :ok = Outage.stop(AgentPool)
    reply_description(handler, f)
    assert_no_send(sweep)
    entry = only_entry(f.id)
    assert entry["status"] == "pending"
    assert entry["reason"] == "endpoint_directory_unavailable"
    assert entry["execution_target"] == exact(f, "one")
  end

  for change <- [:removed, :session] do
    test "registry #{change} during observation never uses the retained static endpoint", ctx do
      endpoint = ctx.endpoint
      attrs = Map.new(endpoint, fn {key, value} -> {Atom.to_string(key), value} end)
      assert {:ok, _} = AgentRegistry.register(attrs)
      on_exit(fn -> AgentRegistry.unregister(endpoint.id) end)
      Application.put_env(:kyuubiki_web, AgentPool, discovery: :registry, endpoints: [endpoint])
      :ok = AgentPool.reload()
      f = saved(endpoint)
      sweep = kick(f.id)
      {handler, _} = receive_request("describe_agent")

      assert :ok =
               AgentRegistry.unregister(endpoint.id, %{
                 "agent_session_id" => endpoint.agent_session_id
               })

      if unquote(change) == :session do
        assert {:ok, replacement} =
                 AgentRegistry.register(Map.put(attrs, "agent_session_id", "replacement"))

        assert replacement.agent_session_id == "replacement"
      end

      reply_description(handler, f)
      assert_no_send(sweep)
      assert only_entry(f.id)["reason"] == "original_endpoint_not_configured"
    end
  end

  test "removed owner does not block another captured peer or reuse its target", ctx do
    peer = agent("durable-peer")
    configure([ctx.endpoint, peer])
    f = saved(ctx.endpoint, ["one", "two"], %{"two" => peer})
    sweep = kick(f.id)
    {handler, _} = receive_request("describe_agent")
    configure([peer])
    reply_description(handler, f)
    {handler, _} = receive_request("describe_agent", "durable-peer")
    reply_description(handler, f)
    {handler, request} = receive_request("cancel_execution", "durable-peer")
    assert request["params"] == exact(f, "two")
    acknowledge(handler, request)
    assert_no_send(sweep)
    [first, second] = entries(f.id)
    assert first["reason"] == "original_endpoint_not_configured"
    assert second["status"] == "requested"
  end

  test "restoring membership after restart retries only the already bound execution", ctx do
    f = saved(ctx.endpoint)
    sweep = kick(f.id)
    {handler, _} = receive_request("describe_agent")
    configure([])
    reply_description(handler, f)
    assert_no_send(sweep)
    assert only_entry(f.id)["execution_target"] == exact(f, "one")
    configure([ctx.endpoint])
    Fixture.restart_coordinator()
    {handler, request} = receive_request("cancel_execution")
    assert request["params"] == exact(f, "one")
    acknowledge(handler, request)
    wait_idle()
    entry = only_entry(f.id)
    assert entry["status"] == "requested"
    assert entry["attempts"] == 2
    refute_receive {:agent_request, _, _, _}, 30
  end

  test "directory outage is persisted and exhausts only the existing three-attempt budget", ctx do
    f = saved(ctx.endpoint)
    assert :ok = Outage.stop(AgentPool)

    for attempt <- 1..3 do
      assert_no_send(kick(f.id))
      entry = only_entry(f.id)
      assert entry["attempts"] == attempt
      assert entry["reason"] == "endpoint_directory_unavailable"
      assert entry["status"] == if(attempt == 3, do: "exhausted", else: "pending")
      assert is_nil(entry["execution_target"])
    end

    before = entries(f.id)
    assert_no_send(kick(f.id))
    assert entries(f.id) == before
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    assert Intent.summary(runtime[@key])["status"] == "unresolved"
    refute Intent.summary(runtime[@key])["execution_terminal_confirmed"]
  end

  for change <- [:replace_reservation, :damage_intent] do
    test "#{change} during final directory lookup cannot reuse earlier authorization", ctx do
      f = saved(ctx.endpoint)
      probe = {_pid, token} = Probe.install(AgentPool, &(&1 == :inspection_endpoints), 2)
      sweep = kick(f.id)
      {handler, _} = receive_request("describe_agent")
      reply_description(handler, f)
      assert_receive {:cancellation_paused, ^token, _}, 2_000
      invalidate(f, unquote(change))
      {:ok, retained} = AnalysisResultStore.get(f.id)
      Probe.resume(probe)
      assert_no_send(sweep)
      assert {:ok, ^retained} = AnalysisResultStore.get(f.id)
    end
  end

  @tag timeout: 30_000
  test "slow reservation cannot start a probe after the original sweep budget", ctx do
    f = saved(ctx.endpoint)
    probe = {_pid, token} = pause_store(f.id, :reserve)
    sweep = kick(f.id)
    assert_receive {:cancellation_paused, ^token, _}, 2_000
    Process.sleep(15_100)
    Probe.resume(probe)
    assert_no_send(sweep)
    entry = only_entry(f.id)
    assert entry["reason"] == "delivery_budget_exhausted"
    assert entry["status"] == "pending"
  end

  test "unchanged membership preserves one observation and exact per-request cancellation", ctx do
    f = saved(ctx.endpoint, ["one", "two"])
    sweep = kick(f.id)
    {handler, _} = receive_request("describe_agent")
    reply_description(handler, f)

    for id <- f.ids do
      {handler, request} = receive_request("cancel_execution")
      assert request["params"] == exact(f, id)
      acknowledge(handler, request)
    end

    assert_no_send(sweep)
    assert Enum.all?(entries(f.id), &(&1["status"] == "requested" and &1["attempts"] == 1))
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    assert Intent.summary(runtime[@key])["status"] == "acknowledged"
    refute Intent.summary(runtime[@key])["execution_terminal_confirmed"]
  end

  defp agent(name) do
    start_supervised!({CancellableSolverAgent, {self(), name}})
    assert_receive {:cancellable_agent_ready, ^name, port}
    %{id: name, host: "127.0.0.1", port: port, agent_session_id: "original-session"}
  end

  defp saved(endpoint, ids \\ ["one"], overrides \\ %{}) do
    f = Fixture.terminal_job(:failed)

    snapshot = %{
      requests:
        Enum.map(ids, fn id ->
          %{endpoint: Map.get(overrides, id, endpoint), request_id: id, job_id: f.id}
        end),
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

    {:ok, recovery} = Intent.attach(f.runtime[@key], f.job, snapshot)
    runtime = Map.put(f.runtime, @key, recovery)
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, f.runtime, runtime)
    Map.merge(f, %{ids: ids, runtime: runtime})
  end

  defp bind_saved(f) do
    token = reserve(f)
    assert :ok = Store.access(f.id, {:bind, token, exact(f, "one")})
    assert :ok = Store.access(f.id, {:finish, token, "cancellation_outcome_unknown"})
  end

  defp reserve(f) do
    assert {:ok, bucket} = Store.access(f.id, :load)
    identity = Intent.identity(hd(bucket["entries"]))
    assert {:ok, token} = Store.access(f.id, {:reserve, bucket["intent_id"], identity})
    token
  end

  defp kick(id) do
    assert {:ok, pid} = Delivery.kick([id])
    ref = Process.monitor(pid)
    on_exit(fn -> if Process.alive?(pid), do: Process.exit(pid, :kill) end)
    {pid, ref}
  end

  defp assert_no_send({pid, ref}) do
    refute_receive {:agent_request, _, _, _}, 30
    assert_receive {:DOWN, ^ref, :process, ^pid, :normal}, 2_000
    refute_receive {:agent_request, _, _, _}, 10
  end

  defp wait_idle,
    do: Outage.eventually(fn -> Registry.lookup(@registry, {Delivery, :sweep}) == [] end)

  defp entries(id) do
    assert {:ok, bucket} = Store.access(id, :load)
    bucket["entries"]
  end

  defp only_entry(id) do
    assert [entry] = entries(id)
    entry
  end

  defp configure(endpoints), do: :sys.replace_state(AgentPool, &%{&1 | endpoints: endpoints})
  defp withdraw(:removed), do: configure([])
  defp withdraw(:unavailable), do: assert(:ok = Outage.stop(AgentPool))
  defp directory_reason(:removed), do: "original_endpoint_not_configured"
  defp directory_reason(:unavailable), do: "endpoint_directory_unavailable"
  defp changed(_, :removed), do: []
  defp changed(endpoint, :session), do: [%{endpoint | agent_session_id: "replacement"}]
  defp changed(endpoint, :host), do: [%{endpoint | host: "127.0.0.2"}]
  defp changed(endpoint, :port), do: [%{endpoint | port: 1}]
  defp changed(endpoint, :id), do: [%{endpoint | id: "replacement"}]

  defp receive_request(method, name \\ "durable-directory") do
    assert_receive {:agent_request, ^name, handler, %{"method" => ^method} = request}, 2_000
    {handler, request}
  end

  defp reply_description(handler, f) do
    active =
      Enum.map(f.ids ++ ["same-job-bystander"], fn id ->
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

  defp exact(f, id),
    do: %{
      "job_id" => f.id,
      "request_id" => id,
      "process_instance_id" => "original-process",
      "generation" => 7
    }

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

  defp assert_outcome(entry, :lost_ack) do
    assert entry["status"] == "pending"
    assert entry["reason"] == "cancellation_outcome_unknown"
  end

  defp assert_outcome(entry, outcome) do
    assert entry["status"] ==
             if(outcome == :requested, do: "requested", else: "target_not_observed")

    assert is_nil(entry["reason"])
  end

  defp pause_store(id, phase) do
    Probe.install(Coordinator, fn
      {:cancellation_intent, ^id, action} when is_tuple(action) -> elem(action, 0) == phase
      _ -> false
    end)
  end

  defp invalidate(f, :replace_reservation), do: reserve(f)

  defp invalidate(f, :damage_intent) do
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    changed = put_in(runtime, [@key, Intent.key(), "sha256"], String.duplicate("0", 64))
    assert :ok = AnalysisResultStore.compare_and_swap(f.id, runtime, changed)
  end
end
