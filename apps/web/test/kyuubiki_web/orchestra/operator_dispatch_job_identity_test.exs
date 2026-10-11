defmodule KyuubikiWeb.Orchestra.OperatorDispatchJobIdentityTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Orchestra.OperatorDispatchCancellation, as: Cancellation
  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchInspection, as: Inspection
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorDispatchResult, as: Result

  setup do
    root =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-parent-dispatch-#{System.unique_integer([:positive])}"
      )

    journal = start_supervised!({Journal, root: root, name: nil})
    on_exit(fn -> File.rm_rf!(root) end)
    endpoint = %{id: "owner", host: "127.0.0.1", port: 1, agent_session_id: "owner-session"}
    %{root: root, journal: journal, endpoint: endpoint}
  end

  test "dispatch records retain the actual RPC job separately from the task across restart",
       ctx do
    {ctx, record} = dispatch(ctx)
    assert record["schema_version"] == "kyuubiki.operator-task-dispatch-record/v2"
    assert record["job_id"] == "parent-job"
    assert record["task_id"] == "parent-job:diagnose:extract.thermal_result_diagnostics"
    path = Path.join(ctx.root, record["attempt_id"] <> ".json")
    bytes = File.read!(path)
    stop_supervised(Journal)
    journal = start_supervised!({Journal, root: ctx.root, name: nil})
    assert {:ok, [^record]} = Journal.lookup(record["task_id"], record["task_digest"], journal)
    assert File.read!(path) == bytes
  end

  test "parent-bound inspection and cancellation share one exact retained target", ctx do
    {ctx, record} = dispatch(ctx)
    target = target()
    assert {:ok, report} = inspect_dispatch(ctx, record, target)
    assert [attempt] = report["attempts"]
    assert attempt["job_id"] == "parent-job"
    assert attempt["observation"]["execution_target"] == target
    before = File.read!(Path.join(ctx.root, record["attempt_id"] <> ".json"))

    assert {:ok, receipt} = cancel(ctx, record, target)
    assert receipt["status"] == "requested"
    assert receipt["cancel_registered"]
    refute receipt["job_wide_fallback_performed"]
    refute receipt["journal_mutation_performed"]
    assert_receive {:cancel, endpoint, ^target}
    assert endpoint == ctx.endpoint
    refute_receive {:cancel, _, _}, 0
    assert File.read!(Path.join(ctx.root, record["attempt_id"] <> ".json")) == before
  end

  test "foreign parent or request identity cannot cancel the retained dispatch", ctx do
    {ctx, record} = dispatch(ctx)

    for field <- ~w(job_id request_id) do
      changed = Map.put(target(), field, "other")
      assert {:error, :operator_task_dispatch_target_mismatch} = cancel(ctx, record, changed)
      assert {:ok, report} = inspect_dispatch(ctx, record, changed)
      assert is_nil(report["attempts"] |> hd() |> get_in(["observation", "execution_target"]))
    end

    refute_receive {:cancel, _, _}, 0
  end

  test "a long workflow task identity can use a short parent execution target", ctx do
    {ctx, record} = dispatch(ctx, String.duplicate("task-node", 80))
    assert byte_size(record["task_id"]) > 256
    assert {:ok, report} = inspect_dispatch(ctx, record, target())
    assert report["attempts"] |> hd() |> get_in(["observation", "execution_target"]) == target()
    assert {:ok, %{"status" => "requested"}} = cancel(ctx, record, target())
    assert_receive {:cancel, _, _}
  end

  test "an unbound RPC cannot invent a job identity from the task name", ctx do
    request = Map.update!(request("task"), "params", &Map.delete(&1, "job_id"))
    assert {:ok, _} = Journal.begin_dispatch(request, ctx.endpoint, ctx.journal)
    assert {:ok, [record]} = Journal.lookup("task", digest(), ctx.journal)
    assert Map.has_key?(record, "job_id") and is_nil(record["job_id"])
    supplied = Map.put(target(), "job_id", "task")
    assert {:ok, report} = inspect_dispatch(ctx, record, supplied)
    assert is_nil(report["attempts"] |> hd() |> get_in(["observation", "execution_target"]))
    assert {:error, :operator_task_dispatch_job_identity_unknown} = cancel(ctx, record, supplied)
    refute_receive {:cancel, _, _}, 0
  end

  test "malformed execution job identities fail before creating a journal file", ctx do
    for job <- [false, [], %{}, 123, "", String.duplicate("x", 1025)] do
      bad = put_in(request("task"), ["params", "job_id"], job)

      assert {:error, :operator_task_dispatch_journal_invalid} =
               Journal.begin_dispatch(bad, ctx.endpoint, ctx.journal)
    end

    refute File.exists?(ctx.root)
  end

  test "record versions have closed field sets and cannot smuggle or omit execution identity",
       ctx do
    {_ctx, record} = dispatch(ctx)
    assert Files.valid?(record)
    assert Files.valid?(Map.put(record, "job_id", nil))
    refute Files.valid?(Map.delete(record, "job_id"))
    refute Files.valid?(Map.put(record, "extra", true))
    refute Files.valid?(Map.put(record, "schema_version", "unknown"))
    legacy = Map.put(record, "schema_version", "kyuubiki.operator-task-dispatch-record/v1")
    refute Files.valid?(legacy)
    assert Files.valid?(Map.delete(legacy, "job_id"))

    for {filename, fields, version} <- [
          {"operator-task-dispatch-record-v2.schema.json", Map.keys(record), Files.schema()},
          {"operator-task-dispatch-record.schema.json", Map.keys(Map.delete(legacy, "job_id")),
           legacy["schema_version"]}
        ] do
      schema =
        Path.expand("../../../../../schemas/#{filename}", __DIR__)
        |> File.read!()
        |> Jason.decode!()

      assert schema["$id"] == version
      assert schema["properties"]["schema_version"]["const"] == version
      assert Enum.sort(schema["required"]) == Enum.sort(fields)
      refute schema["additionalProperties"]
    end
  end

  test "legacy records remain byte-stable and recover receipts without guessed cancellation",
       ctx do
    {ctx, record} = dispatch(ctx, "task")
    path = Path.join(ctx.root, record["attempt_id"] <> ".json")
    stop_supervised(Journal)
    File.rm!(path)

    legacy =
      record
      |> Map.delete("job_id")
      |> Map.put("schema_version", "kyuubiki.operator-task-dispatch-record/v1")

    assert :ok = Files.write(ctx.root, legacy)
    before = File.read!(path)
    journal = start_supervised!({Journal, root: ctx.root, name: nil})
    ctx = %{ctx | journal: journal}
    assert {:ok, [^legacy]} = Journal.lookup("task", digest(), journal)
    supplied = Map.put(target(), "job_id", "task")
    assert {:ok, report} = inspect_dispatch(ctx, legacy, supplied)
    assert is_nil(hd(report["attempts"])["job_id"])
    assert is_nil(report["attempts"] |> hd() |> get_in(["observation", "execution_target"]))
    assert {:error, :operator_task_dispatch_job_identity_unknown} = cancel(ctx, legacy, supplied)
    refute_receive {:cancel, _, _}, 0

    assert {:ok, result} =
             Result.fetch(query(legacy),
               journal: journal,
               endpoints: [ctx.endpoint],
               probe: fn _, identity -> {:ok, retained(identity)} end
             )

    assert result["status"] == "receipt_recovered"
    assert result["outcome"] == "executed"
    refute result["publication_performed"]
    refute result["automatic_replay_authorized"]
    assert File.read!(path) == before

    assert :ok = Journal.finish_dispatch(legacy["attempt_id"], "observed_failed", journal)
    assert {:ok, [finished]} = Journal.lookup("task", digest(), journal)
    assert finished["schema_version"] == legacy["schema_version"]
    refute Map.has_key?(finished, "job_id")
    assert {:ok, reloaded} = Files.load(ctx.root)
    assert reloaded[legacy["attempt_id"]] == finished
  end

  defp dispatch(ctx, id \\ "parent-job:diagnose:extract.thermal_result_diagnostics") do
    assert {:ok, _} = Journal.begin_dispatch(request(id), ctx.endpoint, ctx.journal)
    assert {:ok, [record]} = Journal.lookup(id, digest(), ctx.journal)
    {ctx, record}
  end

  defp request(id),
    do: %{
      "id" => "original-rpc",
      "params" => %{
        "job_id" => "parent-job",
        "task_ir" => %{
          "task_id" => id,
          "integrity" => %{"task_digest" => digest()},
          "operator" => %{"id" => "extract.thermal_result_diagnostics"},
          "execution_program" => %{"program_id" => "extract.thermal_result_diagnostics"}
        }
      }
    }

  defp digest, do: String.duplicate("a", 64)

  defp target,
    do: %{
      "process_instance_id" => "owner-boot",
      "generation" => 3,
      "request_id" => "original-rpc",
      "job_id" => "parent-job"
    }

  defp query(record), do: Map.take(record, ~w(attempt_id task_id task_digest))

  defp inspect_dispatch(ctx, record, target) do
    Inspection.inspect_task(Map.take(record, ~w(task_id task_digest)),
      journal: ctx.journal,
      endpoints: [ctx.endpoint],
      probe: fn _ ->
        {:ok,
         %{
           "lifecycle" => %{"process_instance_id" => target["process_instance_id"]},
           "solver_control" => %{
             "schema_version" => "kyuubiki.agent-solver-control/v1",
             "available" => true,
             "active" => [Map.put(target, "cancel_requested", false)]
           }
         }}
      end
    )
  end

  defp cancel(ctx, record, target) do
    Cancellation.cancel(Map.put(query(record), "execution_target", target),
      journal: ctx.journal,
      endpoints: [ctx.endpoint, %{ctx.endpoint | id: "peer", port: 2}],
      send_cancel: fn endpoint, actual ->
        send(self(), {:cancel, endpoint, actual})

        {:ok,
         %{
           "schema_version" => "kyuubiki.agent-execution-cancellation/v1",
           "execution_target" => actual,
           "status" => "requested",
           "cancel_registered" => true,
           "execution_terminal_confirmed" => false,
           "pending_cancellation_created" => false,
           "operator_package_cleanup_performed" => false,
           "automatic_replay_authorized" => false
         }}
      end
    )
  end

  defp retained(identity) do
    schema =
      File.read!(
        Path.expand("../../../../../schemas/agent-task-result-retention.schema.json", __DIR__)
      )
      |> Jason.decode!()

    policy =
      schema["$defs"]["policy"]["properties"]
      |> Map.new(fn {key, spec} -> {key, spec["const"]} end)

    receipt =
      Map.take(identity, ~w(task_id task_digest operator_id program_id))
      |> Map.merge(%{
        "operator_task_ir_status" => "executed",
        "result" => %{"value" => 42},
        "execution_readiness" => %{"status" => "executed", "ready_to_dispatch" => true}
      })

    Map.merge(identity, %{
      "schema_version" => "kyuubiki.agent-task-result-retention/v1",
      "process_instance_id" => "owner-boot",
      "generation" => 3,
      "status" => "receipt_retained",
      "automatic_replay_authorized" => false,
      "retention_policy" => policy,
      "response" => %{
        "rpc_version" => 1,
        "id" => identity["request_id"],
        "ok" => true,
        "result" => receipt
      }
    })
  end
end
