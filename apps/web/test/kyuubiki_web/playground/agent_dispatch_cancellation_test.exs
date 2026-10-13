defmodule KyuubikiWeb.Playground.AgentDispatchCancellationTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.{
    AgentClient,
    AgentDispatchCancellation,
    AgentExecutionGate,
    AgentPool
  }

  alias KyuubikiWeb.TestSupport.ScopedCancellationAgent

  @endpoint %{
    id: "scoped-owner",
    host: "127.0.0.1",
    port: 5099,
    agent_session_id: "original-session",
    token: "SECRET"
  }
  @job "scoped-job"

  test "captured request remains exact when a later dispatch reuses the same job id" do
    config = Application.get_env(:kyuubiki_web, AgentPool, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, config)
      AgentPool.reload()
    end)

    start_supervised!({ScopedCancellationAgent, {self(), "scoped-owner"}})
    assert_receive {:cancellable_agent_ready, "scoped-owner", port}

    endpoint =
      Map.merge(@endpoint, %{port: port, capacity: 2, methods: ["solve_bar_1d"], token: nil})

    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [endpoint])
    AgentPool.reload()

    original =
      Task.async(fn ->
        AgentClient.request("solve_bar_1d", %{}, fn _ -> :ok end, job_id: @job)
      end)

    assert_receive {:agent_request, "scoped-owner", first, request}, 2_000
    assert {:ok, captured} = AgentExecutionGate.cancel_job_targets(@job)

    later =
      Task.async(fn ->
        AgentClient.request("solve_bar_1d", %{}, fn _ -> :ok end, job_id: @job)
      end)

    assert_receive {:agent_request, "scoped-owner", second, replacement}, 2_000
    assert replacement["id"] != request["id"]

    assert {:ok, receipt} = AgentDispatchCancellation.cancel(@job, captured)
    assert receipt["status"] == "requested"
    assert receipt["registered_count"] == 1
    assert_receive {:agent_request, "scoped-owner", _, cancel}
    assert cancel["method"] == "cancel_execution"

    assert cancel["params"] == %{
             "job_id" => @job,
             "request_id" => request["id"],
             "process_instance_id" => "scoped-owner-process",
             "generation" => 1
           }

    refute_receive {:agent_request, _, _, _}, 20
    refute receipt["execution_terminal_confirmed"]
    assert Task.yield(later, 0) == nil
    send(second, {:reply, %{"ok" => true, "result" => %{"unaffected" => true}}})
    assert {:ok, %{"unaffected" => true}} = Task.await(later, 2_000)
    send(first, {:reply, %{"ok" => true, "result" => %{}}})
    assert {:error, {:rpc_error, "cancelled", _}} = Task.await(original, 2_000)
  end

  test "one bounded probe serves several captured requests, never same-job bystanders" do
    parent = self()

    opts = [
      endpoints: [@endpoint],
      probe: fn _, timeout ->
        assert timeout in 1..15_000
        send(parent, :probe)
        descriptor([entry("one", 1), entry("two", 2), entry("later", 3)])
      end,
      send_cancel: fn endpoint, target, _ ->
        assert endpoint == @endpoint
        send(parent, {:target, target})
        {:ok, ack(target)}
      end
    ]

    assert {:ok, receipt} = AgentDispatchCancellation.cancel(@job, snapshot(["one", "two"]), opts)
    assert receipt["registered_count"] == 2
    assert_receive :probe
    refute_receive :probe
    assert_receive {:target, %{"request_id" => "one", "generation" => 1}}
    assert_receive {:target, %{"request_id" => "two", "generation" => 2}}
    refute_receive {:target, _}
    refute Jason.encode!(receipt) =~ "SECRET"

    for flag <-
          ~w(execution_terminal_confirmed pending_cancellation_created automatic_replay_authorized job_wide_fallback_performed),
        do: refute(receipt[flag])
  end

  test "replaced endpoint session, moved address and removed endpoint never receive a probe" do
    for endpoints <- [
          [],
          [%{@endpoint | agent_session_id: "new-session"}],
          [%{@endpoint | port: 5100}],
          [%{@endpoint | id: "peer"}]
        ] do
      assert {:ok, receipt} =
               AgentDispatchCancellation.cancel(@job, snapshot(["one"]),
                 endpoints: endpoints,
                 probe: never_probe(),
                 send_cancel: never_send()
               )

      assert hd(receipt["targets"])["status"] == "original_endpoint_not_configured"
    end
  end

  test "absent old request does not cancel a new request with the same job id" do
    assert {:ok, receipt} = cancel(snapshot(["one"]), descriptor([entry("replacement", 2)]))
    assert hd(receipt["targets"])["status"] == "request_not_observed_active"
    assert receipt["status"] == "not_requested"
    refute receipt["execution_terminal_confirmed"]
  end

  test "duplicate, foreign or malformed observations never widen the cancellation scope" do
    invalid = [
      descriptor([entry("one", 1), entry("one", 2)]),
      descriptor([Map.put(entry("one", 1), "job_id", "foreign-job")]),
      descriptor([entry("one", 0)]),
      descriptor([entry("one", 18_446_744_073_709_551_616)]),
      descriptor([Map.delete(entry("one", 1), "cancel_requested")]),
      descriptor(List.duplicate(entry("one", 1), 1025)),
      descriptor([entry("one", 1)], "bad\nprocess"),
      descriptor([entry("one", 1)], "unavailable"),
      {:ok, %{}},
      {:error, {:timeout, "SECRET"}}
    ]

    for observation <- invalid do
      assert {:ok, receipt} = cancel(snapshot(["one"]), observation)
      assert receipt["registered_count"] == 0
      refute hd(receipt["targets"])["delivery_attempted"]
      refute Jason.encode!(receipt) =~ "SECRET"
    end
  end

  test "process or generation replacement after observation never triggers retargeting" do
    parent = self()

    assert {:ok, receipt} =
             cancel(snapshot(["one"]), descriptor([entry("one", 1)]), fn _, target, _ ->
               send(parent, {:target, target})
               {:ok, ack(target, false)}
             end)

    assert_receive {:target, %{"generation" => 1, "process_instance_id" => "process-one"}}
    refute_receive {:target, _}
    assert hd(receipt["targets"])["status"] == "target_not_observed"
  end

  test "lost acknowledgement stays unknown and forged success is not accepted" do
    for sender <- [
          fn _, _, _ -> {:error, {:timeout, "SECRET"}} end,
          fn _, _, _ -> exit(:transport_stopped) end,
          fn _, target, _ ->
            {:ok, Map.put(ack(target), "execution_terminal_confirmed", true)}
          end,
          fn _, target, _ -> {:ok, put_in(ack(target), ~w(execution_target generation), 2)} end
        ] do
      assert {:ok, receipt} = cancel(snapshot(["one"]), descriptor([entry("one", 1)]), sender)
      assert receipt["status"] == "cancellation_outcome_unknown"
      assert is_nil(hd(receipt["targets"])["cancel_registered"])
      refute receipt["execution_terminal_confirmed"]
      refute Jason.encode!(receipt) =~ "SECRET"
    end
  end

  test "empty, queued and reserved snapshots do not send requests" do
    for {queued, reserved, status} <- [
          {0, 0, "no_active_dispatch"},
          {1, 0, "cancelled_before_dispatch"},
          {0, 1, "cancelled_before_dispatch"}
        ] do
      targets = %{
        snapshot([])
        | queued_cancelled_count: queued,
          reserved_cancelled_count: reserved
      }

      assert {:ok, receipt} =
               AgentDispatchCancellation.cancel(@job, targets,
                 endpoints: [@endpoint],
                 probe: never_probe(),
                 send_cancel: never_send()
               )

      assert receipt["status"] == status
      assert receipt["targets"] == []
    end
  end

  test "old endpoint-only snapshots and invalid identities do not fall back to cancel_job" do
    invalid = [
      Map.delete(snapshot(["one"]), :requests),
      snapshot(["one", "one"]),
      put_in(snapshot(["one"]), [:requests], [%{endpoint: @endpoint}]),
      snapshot(["bad\nrequest"]),
      %{snapshot([]) | queued_cancelled_count: -1},
      %{snapshot([]) | requests: [request("one") |> Map.put(:job_id, "foreign")]},
      %{snapshot([]) | requests: [request("one") |> put_in([:endpoint, :port], 0)]}
    ]

    for targets <- invalid do
      assert {:error, :invalid_execution_cancellation_snapshot} =
               AgentDispatchCancellation.cancel(@job, targets,
                 endpoints: [@endpoint],
                 probe: never_probe(),
                 send_cancel: never_send()
               )
    end

    assert {:error, :invalid_execution_job_id} =
             AgentDispatchCancellation.cancel("", snapshot([]))
  end

  test "capacity limits reject the entire snapshot before side effects" do
    requests = for i <- 1..129, do: request("request-#{i}")

    endpoints =
      for i <- 1..33, do: %{request("request-#{i}") | endpoint: %{@endpoint | id: "agent-#{i}"}}

    for entries <- [requests, endpoints] do
      assert {:error, :execution_cancellation_capacity_exceeded} =
               AgentDispatchCancellation.cancel(
                 @job,
                 %{snapshot([]) | requests: entries},
                 endpoints: [@endpoint],
                 probe: never_probe(),
                 send_cancel: never_send()
               )
    end
  end

  test "maximum request and endpoint counts are accepted without silently truncating work" do
    entries =
      for i <- 0..127 do
        %{request("request-#{i}") | endpoint: %{@endpoint | id: "agent-#{rem(i, 32)}"}}
      end

    parent = self()

    assert {:ok, receipt} =
             AgentDispatchCancellation.cancel(@job, %{snapshot([]) | requests: entries},
               endpoints: Enum.map(entries, & &1.endpoint),
               clock: fn -> 0 end,
               probe: fn endpoint, _ ->
                 send(parent, :probe)

                 entries
                 |> Enum.filter(&(&1.endpoint.id == endpoint.id))
                 |> Enum.map(&entry(&1.request_id, 1))
                 |> descriptor()
               end,
               send_cancel: fn _, target, _ -> {:ok, ack(target)} end
             )

    assert receipt["registered_count"] == 128
    assert length(receipt["targets"]) == 128
    for _ <- 1..32, do: assert_receive(:probe)
    refute_receive :probe
  end

  test "one failed endpoint probe does not prevent cancellation on another captured endpoint" do
    peer = %{@endpoint | id: "second-captured-owner", port: 5100}
    entries = [request("one"), %{request("two") | endpoint: peer}]

    assert {:ok, receipt} =
             AgentDispatchCancellation.cancel(@job, %{snapshot([]) | requests: entries},
               endpoints: [@endpoint, peer],
               probe: fn endpoint, _ ->
                 if endpoint.id == @endpoint.id,
                   do: exit(:probe_stopped),
                   else: descriptor([entry("two", 2)])
               end,
               send_cancel: fn endpoint, target, _ ->
                 assert endpoint == peer
                 assert target["request_id"] == "two"
                 {:ok, ack(target)}
               end
             )

    assert receipt["status"] == "partially_requested"
    assert receipt["registered_count"] == 1
    assert hd(receipt["targets"])["status"] == "original_endpoint_unreachable"
  end

  test "malformed endpoint session is rejected before fingerprinting or network activity" do
    for session <- [%{}, 42, "bad\nidentity", String.duplicate("a", 257)] do
      targets = %{
        snapshot([])
        | requests: [put_in(request("one"), [:endpoint, :agent_session_id], session)]
      }

      assert {:error, :invalid_execution_cancellation_snapshot} =
               AgentDispatchCancellation.cancel(
                 @job,
                 targets,
                 endpoints: [@endpoint],
                 probe: never_probe(),
                 send_cancel: never_send()
               )
    end
  end

  test "delivery start budget prevents cancels after a slow probe and prevents later probes" do
    Process.put(:cancel_clock, 0)
    parent = self()

    assert {:ok, receipt} =
             AgentDispatchCancellation.cancel(@job, snapshot(["one", "two"]),
               endpoints: [@endpoint],
               clock: fn -> Process.get(:cancel_clock) end,
               probe: fn _, _ ->
                 send(parent, :probe)
                 Process.put(:cancel_clock, 15_000)
                 descriptor([entry("one", 1), entry("two", 2)])
               end,
               send_cancel: never_send()
             )

    assert_receive :probe
    refute_receive :probe
    assert Enum.all?(receipt["targets"], &(&1["status"] == "delivery_budget_exhausted"))
  end

  defp cancel(snapshot, observation, sender \\ never_send()),
    do:
      AgentDispatchCancellation.cancel(@job, snapshot,
        endpoints: [@endpoint],
        probe: fn _, _ -> observation end,
        send_cancel: sender
      )

  defp never_probe, do: fn _, _ -> flunk("must not probe an Agent") end
  defp never_send, do: fn _, _, _ -> flunk("must not send any cancellation") end
  defp request(id), do: %{endpoint: @endpoint, request_id: id, job_id: @job}

  defp snapshot(ids),
    do: %{
      requests: Enum.map(ids, &request/1),
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

  defp entry(id, generation),
    do: %{
      "request_id" => id,
      "job_id" => @job,
      "generation" => generation,
      "cancel_requested" => false
    }

  defp descriptor(active, process \\ "process-one"),
    do:
      {:ok,
       %{
         "lifecycle" => %{"process_instance_id" => process},
         "solver_control" => %{
           "schema_version" => "kyuubiki.agent-solver-control/v1",
           "available" => true,
           "active" => active
         }
       }}

  defp ack(target, matched \\ true),
    do: %{
      "schema_version" => "kyuubiki.agent-execution-cancellation/v1",
      "execution_target" => target,
      "status" => if(matched, do: "requested", else: "target_not_observed"),
      "cancel_registered" => matched,
      "execution_terminal_confirmed" => false,
      "pending_cancellation_created" => false,
      "operator_package_cleanup_performed" => false,
      "automatic_replay_authorized" => false
    }
end
