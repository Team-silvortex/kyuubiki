defmodule KyuubikiWeb.Orchestra.OperatorDispatchResultTest do
  use ExUnit.Case, async: false
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorDispatchResult, as: Result

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
    %{journal: journal, endpoint: endpoint, record: record, query: query}
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
