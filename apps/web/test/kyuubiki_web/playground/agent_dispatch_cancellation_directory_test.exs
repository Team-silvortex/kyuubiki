defmodule KyuubikiWeb.Playground.AgentDispatchCancellationDirectoryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.{AgentDispatchCancellation, AgentPool, AgentRegistry}
  alias KyuubikiWeb.TestSupport.CancellableSolverAgent

  @endpoint %{
    id: "directory-owner",
    host: "127.0.0.1",
    port: 5099,
    agent_session_id: "original-session",
    control_mode: "standalone"
  }
  @job "directory-job"
  @supervisor KyuubikiWeb.Supervisor

  setup do
    config = Application.get_env(:kyuubiki_web, AgentPool, [])
    Application.put_env(:kyuubiki_web, AgentPool, discovery: :static, endpoints: [@endpoint])
    :ok = AgentPool.reload()

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, config)

      if is_nil(Process.whereis(AgentPool)),
        do: assert({:ok, _} = Supervisor.restart_child(@supervisor, AgentPool))

      :sys.resume(AgentPool)
      :ok = AgentPool.reload()
    end)

    :ok
  end

  for change <- [:removed, :session, :host, :port, :id] do
    test "#{change} during observation blocks cancellation without retargeting" do
      receipt =
        cancel(["one"],
          probe: fn _, _ ->
            configure(changed(unquote(change)))
            descriptor(["one"])
          end,
          send_cancel: never_send()
        )

      assert [target] = receipt["targets"]
      assert_not_sent(target, "original_endpoint_not_configured")
      assert receipt["registered_count"] == 0
    end

    test "#{change} after first send blocks later cached targets but retains its receipt" do
      receipt =
        cancel(["one", "two"],
          probe: fn _, _ ->
            send(self(), :probe)
            descriptor(["one", "two", "bystander"])
          end,
          send_cancel: fn _, target, _ ->
            assert target["request_id"] == "one"
            configure(changed(unquote(change)))
            {:ok, ack(target)}
          end
        )

      assert [first, second] = receipt["targets"]
      assert first["cancel_registered"]
      assert first["delivery_attempted"]
      assert_not_sent(second, "original_endpoint_not_configured")
      assert receipt["registered_count"] == 1
      assert receipt["status"] == "partially_requested"
      assert_receive :probe
      refute_receive :probe, 10
    end
  end

  test "current endpoint metadata is used without changing the captured identity" do
    current = Map.put(@endpoint, :control_mode, "mesh")
    configure([current])

    receipt =
      cancel(["one"],
        probe: fn endpoint, _ ->
          assert endpoint == current
          configure([Map.put(current, :role, "worker")])
          descriptor(["one"])
        end,
        send_cancel: fn endpoint, target, _ ->
          assert endpoint == Map.put(current, :role, "worker")
          assert target["process_instance_id"] == "original-process"
          assert target["generation"] === 1
          assert target["job_id"] == @job
          {:ok, ack(target)}
        end
      )

    assert receipt["registered_count"] == 1
  end

  test "unchanged directory shares one probe across exact captured requests" do
    receipt =
      cancel(["one", "two"],
        probe: fn _, _ ->
          send(self(), :probe)
          descriptor(["one", "two", "bystander"])
        end,
        send_cancel: fn _, target, _ ->
          assert target["request_id"] in ["one", "two"]
          {:ok, ack(target)}
        end
      )

    assert receipt["registered_count"] == 2
    assert_receive :probe
    refute_receive :probe, 10
  end

  test "removing one owner does not redirect its request or block a captured peer" do
    peer = %{@endpoint | id: "peer", port: 5100}
    configure([@endpoint, peer])
    targets = snapshot(["one", "two"])
    targets = put_in(targets.requests, [hd(targets.requests), request("two", peer)])

    assert {:ok, receipt} =
             AgentDispatchCancellation.cancel(@job, targets,
               probe: fn endpoint, _ ->
                 if endpoint.id == @endpoint.id, do: configure([peer])
                 descriptor(["one", "two"])
               end,
               send_cancel: fn endpoint, target, _ ->
                 assert endpoint.id == "peer"
                 assert target["request_id"] == "two"
                 {:ok, ack(target)}
               end
             )

    assert [first, second] = receipt["targets"]
    assert_not_sent(first, "original_endpoint_not_configured")
    assert second["cancel_registered"]
    assert receipt["status"] == "partially_requested"
  end

  for after_send <- [false, true] do
    test "directory outage after #{if after_send, do: "send", else: "probe"} fails closed per target" do
      receipt =
        cancel(["one", "two"],
          probe: fn _, _ ->
            unless unquote(after_send), do: stop_directory()
            descriptor(["one", "two"])
          end,
          send_cancel: fn _, target, _ ->
            assert unquote(after_send)
            assert target["request_id"] == "one"
            stop_directory()
            {:ok, ack(target)}
          end
        )

      assert [first, second] = receipt["targets"]

      if unquote(after_send) do
        assert first["cancel_registered"]
        assert receipt["registered_count"] == 1
      else
        assert_not_sent(first, "endpoint_directory_unavailable")
        assert receipt["registered_count"] == 0
      end

      assert_not_sent(second, "endpoint_directory_unavailable")
    end
  end

  test "directory loss cannot erase a sent request whose acknowledgement was lost" do
    receipt =
      cancel(["one", "two"],
        probe: fn _, _ -> descriptor(["one", "two"]) end,
        send_cancel: fn _, target, _ ->
          assert target["request_id"] == "one"
          stop_directory()
          {:error, :closed}
        end
      )

    assert [first, second] = receipt["targets"]
    assert first["status"] == "cancellation_outcome_unknown"
    assert first["delivery_attempted"]
    assert is_nil(first["cancel_registered"])
    assert_not_sent(second, "endpoint_directory_unavailable")
    assert receipt["status"] == "cancellation_outcome_unknown"
  end

  test "queued-only cleanup does not require an available endpoint directory" do
    stop_directory()
    targets = %{snapshot([]) | queued_cancelled_count: 2}

    assert {:ok, receipt} = AgentDispatchCancellation.cancel(@job, targets)
    assert receipt["status"] == "cancelled_before_dispatch"
    assert receipt["targets"] == []
  end

  test "unavailable directory before observation never falls back to the captured endpoint" do
    stop_directory()

    receipt =
      cancel(["one"],
        probe: fn _, _ -> flunk("unavailable directory must not authorize a probe") end,
        send_cancel: never_send()
      )

    assert [target] = receipt["targets"]
    assert_not_sent(target, "endpoint_directory_unavailable")
  end

  for change <- [:removed, :session, :unchanged, :rejected_replacement] do
    test "registry #{change} during probe uses live membership, never the static fallback" do
      attrs = %{
        "id" => @endpoint.id,
        "host" => @endpoint.host,
        "port" => @endpoint.port,
        "agent_session_id" => @endpoint.agent_session_id
      }

      assert {:ok, _} = AgentRegistry.register(attrs)
      on_exit(fn -> AgentRegistry.unregister(@endpoint.id) end)
      Application.put_env(:kyuubiki_web, AgentPool, discovery: :registry, endpoints: [@endpoint])
      :ok = AgentPool.reload()

      receipt =
        cancel(["one"],
          probe: fn _, _ ->
            change_registry(unquote(change), attrs)
            descriptor(["one"])
          end,
          send_cancel: fn _, target, _ ->
            send(self(), :sent)
            {:ok, ack(target)}
          end
        )

      if unquote(change) in [:unchanged, :rejected_replacement] do
        assert receipt["registered_count"] == 1
        assert_receive :sent
      else
        assert [target] = receipt["targets"]
        assert_not_sent(target, "original_endpoint_not_configured")
        refute_receive :sent, 10
      end
    end
  end

  test "directory wait is bounded by the remaining delivery budget" do
    owner = self()
    clock = :atomics.new(1, [])
    pool = Process.whereis(AgentPool)

    try do
      task =
        Task.async(fn ->
          cancel(["one"],
            clock: fn -> :atomics.get(clock, 1) end,
            probe: fn _, _ ->
              :atomics.put(clock, 1, 14_950)
              :ok = :sys.suspend(pool)
              send(owner, :probe)
              descriptor(["one"])
            end,
            send_cancel: fn _, target, _ ->
              send(owner, :unexpected_cancel)
              {:ok, ack(target)}
            end
          )
        end)

      assert_receive :probe, 1_000
      receipt = Task.await(task, 1_000)
      assert [target] = receipt["targets"]
      assert_not_sent(target, "endpoint_directory_unavailable")
      refute_receive :unexpected_cancel, 10
    after
      :sys.resume(pool)
    end
  end

  for stage <- [:initial_lookup, :post_probe_lookup] do
    test "budget includes time awaiting #{stage} and is never reset by directory lookup" do
      clock = :atomics.new(1, [])
      owner = self()
      pool = Process.whereis(AgentPool)
      if unquote(stage) == :initial_lookup, do: :sys.suspend(pool)

      try do
        task =
          Task.async(fn ->
            cancel(["one"],
              clock: fn -> :atomics.get(clock, 1) end,
              probe: budget_probe(unquote(stage), pool, owner),
              send_cancel: fn _, target, _ ->
                send(owner, :unexpected_cancel)
                {:ok, ack(target)}
              end
            )
          end)

        if unquote(stage) == :post_probe_lookup, do: assert_receive(:probe, 1_000)
        await_lookup(pool, task.pid)
        :atomics.put(clock, 1, 15_000)
        :ok = :sys.resume(pool)
        receipt = Task.await(task, 2_000)
        assert [target] = receipt["targets"]
        assert_not_sent(target, "delivery_budget_exhausted")
        refute_receive :unexpected_cancel, 10
        refute_receive :unexpected_probe, 10
      after
        :sys.resume(pool)
      end
    end
  end

  for change <- [:removed, :session] do
    test "real TCP observation cannot authorize a cancel after #{change}" do
      name = "directory-wire"
      start_supervised!({CancellableSolverAgent, {self(), name}})
      assert_receive {:cancellable_agent_ready, ^name, port}
      endpoint = %{@endpoint | port: port}
      configure([endpoint])
      targets = %{snapshot([]) | requests: [request("one", endpoint)]}
      task = Task.async(fn -> AgentDispatchCancellation.cancel(@job, targets) end)
      assert_receive {:agent_request, ^name, handler, %{"method" => "describe_agent"}}, 2_000

      configure(
        if(unquote(change) == :removed, do: [], else: [%{endpoint | agent_session_id: "new"}])
      )

      {:ok, description} = descriptor(["one"])
      send(handler, {:reply, %{"ok" => true, "result" => description}})
      refute_receive {:agent_request, ^name, _, _}, 100
      assert {:ok, receipt} = Task.await(task, 2_000)
      assert [target] = receipt["targets"]
      assert_not_sent(target, "original_endpoint_not_configured")
    end
  end

  defp configure(endpoints), do: :sys.replace_state(AgentPool, &%{&1 | endpoints: endpoints})
  defp stop_directory, do: assert(:ok = Supervisor.terminate_child(@supervisor, AgentPool))
  defp changed(:removed), do: []
  defp changed(:session), do: [%{@endpoint | agent_session_id: "replacement-session"}]
  defp changed(:host), do: [%{@endpoint | host: "127.0.0.2"}]
  defp changed(:port), do: [%{@endpoint | port: 5100}]
  defp changed(:id), do: [%{@endpoint | id: "replacement-owner"}]
  defp never_send, do: fn _, _, _ -> flunk("must not send cancellation") end

  defp change_registry(:unchanged, _), do: :ok

  defp change_registry(:removed, attrs),
    do: assert(:ok = AgentRegistry.unregister(attrs["id"]))

  defp change_registry(:session, attrs) do
    assert :ok = AgentRegistry.unregister(attrs["id"], Map.take(attrs, ["agent_session_id"]))

    assert {:ok, replacement} =
             AgentRegistry.register(Map.put(attrs, "agent_session_id", "replacement-session"))

    assert replacement.agent_session_id == "replacement-session"
  end

  defp change_registry(:rejected_replacement, attrs) do
    assert {:error, {:agent_session_conflict, _}} =
             AgentRegistry.register(Map.put(attrs, "agent_session_id", "replacement-session"))
  end

  defp cancel(ids, opts) do
    assert {:ok, receipt} = AgentDispatchCancellation.cancel(@job, snapshot(ids), opts)
    refute receipt["execution_terminal_confirmed"]
    refute receipt["automatic_replay_authorized"]
    refute receipt["pending_cancellation_created"]
    refute receipt["job_wide_fallback_performed"]
    receipt
  end

  defp assert_not_sent(target, status) do
    assert target["status"] == status
    assert target["delivery_attempted"] === false
    assert target["cancel_registered"] === false
    assert is_nil(target["agent_acknowledgement"])
  end

  defp await_lookup(pool, caller, attempts \\ 200)
  defp await_lookup(_, _, 0), do: flunk("cancellation never rechecked the endpoint directory")

  defp await_lookup(pool, caller, attempts) do
    {:messages, messages} = Process.info(pool, :messages)

    if Enum.any?(messages, &match?({:"$gen_call", {^caller, _}, :inspection_endpoints}, &1)) do
      :ok
    else
      Process.sleep(5)
      await_lookup(pool, caller, attempts - 1)
    end
  end

  defp budget_probe(stage, pool, owner) do
    fn _, _ ->
      if stage == :post_probe_lookup do
        :ok = :sys.suspend(pool)
        send(owner, :probe)
      else
        send(owner, :unexpected_probe)
      end

      descriptor(["one"])
    end
  end

  defp request(id, endpoint \\ @endpoint),
    do: %{endpoint: endpoint, request_id: id, job_id: @job}

  defp snapshot(ids),
    do: %{
      requests: Enum.map(ids, &request/1),
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

  defp descriptor(ids) do
    active =
      Enum.map(ids, fn id ->
        %{"request_id" => id, "job_id" => @job, "generation" => 1, "cancel_requested" => false}
      end)

    {:ok,
     %{
       "lifecycle" => %{"process_instance_id" => "original-process"},
       "solver_control" => %{
         "schema_version" => "kyuubiki.agent-solver-control/v1",
         "available" => true,
         "active" => active
       }
     }}
  end

  defp ack(target),
    do: %{
      "schema_version" => "kyuubiki.agent-execution-cancellation/v1",
      "execution_target" => target,
      "status" => "requested",
      "cancel_registered" => true,
      "execution_terminal_confirmed" => false,
      "pending_cancellation_created" => false,
      "operator_package_cleanup_performed" => false,
      "automatic_replay_authorized" => false
    }
end
