defmodule KyuubikiWeb.Orchestra.OperatorDispatchResultTest do
  use ExUnit.Case, async: false
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchResult, as: Result
  alias KyuubikiWeb.TestSupport.ControlledSolverAgent

  @schema_path Path.expand(
                 "../../../../../schemas/agent-task-result-retention.schema.json",
                 __DIR__
               )
  @external_resource @schema_path
  @retention_policy @schema_path
                    |> File.read!()
                    |> Jason.decode!()
                    |> get_in(["$defs", "policy", "properties"])
                    |> Map.new(fn {key, specification} ->
                      {key, Map.fetch!(specification, "const")}
                    end)

  setup do
    root =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-result-test-#{Base.encode16(:crypto.strong_rand_bytes(16))}"
      )

    journal = start_supervised!({Journal, root: root, name: nil})
    on_exit(fn -> File.rm_rf!(root) end)
    endpoint = %{id: "owner", host: "127.0.0.1", port: 1, agent_session_id: "owner-session"}

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

    {:ok, _attempt} = Journal.begin_dispatch(request, endpoint, journal)
    {:ok, [record]} = Journal.lookup("task", String.duplicate("a", 64), journal)
    query = Map.take(record, ~w(attempt_id task_id task_digest))

    %{
      root: root,
      journal: journal,
      endpoint: endpoint,
      record: record,
      query: query,
      request: request
    }
  end

  test "recovering a retained receipt never rewrites a sealed unknown observation", ctx do
    attempt = ctx.query["attempt_id"]
    assert :ok = Journal.finish_dispatch(attempt, "outcome_unknown", ctx.journal)
    {:ok, before} = Journal.lookup_task("task", ctx.journal)
    path = Path.join(ctx.root, attempt <> ".json")
    bytes = File.read!(path)

    assert {:error, :operator_task_dispatch_transition_rejected} =
             Journal.finish_dispatch(attempt, "observed_executed", ctx.journal)

    assert {:ok, result} = fetch(ctx, fn _, _ -> {:ok, retained(ctx.record)} end)
    assert result["status"] == "receipt_recovered"
    assert result["outcome"] == "executed"
    refute result["automatic_replay_authorized"]
    refute result["publication_performed"]
    assert {:ok, ^before} = Journal.lookup_task("task", ctx.journal)
    assert File.read!(path) == bytes
  end

  test "one original endpoint returns a fully gated receipt without updating the journal", ctx do
    response = retained(ctx.record)
    parent = self()

    probe = fn endpoint, query ->
      send(parent, {:probe, endpoint, query})
      {:ok, response}
    end

    assert {:ok, result} = fetch(ctx, probe)
    assert result["status"] == "receipt_recovered"
    assert result["outcome"] == "executed"
    refute result["automatic_replay_authorized"]
    refute result["publication_performed"]
    assert_receive {:probe, endpoint, query}
    assert endpoint == ctx.endpoint

    assert query ==
             Map.take(
               ctx.record,
               ~w(attempt_id request_id task_id task_digest operator_id program_id)
             )

    assert {:ok, [record]} = Journal.lookup("task", ctx.query["task_digest"], ctx.journal)
    assert record == ctx.record
    assert result["completion"]["result"]["result"]["value"] == 42
  end

  test "a valid receipt cannot escape after its journal file disappears during retrieval", ctx do
    probe = fn _, _ ->
      File.rm!(Path.join(ctx.root, ctx.record["attempt_id"] <> ".json"))
      {:ok, retained(ctx.record)}
    end

    assert {:error, :operator_task_dispatch_journal_invalid} = fetch(ctx, probe)
  end

  for fault <- [:corrupt, :replacement, :partial, :missing_root],
      reply <- [:retained, :expired, :unreachable] do
    test "#{fault} during #{reply} lookup rejects the reply and quarantines damaged history",
         ctx do
      path = Path.join(ctx.root, ctx.record["attempt_id"] <> ".json")

      probe = fn _, _ ->
        case unquote(fault) do
          :corrupt ->
            File.write!(path, "SECRET damaged observation")

          :replacement ->
            assert :ok =
                     Files.write(
                       ctx.root,
                       Map.put(ctx.record, "state", "outcome_unknown"),
                       ctx.record
                     )

          :partial ->
            File.write!(path <> ".next", "SECRET incomplete replacement")

          :missing_root ->
            File.rm_rf!(ctx.root)
        end

        case unquote(reply) do
          :retained ->
            {:ok, retained(ctx.record)}

          :expired ->
            {:ok,
             retained(ctx.record) |> Map.put("status", "not_retained") |> Map.put("response", nil)}

          :unreachable ->
            {:error, :timeout}
        end
      end

      assert {:error, error} = fetch(ctx, probe)

      assert error in [
               :operator_task_dispatch_journal_invalid,
               :operator_task_dispatch_journal_unavailable
             ]

      before = File.read(path)
      assert {:error, ^error} = Journal.lookup_task("task", ctx.journal)
      assert File.read(path) == before
      assert Process.alive?(ctx.journal)
    end
  end

  for state <- ~w(observed_executed outcome_unknown not_dispatched) do
    test "concurrent #{state} rejects only the stale lookup and permits a fresh read", ctx do
      probe = fn _, _ ->
        assert :ok =
                 Journal.finish_dispatch(ctx.record["attempt_id"], unquote(state), ctx.journal)

        {:ok, retained(ctx.record)}
      end

      assert {:error, :operator_task_dispatch_observation_changed} = fetch(ctx, probe)
      assert {:ok, [current]} = Journal.lookup_task("task", ctx.journal)
      assert current["state"] == unquote(state)
      bytes = File.read!(Path.join(ctx.root, current["attempt_id"] <> ".json"))

      assert {:ok, result} = fetch(ctx, fn _, _ -> {:ok, retained(ctx.record)} end)

      expected =
        if unquote(state) == "not_dispatched", do: "not_dispatched", else: "receipt_recovered"

      assert result["status"] == expected
      refute result["automatic_replay_authorized"]
      refute result["publication_performed"]
      assert File.read!(Path.join(ctx.root, current["attempt_id"] <> ".json")) == bytes
    end
  end

  test "eviction during lookup cannot return a receipt detached from its original observation",
       ctx do
    assert :ok =
             Journal.finish_dispatch(ctx.record["attempt_id"], "observed_executed", ctx.journal)

    probe = fn _, _ ->
      for _ <- 1..140 do
        assert {:ok, id} = Journal.begin_dispatch(ctx.request, ctx.endpoint, ctx.journal)
        assert :ok = Journal.finish_dispatch(id, "not_dispatched", ctx.journal)
      end

      {:ok, retained(ctx.record)}
    end

    assert {:error, :operator_task_dispatch_observation_changed} = fetch(ctx, probe)
    assert {:ok, records} = Journal.lookup_task("task", ctx.journal)
    assert length(records) == 128
    refute Enum.any?(records, &(&1["attempt_id"] == ctx.record["attempt_id"]))

    assert {:ok, %{"status" => "no_retained_dispatch"}} =
             fetch(ctx, fn _, _ -> flunk("no RPC") end)
  end

  test "journal process loss during retrieval cannot return a cached completion", ctx do
    probe = fn _, _ ->
      GenServer.stop(ctx.journal, :normal)
      {:ok, retained(ctx.record)}
    end

    assert {:error, :operator_task_dispatch_journal_unavailable} = fetch(ctx, probe)
  end

  test "revalidation stays scoped to the selected observation, not unrelated payloads", ctx do
    other = put_in(ctx.request, ["params", "task_ir", "task_id"], "unrelated")
    {:ok, id} = Journal.begin_dispatch(other, ctx.endpoint, ctx.journal)

    probe = fn _, _ ->
      File.write!(Path.join(ctx.root, id <> ".json"), "damaged unrelated payload")
      {:ok, retained(ctx.record)}
    end

    assert {:ok, %{"status" => "receipt_recovered"}} = fetch(ctx, probe)
    assert :ok = Journal.verify_observation(ctx.record, ctx.journal)
    assert {:error, _} = Journal.lookup_task("unrelated", ctx.journal)
  end

  test "malformed observation verification cannot poison the healthy journal", ctx do
    for bad <- [
          nil,
          [],
          %{},
          Map.put(ctx.record, "extra", true),
          Map.put(ctx.record, "state", "bad")
        ] do
      assert {:error, :operator_task_dispatch_journal_invalid} =
               Journal.verify_observation(bad, ctx.journal)
    end

    assert :ok = Journal.verify_observation(ctx.record, ctx.journal)
  end

  test "all identity mirrors, RPC framing and completion contradiction fail closed", ctx do
    payload = retained(ctx.record)

    for path <- [
          ~w(attempt_id),
          ~w(request_id),
          ~w(task_digest),
          ~w(operator_id),
          ~w(program_id),
          ~w(response id),
          ~w(response result task_digest),
          ~w(response result operator_task_ir_status)
        ] do
      bad = put_in(payload, path, "wrong")
      assert {:ok, result} = fetch(ctx, fn _, _ -> {:ok, bad} end)
      assert result["status"] == "agent_receipt_invalid", inspect(path)
      assert result["outcome"] == "unknown"
      assert is_nil(result["completion"])
    end

    bad = put_in(payload, ~w(response result execution_readiness status), "blocked")
    assert {:ok, %{"status" => "agent_receipt_invalid"}} = fetch(ctx, fn _, _ -> {:ok, bad} end)
  end

  test "missing, replaced or unconfigured original targets never trigger a peer RPC", ctx do
    probe = fn _, _ -> flunk("must not probe") end

    assert {:ok, %{"status" => "original_endpoint_not_configured"}} =
             Result.fetch(ctx.query, journal: ctx.journal, endpoints: [], probe: probe)

    replaced = %{ctx.endpoint | agent_session_id: "new-session"}

    assert {:ok, %{"status" => "original_endpoint_not_configured"}} =
             Result.fetch(ctx.query, journal: ctx.journal, endpoints: [replaced], probe: probe)

    missing = Map.put(ctx.query, "attempt_id", String.duplicate("f", 32))

    assert {:ok, %{"status" => "no_retained_dispatch"}} =
             Result.fetch(missing, journal: ctx.journal, endpoints: [ctx.endpoint], probe: probe)
  end

  test "pending, expiry, oversize and duplicate attempts stay unknown with no result", ctx do
    for status <- ~w(pending not_retained result_not_retained attempt_identity_ambiguous) do
      response = retained(ctx.record) |> Map.put("status", status) |> Map.put("response", nil)
      assert {:ok, result} = fetch(ctx, fn _, _ -> {:ok, response} end)
      assert result["status"] == status
      assert result["outcome"] == "unknown"
    end

    assert {:ok, %{"status" => "original_endpoint_unreachable"}} =
             fetch(ctx, fn _, _ -> {:error, :timeout} end)
  end

  test "queries cannot provide destinations or select a different digest", ctx do
    assert {:error, :operator_task_dispatch_query_invalid} =
             Result.fetch(Map.put(ctx.query, "host", "untrusted"))

    wrong = Map.put(ctx.query, "task_digest", String.duplicate("b", 64))

    assert {:ok, %{"status" => "no_retained_dispatch"}} =
             Result.fetch(wrong,
               journal: ctx.journal,
               endpoints: [ctx.endpoint],
               probe: fn _, _ -> flunk("no RPC") end
             )
  end

  test "unknown native replies require every field and the exact bounded retention policy", ctx do
    payload =
      retained(ctx.record) |> Map.put("status", "not_retained") |> Map.put("response", nil)

    for field <- Map.keys(payload) do
      malformed = Map.delete(payload, field)

      assert {:ok, %{"status" => "agent_receipt_invalid", "outcome" => "unknown"}} =
               fetch(ctx, fn _, _ -> {:ok, malformed} end),
             field
    end

    for malformed <- [
          Map.put(payload, "generation", -1),
          Map.put(payload, "generation", 1.5),
          Map.put(payload, "generation", 18_446_744_073_709_551_616),
          Map.put(payload, "process_instance_id", <<255>>),
          Map.put(payload, "retryable", true),
          put_in(payload, ~w(retention_policy survives_agent_restart), true),
          put_in(payload, ~w(retention_policy automatic_replay_authorized), true),
          put_in(payload, ~w(retention_policy ttl_ms), 600_001),
          put_in(payload, ~w(retention_policy extra), "authority"),
          Map.put(payload, "retention_policy", nil)
        ] do
      assert {:ok, result} = fetch(ctx, fn _, _ -> {:ok, malformed} end)
      assert result["status"] == "agent_receipt_invalid"
      assert result["outcome"] == "unknown"
      assert is_nil(result["completion"])
      refute result["publication_performed"]
      refute result["automatic_replay_authorized"]
    end

    payload = Map.put(payload, "generation", nil)

    assert {:ok, %{"status" => "not_retained", "outcome" => "unknown"}} =
             fetch(ctx, fn _, _ -> {:ok, payload} end)
  end

  test "retained terminal receipts reject progress and terminal frame mixtures", ctx do
    before = File.read!(Path.join(ctx.root, ctx.record["attempt_id"] <> ".json"))

    for {key, value} <- [
          {"event", "progress"},
          {"event", nil},
          {"progress", %{}},
          {"progress", nil}
        ] do
      payload = update_in(retained(ctx.record), ["response"], &Map.put(&1, key, value))
      assert {:ok, result} = fetch(ctx, fn _, _ -> {:ok, payload} end)
      assert result["status"] == "agent_receipt_invalid"
      assert result["outcome"] == "unknown"
      assert is_nil(result["completion"])
      refute result["automatic_replay_authorized"]
      refute result["publication_performed"]
      assert File.read!(Path.join(ctx.root, ctx.record["attempt_id"] <> ".json")) == before
    end
  end

  test "valid success and typed failure receipts preserve optional fields and payload extensions",
       ctx do
    for original <- [retained(ctx.record), failed_retained(ctx.record)],
        absent <- [false, true] do
      {counterpart, outcome} =
        if original["response"]["ok"], do: {"error", "executed"}, else: {"result", "failed"}

      payload =
        update_in(original, ["response"], fn response ->
          response = Map.put(response, "trace_extension", %{"opaque" => true})

          if absent,
            do: Map.delete(response, counterpart),
            else: Map.put(response, counterpart, nil)
        end)

      assert {:ok, result} = fetch(ctx, fn _, _ -> {:ok, payload} end)
      assert result["status"] == "receipt_recovered"
      assert result["outcome"] == outcome

      if outcome == "executed",
        do: assert(result["completion"]["result"]["result"]["value"] == 42)

      refute result["automatic_replay_authorized"]
      refute result["publication_performed"]
    end
  end

  test "retained typed failures reject mixed frames and malformed error text", ctx do
    original = failed_retained(ctx.record)
    before = File.read!(Path.join(ctx.root, ctx.record["attempt_id"] <> ".json"))

    for payload <- [
          put_in(original, ~w(response event), "heartbeat"),
          put_in(original, ~w(response progress), nil),
          put_in(original, ~w(response result), %{}),
          put_in(original, ~w(response error code), false),
          original
          |> put_in(~w(response error message), " ")
          |> put_in(~w(response error details operator_task_failure_receipt message), " ")
        ] do
      assert {:ok, result} = fetch(ctx, fn _, _ -> {:ok, payload} end)
      assert result["status"] == "agent_receipt_invalid"
      assert result["outcome"] == "unknown"
      assert is_nil(result["completion"])
      refute result["automatic_replay_authorized"]
      refute result["publication_performed"]
      assert File.read!(Path.join(ctx.root, ctx.record["attempt_id"] <> ".json")) == before
    end
  end

  for fault <- [:none, :nested_status, :outer_result, :malformed] do
    test "raw #{fault} original-result response is gated without rewriting history", ctx do
      start_supervised!({ControlledSolverAgent, {self(), %{"stage" => "lookup"}}})
      assert_receive {:fake_agent_ready, port}
      endpoint = %{ctx.endpoint | port: port}
      {:ok, attempt} = Journal.begin_dispatch(ctx.request, endpoint, ctx.journal)
      assert :ok = Journal.finish_dispatch(attempt, "outcome_unknown", ctx.journal)
      {:ok, records} = Journal.lookup_task("task", ctx.journal)
      record = Enum.find(records, &(&1["attempt_id"] == attempt))
      query = Map.take(record, ~w(attempt_id task_id task_digest))
      path = Path.join(ctx.root, attempt <> ".json")
      before = File.read!(path)

      lookup =
        Task.async(fn -> Result.fetch(query, journal: ctx.journal, endpoints: [endpoint]) end)

      assert_receive {:solver_request, agent, request}, 2_000
      assert request["method"] == "fetch_operator_task_result"
      assert request["params"]["attempt_id"] == attempt
      id = Jason.encode!(request["id"])
      payload = Jason.encode!(retained(record))

      raw =
        case unquote(fault) do
          :none ->
            ~s({"rpc_version":1,"id":#{id},"ok":true,"result":#{payload}})

          :nested_status ->
            payload =
              String.replace(
                payload,
                ~S("operator_task_ir_status":"executed"),
                ~S("operator_task_ir_status":"executed","operator_task_ir_status":"blocked")
              )

            ~s({"rpc_version":1,"id":#{id},"ok":true,"result":#{payload}})

          :outer_result ->
            ~s({"rpc_version":1,"id":#{id},"ok":true,"result":#{payload},"result":{}})

          :malformed ->
            ~s({"rpc_version":1,"id":#{id},"ok":true,"SECRET":"PRIVATE",)
        end

      send(agent, {:raw_reply, raw})
      assert {:ok, result} = Task.await(lookup)

      if unquote(fault) == :none do
        assert result["status"] == "receipt_recovered"
        assert result["outcome"] == "executed"
        assert result["completion"]["result"]["result"]["value"] == 42
      else
        assert result["status"] == "agent_receipt_invalid"
        assert result["outcome"] == "unknown"
        assert is_nil(result["completion"])
      end

      refute result["automatic_replay_authorized"]
      refute result["publication_performed"]
      refute inspect(result) =~ "PRIVATE"
      assert File.read!(path) == before
      assert {:ok, ^records} = Journal.lookup_task("task", ctx.journal)
      refute_receive {:solver_request, _, _}, 20
    end
  end

  defp failed_retained(record) do
    failure =
      Map.merge(Map.take(record, ~w(task_id task_digest operator_id program_id)), %{
        "schema_version" => "kyuubiki.agent-operator-task-failure/v1",
        "failure_owner" => "agent_runtime",
        "failure_stage" => "solving",
        "reason_code" => "worker_failed",
        "message" => "worker failed",
        "recovery" => %{
          "retryable" => false,
          "safe_to_continue_other_tasks" => true,
          "required_action" => "inspect_input"
        }
      })

    put_in(retained(record), ["response"], %{
      "rpc_version" => 1,
      "id" => record["request_id"],
      "ok" => false,
      "result" => nil,
      "error" => %{
        "code" => "worker_failed",
        "message" => "worker failed",
        "details" => %{"operator_task_failure_receipt" => failure}
      }
    })
  end

  defp fetch(ctx, probe),
    do: Result.fetch(ctx.query, journal: ctx.journal, endpoints: [ctx.endpoint], probe: probe)

  defp retained(record) do
    summary = Map.take(record, ~w(task_id task_digest operator_id program_id))

    receipt =
      Map.merge(summary, %{
        "operator_task_ir_status" => "executed",
        "result" => %{"value" => 42},
        "execution_readiness" => %{"status" => "executed", "ready_to_dispatch" => true}
      })

    Map.merge(
      Map.take(record, ~w(attempt_id request_id task_id task_digest operator_id program_id)),
      %{
        "schema_version" => "kyuubiki.agent-task-result-retention/v1",
        "process_instance_id" => "agent-boot",
        "generation" => 1,
        "status" => "receipt_retained",
        "automatic_replay_authorized" => false,
        "retention_policy" => @retention_policy,
        "response" => %{
          "rpc_version" => 1,
          "id" => record["request_id"],
          "ok" => true,
          "result" => receipt
        }
      }
    )
  end
end
