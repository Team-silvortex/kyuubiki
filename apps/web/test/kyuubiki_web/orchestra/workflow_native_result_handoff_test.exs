defmodule KyuubikiWeb.Orchestra.WorkflowNativeResultHandoffTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.{OperatorTaskExecutionSummary, OperatorTaskReadiness}
  alias KyuubikiWeb.WorkflowNativeOperatorRuntime, as: Native

  @policy Path.expand("../../../../../schemas/agent-task-result-retention.schema.json", __DIR__)
          |> File.read!()
          |> Jason.decode!()
          |> get_in(["$defs", "policy", "properties"])
          |> Map.new(fn {key, spec} -> {key, spec["const"]} end)

  defmodule Client do
    def run_operator_task_ir(task, opts, _progress) do
      journal = Process.get(:handoff_journal)

      request = %{
        "id" => "handoff-rpc",
        "params" => %{"task_ir" => task, "job_id" => opts[:job_id]}
      }

      {:ok, attempt} =
        Journal.begin_dispatch(request, %{id: "owner", host: "127.0.0.1", port: 1}, journal)

      :ok = Journal.finish_dispatch(attempt, "observed_executed", journal)
      {:ok, summary} = OperatorTaskExecutionSummary.build(task)
      send(self(), {:executed, attempt})

      {:ok,
       Map.merge(Map.take(summary, ~w(task_id task_digest operator_id program_id)), %{
         "operator_task_ir_status" => "executed",
         "execution_readiness" => OperatorTaskReadiness.local_executed(),
         "result" => %{"value" => 1.0, "tiny" => 1.0e-30}
       })}
    end
  end

  setup do
    root = Path.join(System.tmp_dir!(), "native-handoff-#{System.unique_integer([:positive])}")
    journal = start_supervised!({Journal, root: root, name: nil})
    Process.put(:handoff_journal, journal)
    on_exit(fn -> File.rm_rf!(root) end)
    %{journal: journal, root: root}
  end

  test "successful native output is handed off before it escapes the runtime", ctx do
    callback = fn {:commit, request, receipt} ->
      send(self(), {:committed, request, receipt})
      :ok
    end

    assert {:ok, %{"value" => 1.0}} = run(ctx, native_result_checkpoint: callback)
    assert_receive {:executed, attempt}
    assert_receive {:committed, request, receipt}
    assert request["attempt_id"] == attempt
    assert receipt["result"] === %{"value" => 1.0, "tiny" => 1.0e-30}
  end

  test "durable receipt reuse avoids a new Agent request but still checks the observation", ctx do
    callback = memory_callback()
    assert {:ok, expected} = run(ctx, native_result_checkpoint: callback)
    assert_receive {:executed, attempt}
    path = Path.join(ctx.root, attempt <> ".json")
    bytes = File.read!(path)

    assert {:ok, ^expected} =
             run(ctx,
               native_result_checkpoint: callback,
               recover_native_results: true,
               endpoints: [],
               probe: fn _, _ -> flunk("unexpected RPC") end
             )

    refute_receive {:executed, _}, 0
    assert File.read!(path) == bytes

    File.rm!(path)

    assert {:error, {:native_workflow_recovery_blocked, :journal_unavailable}} =
             run(ctx, native_result_checkpoint: callback, recover_native_results: true)

    refute_receive {:executed, _}, 0
  end

  for fault <- [:error, :raise, :exit, :throw, :unexpected] do
    test "#{fault} during handoff blocks publication instead of losing the completed result",
         ctx do
      callback = fn {:commit, _, _} ->
        case unquote(fault) do
          :error -> {:error, :unavailable}
          :raise -> raise "private storage failure"
          :exit -> exit(:private_storage_failure)
          :throw -> throw(:private_storage_failure)
          :unexpected -> :unexpected
        end
      end

      assert {:error, {:native_workflow_recovery_blocked, reason}} =
               run(ctx, native_result_checkpoint: callback)

      assert reason in [:result_checkpoint_commit_failed, :result_checkpoint_invalid]
      assert_receive {:executed, _}
      refute_receive {:executed, _}, 0
    end
  end

  test "a handoff that loses the selected observation cannot publish success", ctx do
    callback = fn {:commit, request, _} ->
      File.rm!(Path.join(ctx.root, request["attempt_id"] <> ".json"))
      :ok
    end

    assert {:error, {:native_workflow_recovery_blocked, :journal_unavailable}} =
             run(ctx, native_result_checkpoint: callback)
  end

  test "ownership loss during handoff fences the result", ctx do
    callback = fn {:commit, _, _} ->
      Process.put(:handoff_owner_lost, true)
      :ok
    end

    gate = fn -> if Process.get(:handoff_owner_lost), do: {:error, :stale_claim}, else: :ok end

    assert {:error, :stale_claim} =
             run(ctx, native_result_checkpoint: callback, before_dispatch: gate)
  end

  test "invalid callbacks are rejected before dispatch", ctx do
    for callback <- [false, %{}, fn -> :ok end] do
      assert {:error, :invalid_native_workflow_operator_request} =
               run(ctx, native_result_checkpoint: callback)

      refute_receive {:executed, _}, 0
    end
  end

  test "a damaged retained receipt cannot authorize output or trigger a second dispatch", ctx do
    assert {:ok, _} = run(ctx, [])
    assert_receive {:executed, _}

    assert {:error, {:native_workflow_recovery_blocked, :receipt_invalid}} =
             run(ctx,
               native_result_checkpoint: fn {:lookup, _} -> {:ok, %{}} end,
               recover_native_results: true
             )

    refute_receive {:executed, _}, 0
  end

  test "a checkpoint miss cannot authorize replay when the original Agent receipt is gone", ctx do
    assert {:ok, _} = run(ctx, [])
    assert_receive {:executed, _}

    assert {:error, {:native_workflow_recovery_blocked, :receipt_unavailable}} =
             run(ctx,
               native_result_checkpoint: fn {:lookup, _} -> :missing end,
               recover_native_results: true,
               endpoints: []
             )

    refute_receive {:executed, _}, 0
  end

  test "a stored receipt cannot hide a new competing successful attempt", ctx do
    callback = memory_callback()
    assert {:ok, _} = run(ctx, native_result_checkpoint: callback)
    assert {:ok, _} = run(ctx, [])

    assert {:error, {:native_workflow_recovery_blocked, :dispatch_attempt_ambiguous}} =
             run(ctx, native_result_checkpoint: callback, recover_native_results: true)
  end

  test "history lost during checkpoint read still blocks output", ctx do
    callback = memory_callback()
    assert {:ok, _} = run(ctx, native_result_checkpoint: callback)

    changing = fn {:lookup, request} = action ->
      File.rm!(Path.join(ctx.root, request["attempt_id"] <> ".json"))
      callback.(action)
    end

    assert {:error, {:native_workflow_recovery_blocked, :journal_unavailable}} =
             run(ctx, native_result_checkpoint: changing, recover_native_results: true)
  end

  test "checkpoint store loss is not silently treated as an empty cache", ctx do
    assert {:ok, _} = run(ctx, [])
    assert_receive {:executed, _}

    assert {:error, {:native_workflow_recovery_blocked, :result_checkpoint_unavailable}} =
             run(ctx,
               native_result_checkpoint: fn _ -> {:error, :storage_unavailable} end,
               recover_native_results: true,
               probe: fn _, _ -> flunk("must not fetch after storage failure") end
             )

    refute_receive {:executed, _}, 0
  end

  test "a recovered original receipt is handed off before downstream continuation", ctx do
    callback = memory_callback()
    assert {:ok, expected} = run(ctx, native_result_checkpoint: callback)
    assert_receive {:executed, _}

    recovering = fn
      {:lookup, _} ->
        :missing

      {:commit, request, receipt} ->
        send(self(), {:recovered_handoff, request, receipt})
        :ok
    end

    endpoint = %{id: "owner", host: "127.0.0.1", port: 1}

    probe = fn _, query ->
      {:ok, [record]} = Journal.lookup_task(query["task_id"], ctx.journal)
      request = KyuubikiWeb.Orchestra.WorkflowNativeResultCheckpoint.request(record)
      {:ok, receipt} = callback.({:lookup, request})

      {:ok,
       Map.merge(query, %{
         "schema_version" => "kyuubiki.agent-task-result-retention/v1",
         "status" => "receipt_retained",
         "generation" => 1,
         "process_instance_id" => "original",
         "automatic_replay_authorized" => false,
         "retention_policy" => @policy,
         "response" => %{
           "rpc_version" => 1,
           "id" => query["request_id"],
           "ok" => true,
           "result" => receipt
         }
       })}
    end

    assert {:ok, ^expected} =
             run(ctx,
               native_result_checkpoint: recovering,
               recover_native_results: true,
               endpoints: [endpoint],
               probe: probe
             )

    assert_receive {:recovered_handoff, _, receipt}
    assert receipt["result"] === expected
    refute_receive {:executed, _}, 0
  end

  defp memory_callback do
    fn
      {:commit, request, receipt} ->
        Process.put({:saved_receipt, request}, receipt)
        :ok

      {:lookup, request} ->
        case Process.get({:saved_receipt, request}) do
          nil -> :missing
          receipt -> {:ok, receipt}
        end
    end
  end

  defp run(ctx, opts) do
    Native.run(
      "extract",
      "extract.thermal_result_diagnostics",
      %{},
      %{},
      %{"id" => "diagnose"},
      %{"job_id" => "handoff-parent"},
      Keyword.merge([client: Client, journal: ctx.journal], opts)
    )
  end
end
