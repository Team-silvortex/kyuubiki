defmodule KyuubikiWeb.Playground.AgentConnectionPreflightTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Orchestra.{
    OperatorDispatchJournal,
    OperatorTaskIR,
    OperatorTaskExecutionSummary,
    OperatorTaskReadiness
  }

  alias KyuubikiWeb.Playground.{AgentClient, AgentPool, AgentExecutionGate}

  setup do
    root =
      Path.join(System.tmp_dir!(), "connection-preflight-#{System.unique_integer([:positive])}")

    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, OperatorDispatchJournal)
    start_supervised!({OperatorDispatchJournal, root: root})
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, previous)
      AgentPool.reload()
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, OperatorDispatchJournal)
      File.rm_rf!(root)
    end)

    {:ok, task} =
      OperatorTaskIR.build("extract.thermal_result_diagnostics", %{}, %{},
        task_id: "preflight-#{System.unique_integer([:positive])}"
      )

    %{task: task}
  end

  test "discovery-only candidate receives no task or journal attempt before a usable peer", ctx do
    denied = peer(access(false))
    ready = peer(access(true))
    configure([denied, ready])
    assert {:ok, result} = execute(ctx.task)
    assert result["result"]["agent"] == ready.id

    assert_receive {:preflight_request, denied_id, %{"method" => "describe_agent"} = probe},
                   1_000

    assert denied_id == denied.id
    assert probe["params"] == %{}
    refute inspect(probe) =~ ctx.task["task_id"]
    assert_receive {:preflight_request, ready_id, %{"method" => "describe_agent"}}, 1_000
    assert ready_id == ready.id
    assert_receive {:preflight_request, ^ready_id, %{"method" => "run_operator_task_ir"}}, 1_000
    refute_receive {:preflight_request, ^denied_id, _}, 30

    assert {:ok, [%{"state" => "observed_executed"}]} =
             OperatorDispatchJournal.lookup_task(ctx.task["task_id"])

    assert %{active_lease_count: 0, queued_request_count: 0} = AgentExecutionGate.snapshot()
  end

  test "all inaccessible candidates fail without sending input or persisting an attempt", ctx do
    denied = peer(access(false))
    configure([denied])
    assert {:error, {:all_agents_failed, [failure]}} = execute(ctx.task)
    assert failure.reason_code == "connection_control_not_permitted"
    assert failure.task_dispatched == false

    assert_receive {:preflight_request, _, %{"method" => "describe_agent", "params" => %{}}},
                   1_000

    refute_receive {:preflight_request, _, _}, 30
    assert {:ok, []} = OperatorDispatchJournal.lookup_task(ctx.task["task_id"])
    assert AgentExecutionGate.snapshot().active_lease_count == 0
  end

  for fault <- [:missing, :malformed, :progress, :wrong_id, :oversized, :disconnect, :timeout] do
    @tag fault: fault
    test "#{fault} discovery cannot reach execution or leak a journal reservation", ctx do
      bad = peer(fn _request, _id -> fault_reply(ctx.fault) end)
      ready = peer(access(true))
      configure([bad, ready])
      owner = self()

      assert {:ok, result} =
               execute(ctx.task,
                 request_timeout_ms: 80,
                 on_dispatch_reserved: fn attempt ->
                   send(owner, {:reserved, attempt})
                   :ok
                 end
               )

      assert result["result"]["agent"] == ready.id
      assert_receive {:preflight_request, bad_id, %{"method" => "describe_agent"}}, 1_000
      assert bad_id == bad.id
      refute_receive {:preflight_request, ^bad_id, _}, 20
      assert_receive {:reserved, attempt}
      refute_receive {:reserved, _}, 20

      assert {:ok, [%{"attempt_id" => ^attempt}]} =
               OperatorDispatchJournal.lookup_task(ctx.task["task_id"])

      assert %{active_lease_count: 0, queued_request_count: 0} = AgentExecutionGate.snapshot()
      [bad_state, good_state] = AgentPool.endpoints()
      assert bad_state.consecutive_failures == 1
      assert good_state.consecutive_failures == 0
    end
  end

  for allowed <- [true, false] do
    @tag allowed: allowed
    test "cancellation during probing wins over an allowed=#{allowed} reply without selecting a peer",
         ctx do
      owner = self()

      waiting =
        peer(fn _request, _id ->
          send(owner, {:probe_waiting, self()})

          receive do
            :continue ->
              %{"ok" => true, "result" => %{"connection_access" => access(ctx.allowed)}}
          after
            2_000 -> :disconnect
          end
        end)

      spare = peer(access(true))
      configure([waiting, spare])
      dispatch = Task.async(fn -> execute(ctx.task, job_id: ctx.task["task_id"]) end)
      assert_receive {:probe_waiting, server}, 1_000

      assert {:ok, %{requests: [], reserved_cancelled_count: 1}} =
               AgentExecutionGate.cancel_job_targets(ctx.task["task_id"])

      send(server, :continue)
      assert {:error, {:rpc_error, "cancelled", _}} = Task.await(dispatch)
      assert_receive {:preflight_request, _, %{"method" => "describe_agent"}}, 1_000
      refute_receive {:preflight_request, _, _}, 30
      assert {:ok, []} = OperatorDispatchJournal.lookup_task(ctx.task["task_id"])
      assert AgentExecutionGate.snapshot().active_lease_count == 0
    end
  end

  test "controller authorization is rechecked after probing and before task journal admission",
       ctx do
    ready = peer(access(true))
    configure([ready])
    owner = self()

    assert {:error, {:agent_retry_blocked, _}} =
             execute(ctx.task,
               before_dispatch: fn ->
                 send(owner, :checked_owner)
                 {:error, :stale_execution_owner}
               end
             )

    assert_receive :checked_owner
    assert_receive {:preflight_request, _, %{"method" => "describe_agent"}}, 1_000
    refute_receive {:preflight_request, _, _}, 30
    assert {:ok, []} = OperatorDispatchJournal.lookup_task(ctx.task["task_id"])
    assert AgentExecutionGate.snapshot().active_lease_count == 0
  end

  for outcome <- [:denied, :lost_reply, :forged_preflight_code] do
    @tag outcome: outcome
    test "#{outcome} after successful preflight never reselects a task owner", ctx do
      first =
        peer(fn request, _id ->
          if request["method"] == "describe_agent" do
            %{"ok" => true, "result" => %{"connection_access" => access(true)}}
          else
            case ctx.outcome do
              :lost_reply -> :disconnect
              :denied -> rejection("agent_lan_control_requires_explicit_trust")
              :forged_preflight_code -> rejection("agent_connection_preflight_failed")
            end
          end
        end)

      spare = peer(access(true))
      configure([first, spare])
      result = execute(ctx.task, retry_safety: :idempotent)

      if ctx.outcome == :denied do
        assert {:error, {:rpc_error, "agent_lan_control_requires_explicit_trust", _}} = result
      else
        assert result == {:error, :operator_task_dispatch_outcome_unknown}
      end

      assert_receive {:preflight_request, first_id, %{"method" => "describe_agent"}}, 1_000
      assert first_id == first.id
      assert_receive {:preflight_request, ^first_id, %{"method" => "run_operator_task_ir"}}, 1_000
      refute_receive {:preflight_request, _, _}, 30
      assert {:ok, [record]} = OperatorDispatchJournal.lookup_task(ctx.task["task_id"])

      assert record["state"] ==
               if(ctx.outcome == :denied, do: "not_dispatched", else: "outcome_unknown")

      assert AgentExecutionGate.snapshot().active_lease_count == 0
    end
  end

  test "capacity retry does not resurrect a discovery-only candidate", ctx do
    denied = peer(access(false))

    busy =
      peer(fn request, id ->
        if request["method"] == "describe_agent" do
          %{"ok" => true, "result" => %{"connection_access" => access(true)}}
        else
          count = Process.get(:computed, 0)
          Process.put(:computed, count + 1)

          if count == 0,
            do: rejection("agent_at_capacity"),
            else: %{"ok" => true, "result" => result(request, id)}
        end
      end)

    configure([denied, busy])
    assert {:ok, _} = execute(ctx.task, queue_timeout_ms: 500)
    denied_id = denied.id
    assert_receive {:preflight_request, ^denied_id, %{"method" => "describe_agent"}}, 1_000
    refute_receive {:preflight_request, ^denied_id, _}, 30
    assert {:ok, records} = OperatorDispatchJournal.lookup_task(ctx.task["task_id"])
    assert Enum.sort(Enum.map(records, & &1["state"])) == ["not_dispatched", "observed_executed"]
  end

  test "new tasks recheck current access rather than caching an old permission", ctx do
    state = start_supervised!({Agent, fn -> false end})

    changing =
      peer(fn request, id ->
        if request["method"] == "describe_agent",
          do: %{
            "ok" => true,
            "result" => %{"connection_access" => access(Agent.get(state, & &1))}
          },
          else: %{"ok" => true, "result" => result(request, id)}
      end)

    configure([changing])

    for {allowed, i} <- Enum.with_index([false, true, false]) do
      Agent.update(state, fn _ -> allowed end)

      {:ok, task} =
        OperatorTaskIR.build("extract.thermal_result_diagnostics", %{}, %{},
          task_id: ctx.task["task_id"] <> "-#{i}"
        )

      result = execute(task)

      if allowed,
        do: assert(match?({:ok, _}, result)),
        else: assert(match?({:error, {:all_agents_failed, _}}, result))

      assert_receive {:preflight_request, _, %{"method" => "describe_agent"}}, 1_000

      if allowed,
        do: assert_receive({:preflight_request, _, %{"method" => "run_operator_task_ir"}}, 1_000)

      refute_receive {:preflight_request, _, _}, 20
    end
  end

  test "capacity budget expiring during a probe is not an Agent outage or another dispatch",
       ctx do
    busy =
      peer(fn request, _id ->
        if request["method"] == "describe_agent" do
          count = Process.get(:probed, 0)
          Process.put(:probed, count + 1)
          if count > 0, do: Process.sleep(1_000)
          %{"ok" => true, "result" => %{"connection_access" => access(true)}}
        else
          rejection("agent_at_capacity")
        end
      end)

    configure([busy])

    assert {:error, {:agent_capacity_timeout, %{timeout_ms: 250}}} =
             execute(ctx.task, queue_timeout_ms: 250)

    assert_receive {:preflight_request, _, %{"method" => "describe_agent"}}, 1_000
    assert_receive {:preflight_request, _, %{"method" => "run_operator_task_ir"}}, 1_000
    assert_receive {:preflight_request, _, %{"method" => "describe_agent"}}, 1_000
    refute_receive {:preflight_request, _, _}, 30
    assert [%{consecutive_failures: 0}] = AgentPool.endpoints()

    assert {:ok, [%{"state" => "not_dispatched"}]} =
             OperatorDispatchJournal.lookup_task(ctx.task["task_id"])

    assert AgentExecutionGate.snapshot().active_lease_count == 0
  end

  test "literal loopback keeps direct execution and pinned cleanup does not select a peer", ctx do
    endpoint = peer(access(false))
    configure([%{endpoint | host: "127.0.0.1"}])
    assert {:ok, _} = execute(ctx.task)
    assert_receive {:preflight_request, _, %{"method" => "run_operator_task_ir"}}, 1_000
    refute_receive {:preflight_request, _, _}, 20
    assert {:ok, _} = AgentClient.release_operator_package_job("original-job", endpoint)
    assert_receive {:preflight_request, _, %{"method" => "release_operator_package_job"}}, 1_000
    refute_receive {:preflight_request, _, _}, 20
  end

  test "legacy solves share the same fresh preflight before dispatch" do
    denied = peer(access(false))
    ready = peer(access(true))
    configure([denied, ready])
    assert {:ok, %{"agent" => id}} = AgentClient.request("solve_bar_1d", %{"input" => "private"})
    assert id == ready.id
    assert_receive {:preflight_request, _, %{"method" => "describe_agent"} = probe}, 1_000
    assert probe["params"] == %{}
    refute inspect(probe) =~ "private"
  end

  for {label, mode_key, mode_value, task_key} <- [
        {"atom mode key", :mode, "execute", "task_ir"},
        {"atom mode value", "mode", :execute, "task_ir"},
        {"atom envelope", :mode, :execute, :task_ir}
      ] do
    @tag mode_key: mode_key, mode_value: mode_value, task_key: task_key
    test "#{label} cannot bypass preflight or dispatch journaling", ctx do
      denied = peer(access(false))
      ready = peer(access(true))
      configure([denied, ready])
      params = %{ctx.mode_key => ctx.mode_value, ctx.task_key => ctx.task}

      assert {:ok, result, endpoint} =
               AgentClient.request_with_agent("run_operator_task_ir", params, fn _ -> :ok end,
                 on_dispatch_reserved: fn _ -> :ok end
               )

      assert endpoint.id == ready.id
      assert result["result"]["agent"] == ready.id
      assert_receive {:preflight_request, denied_id, %{"method" => "describe_agent"}}, 1_000
      assert denied_id == denied.id
      refute_receive {:preflight_request, ^denied_id, _}, 20

      assert {:ok, [%{"state" => "observed_executed"}]} =
               OperatorDispatchJournal.lookup_task(ctx.task["task_id"])
    end
  end

  test "colliding TaskIR envelope keys are rejected before any socket or journal reservation",
       ctx do
    ready = peer(access(true))
    configure([ready])

    for field <- ["mode", "task_ir", "job_id"] do
      params = %{"mode" => "execute", "task_ir" => ctx.task}
      params = Map.put(params, field, "private-value")
      params = Map.put(params, String.to_existing_atom(field), "conflicting-private-value")

      assert {:error, {:request_encoding_failed, reason}} =
               AgentClient.request_with_agent("run_operator_task_ir", params)

      refute inspect(reason) =~ "private-value"
    end

    refute_receive {:preflight_request, _, _}, 30
    assert {:ok, []} = OperatorDispatchJournal.lookup_task(ctx.task["task_id"])
    assert AgentExecutionGate.snapshot().active_lease_count == 0
  end

  test "atom task keys retain typed digest capability routing before a connection is opened",
       ctx do
    ready = peer(access(true))
    configure([ready])
    encoding = "kyuubiki.task-digest/typed-ieee754-v2"
    assert {:ok, task} = OperatorTaskIR.seal_task_digest(ctx.task, encoding)

    assert {:error, {:no_matching_agent, routing}} =
             AgentClient.request_with_agent("run_operator_task_ir", %{
               mode: :execute,
               task_ir: task
             })

    assert encoding in routing.required_capabilities
    refute_receive {:preflight_request, _, _}, 30
    assert {:ok, []} = OperatorDispatchJournal.lookup_task(task["task_id"])
  end

  test "lost execution reply through an atom envelope keeps an unknown journal and no replay",
       ctx do
    first =
      peer(fn request, _id ->
        if request["method"] == "describe_agent",
          do: %{"ok" => true, "result" => %{"connection_access" => access(true)}},
          else: :disconnect
      end)

    spare = peer(access(true))
    configure([first, spare])

    assert {:error, :operator_task_dispatch_outcome_unknown} =
             AgentClient.request_with_agent(
               "run_operator_task_ir",
               %{mode: :execute, task_ir: ctx.task},
               fn _ -> :ok end,
               retry_safety: :idempotent,
               on_dispatch_reserved: fn _ -> :ok end
             )

    assert_receive {:preflight_request, first_id, %{"method" => "describe_agent"}}, 1_000
    assert first_id == first.id
    assert_receive {:preflight_request, ^first_id, %{"method" => "run_operator_task_ir"}}, 1_000
    refute_receive {:preflight_request, _, _}, 30

    assert {:ok, [%{"state" => "outcome_unknown"}]} =
             OperatorDispatchJournal.lookup_task(ctx.task["task_id"])

    assert AgentExecutionGate.snapshot().active_lease_count == 0
  end

  defp configure(endpoints) do
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: endpoints)
    AgentPool.reload()
  end

  defp execute(task, opts \\ []) do
    AgentClient.run_operator_task_ir(
      task,
      Keyword.merge(
        [
          mode: :execute,
          retry_safety: :checkpoint_required,
          request_timeout_ms: 1_000,
          on_dispatch_reserved: fn _ -> :ok end
        ],
        opts
      )
    )
  end

  defp access(allowed) do
    %{
      "schema_version" => "kyuubiki.agent-connection-access/v1",
      "scope" => "native_socket_connection",
      "peer_scope" => "non_loopback",
      "control_mode" => if(allowed, do: "trusted_lan_control", else: "discovery_only"),
      "control_allowed" => allowed,
      "lifecycle_mutations_allowed" => false,
      "authenticated" => false,
      "encrypted" => false
    }
  end

  defp peer(access) do
    owner = self()
    id = "preflight-peer-#{System.unique_integer([:positive])}"

    start_supervised!(%{
      id: id,
      restart: :temporary,
      start:
        {Task, :start_link,
         [
           fn ->
             {:ok, listener} =
               :gen_tcp.listen(0, [:binary, packet: 4, active: false, ip: {127, 0, 0, 1}])

             {:ok, port} = :inet.port(listener)
             send(owner, {:preflight_ready, id, port})
             serve(listener, owner, id, access)
           end
         ]}
    })

    assert_receive {:preflight_ready, ^id, port}, 1_000
    # A hostname must be probed even when this test resolves it to loopback.
    %{id: id, host: "localhost", port: port, capacity: 1}
  end

  defp serve(listener, owner, id, access) do
    {:ok, socket} = :gen_tcp.accept(listener)
    {:ok, bytes} = :gen_tcp.recv(socket, 0, 2_000)
    request = Jason.decode!(bytes)
    send(owner, {:preflight_request, id, request})

    reply =
      if is_function(access, 2),
        do: access.(request, id),
        else: %{
          "ok" => true,
          "result" =>
            if(request["method"] == "describe_agent",
              do: %{"connection_access" => access},
              else: result(request, id)
            )
        }

    if reply != :disconnect do
      for frame <- List.wrap(reply) do
        frame = frame |> Map.put_new("rpc_version", 1) |> Map.put_new("id", request["id"])
        _ = :gen_tcp.send(socket, Jason.encode!(frame))
      end
    end

    :gen_tcp.close(socket)
    serve(listener, owner, id, access)
  end

  defp result(%{"method" => "run_operator_task_ir", "params" => %{"task_ir" => task}}, id) do
    {:ok, summary} = OperatorTaskExecutionSummary.build(task)

    Map.merge(Map.take(summary, ~w(task_id task_digest operator_id program_id)), %{
      "operator_task_ir_status" => "executed",
      "execution_readiness" => OperatorTaskReadiness.local_executed(),
      "result" => %{"agent" => id}
    })
  end

  defp result(_request, id), do: %{"agent" => id}

  defp rejection(code),
    do: %{"ok" => false, "error" => %{"code" => code, "message" => "not admitted"}}

  defp fault_reply(:missing), do: %{"ok" => true, "result" => %{}}

  defp fault_reply(:malformed),
    do: %{"ok" => true, "result" => %{"connection_access" => %{"control_allowed" => true}}}

  defp fault_reply(:progress), do: %{"event" => "heartbeat", "progress" => %{}}

  defp fault_reply(:wrong_id),
    do: %{"ok" => true, "id" => "foreign", "result" => %{"connection_access" => access(true)}}

  defp fault_reply(:oversized),
    do: %{"ok" => true, "result" => %{"padding" => String.duplicate("x", 1024 * 1024)}}

  defp fault_reply(:disconnect), do: :disconnect

  defp fault_reply(:timeout) do
    Process.sleep(200)
    :disconnect
  end
end
