defmodule KyuubikiWeb.Orchestra.OperatorDispatchCancellationTest do
  use ExUnit.Case, async: false
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorDispatchCancellation, as: Cancellation

  setup do
    root =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-cancel-test-#{Base.encode16(:crypto.strong_rand_bytes(16))}"
      )

    journal = start_supervised!({Journal, root: root, name: nil})
    on_exit(fn -> File.rm_rf!(root) end)
    endpoint = %{id: "owner", host: "127.0.0.1", port: 1, agent_session_id: "original-session"}

    request = %{
      "id" => "original-rpc",
      "params" => %{
        "task_ir" => %{
          "task_id" => "task",
          "integrity" => %{"task_digest" => String.duplicate("a", 64)},
          "operator" => %{"id" => "op"},
          "execution_program" => %{"program_id" => "op"}
        }
      }
    }

    {:ok, attempt} = Journal.begin_dispatch(request, endpoint, journal)

    target = %{
      "process_instance_id" => "original-process",
      "request_id" => "original-rpc",
      "generation" => 3,
      "job_id" => "task"
    }

    query = %{
      "task_id" => "task",
      "task_digest" => String.duplicate("a", 64),
      "attempt_id" => attempt,
      "execution_target" => target
    }

    %{journal: journal, endpoint: endpoint, query: query, target: target}
  end

  test "exact target is sent once only to its retained owner without journal mutation", ctx do
    {:ok, before} = Journal.lookup("task", ctx.query["task_digest"], ctx.journal)
    parent = self()

    assert {:ok, result} =
             cancel(ctx, fn endpoint, target ->
               send(parent, {:cancel, endpoint, target})
               {:ok, ack(target)}
             end)

    assert_receive {:cancel, endpoint, target}
    assert endpoint == ctx.endpoint
    assert target == ctx.target
    refute_receive {:cancel, _, _}
    assert result["status"] == "requested"
    assert result["cancel_registered"] == true
    assert result["delivery_attempted"] == true
    assert result["agent_acknowledgement"] == ack(ctx.target)

    for field <-
          ~w(execution_terminal_confirmed automatic_replay_authorized journal_mutation_performed publication_performed job_wide_fallback_performed),
        do: assert(result[field] == false)

    assert {:ok, ^before} = Journal.lookup("task", ctx.query["task_digest"], ctx.journal)
  end

  test "missing attempts, terminal observations and changed original sessions cause no RPC",
       ctx do
    never = fn _, _ -> flunk("must not contact any Agent") end

    for endpoints <- [[], [%{ctx.endpoint | agent_session_id: "replacement-session"}]] do
      assert {:ok, result} =
               Cancellation.cancel(ctx.query,
                 journal: ctx.journal,
                 endpoints: endpoints,
                 send_cancel: never
               )

      assert result["status"] == "original_endpoint_not_configured"
      refute result["delivery_attempted"]
    end

    missing = Map.put(ctx.query, "attempt_id", String.duplicate("f", 32))
    assert {:ok, %{"status" => "no_retained_dispatch"}} = cancel(ctx, never, missing)

    for state <- ~w(not_dispatched observed_executed observed_failed observed_blocked) do
      :ok = Journal.finish_dispatch(ctx.query["attempt_id"], state, ctx.journal)
      assert {:ok, result} = cancel(ctx, never)

      assert result["status"] ==
               if(state == "not_dispatched", do: state, else: "retained_terminal_dispatch")

      refute result["cancel_registered"]
      refute result["execution_terminal_confirmed"]
    end
  end

  test "stale generation or process acknowledgement does not select a new target", ctx do
    parent = self()

    assert {:ok, result} =
             cancel(ctx, fn endpoint, target ->
               send(parent, {:cancel, endpoint, target})
               {:ok, ack(target, false)}
             end)

    assert_receive {:cancel, _, target}
    assert target == ctx.target
    assert result["status"] == "target_not_observed"
    refute result["cancel_registered"]
    refute_receive {:cancel, _, _}
  end

  test "transport failure stays unknown and never claims cancellation was rejected", ctx do
    assert {:ok, result} =
             cancel(ctx, fn _, _ -> {:error, {:timeout, "SECRET endpoint diagnostic"}} end)

    assert result["status"] == "cancellation_outcome_unknown"
    assert result["uncertainty_reason"] == "original_endpoint_unreachable"
    assert is_nil(result["cancel_registered"])
    assert is_nil(result["agent_acknowledgement"])
    assert result["delivery_attempted"]
    refute inspect(result) =~ "SECRET"
  end

  test "foreign targets contradictory flags missing fields and unknown promises invalidate acknowledgement",
       ctx do
    original = ack(ctx.target)
    changed_target = put_in(original, ~w(execution_target generation), 4)

    mutations =
      [
        changed_target,
        Map.put(original, "cancel_registered", false),
        Map.put(original, "status", "cancelled"),
        Map.put(original, "schema_version", "unknown"),
        Map.put(original, "extra", true)
      ] ++
        Enum.map(
          ~w(execution_terminal_confirmed automatic_replay_authorized pending_cancellation_created operator_package_cleanup_performed),
          &Map.put(original, &1, true)
        ) ++
        Enum.map(Map.keys(original), &Map.delete(original, &1))

    for mutation <- mutations do
      assert {:ok, result} = cancel(ctx, fn _, _ -> {:ok, mutation} end)
      assert result["status"] == "cancellation_outcome_unknown"
      assert result["uncertainty_reason"] == "agent_acknowledgement_invalid"
      assert is_nil(result["cancel_registered"])
      assert is_nil(result["agent_acknowledgement"])
    end
  end

  test "malformed queries and foreign request identities fail before side effects", ctx do
    never = fn _, _ -> flunk("invalid query must not send") end

    assert {:error, :operator_task_dispatch_target_mismatch} =
             cancel(
               ctx,
               never,
               put_in(ctx.query, ~w(execution_target request_id), "another-request")
             )

    invalid = [
      nil,
      %{},
      Map.put(ctx.query, "host", "untrusted"),
      Map.put(ctx.query, "attempt_id", "bad"),
      Map.put(ctx.query, "task_digest", String.duplicate("A", 64)),
      Map.put(ctx.query, "execution_target", nil)
    ]

    invalid = invalid ++ Enum.map(Map.keys(ctx.query), &Map.delete(ctx.query, &1))

    invalid =
      invalid ++
        Enum.map(~w(process_instance_id request_id generation job_id), fn field ->
          put_in(ctx.query, ["execution_target"], Map.delete(ctx.target, field))
        end)

    invalid =
      invalid ++
        Enum.map(
          [0, -1, 1.0, "3", nil, 18_446_744_073_709_551_616],
          &put_in(ctx.query, ~w(execution_target generation), &1)
        )

    invalid =
      invalid ++
        Enum.flat_map(~w(process_instance_id request_id job_id), fn field ->
          Enum.map(
            ["", " ", "x\ny", String.duplicate("x", 257), String.duplicate("\u754c", 100)],
            &put_in(ctx.query, ["execution_target", field], &1)
          )
        end)

    invalid =
      invalid ++
        [
          put_in(ctx.query, ~w(execution_target process_instance_id), "unavailable"),
          put_in(ctx.query, ~w(execution_target job_id), "another-job"),
          put_in(ctx.query, ["execution_target"], Map.put(ctx.target, "cancel_all", true))
        ]

    for query <- invalid,
        do: assert({:error, :operator_task_dispatch_query_invalid} == cancel(ctx, never, query))
  end

  defp cancel(ctx, sender), do: cancel(ctx, sender, ctx.query)

  defp cancel(ctx, sender, query),
    do:
      Cancellation.cancel(query,
        journal: ctx.journal,
        endpoints: [ctx.endpoint, %{ctx.endpoint | id: "peer", port: 2}],
        send_cancel: sender
      )

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
