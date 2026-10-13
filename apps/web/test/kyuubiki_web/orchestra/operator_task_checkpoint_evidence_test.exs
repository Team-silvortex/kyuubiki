defmodule KyuubikiWeb.Orchestra.OperatorTaskCheckpointEvidenceTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.CanonicalJson
  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRun, as: Run
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor, as: Executor
  alias KyuubikiWeb.Orchestra.OperatorTaskIR, as: IR

  test "zero errors alone cannot certify preparation" do
    for preparation <- [%{}, %{"error_count" => 0}, %{"error_count" => 1}] do
      assert {:error, _} = Run.checkpoint_checked(batch(), preparation: preparation)
    end
  end

  test "preparation counters source identity and task summaries must agree" do
    good = batch()
    assert {:ok, prepared} = Executor.prepare_batch(good)

    changes = [
      {"task_count", 1},
      {"verified_count", 1},
      {"error_count", 1},
      {"verified_count", 2.0},
      {"error_count", -1},
      {"task_count", "2"},
      {"batch_digest", String.duplicate("0", 64)},
      {"run_phase", "execute"},
      {"summaries", []},
      {"summaries", nil},
      {"summaries", [hd(prepared["summaries"]), hd(prepared["summaries"])]},
      {"failure_receipts", [%{"case_id" => "case-a"}]},
      {"error_code_counts", %{"failure" => 1}},
      {"error_codes", ["failure"]}
    ]

    for {field, value} <- changes do
      assert {:error, _} =
               Run.checkpoint_checked(good, preparation: Map.put(prepared, field, value))
    end

    for field <- ~w(case_id task_id task_digest operator_id program_id status) do
      invalid = put_in(prepared, ["summaries", Access.at(0), field], "wrong")
      assert {:error, _} = Run.checkpoint_checked(good, preparation: invalid)
    end
  end

  test "preparation checkpoints retain only compact bound evidence across JSON exchange" do
    good = batch()
    assert {:ok, prepared} = Executor.prepare_batch(good)
    prepared = Map.update!(prepared, "summaries", &Enum.reverse/1)
    assert {:ok, checkpoint} = Run.checkpoint_checked(good, preparation: prepared)
    checkpoint = checkpoint |> Jason.encode!() |> Jason.decode!()
    assert {:ok, plan} = Run.resume_plan(good, checkpoint)
    assert plan["next_action"] == "execute"
    assert plan["target_case_ids"] == ~w(case-a case-b)
    assert length(checkpoint["preparation"]["task_evidence"]) == 2
    refute Map.has_key?(checkpoint["preparation"], "summaries")
    refute Jason.encode!(checkpoint) =~ "admission_report"
  end

  test "real preparation failure remains inspectable and cannot become prepared by rehashing" do
    bad = put_in(batch(), ["tasks", Access.at(1), "task_ir", "config", "tampered"], true)
    assert {:ok, prepared} = Executor.prepare_batch(bad)
    assert prepared["error_count"] == 1
    assert {:ok, checkpoint} = Run.checkpoint_checked(bad, preparation: prepared)
    assert {:ok, plan} = Run.resume_plan(bad, checkpoint)
    assert plan["next_action"] == "fix_invalid_cases"

    for {key, value} <- [{"error_count", 0}, {"verified_count", 2}, {"task_evidence", []}] do
      changed = checkpoint |> put_in(["preparation", key], value) |> rehash()
      assert {:error, _} = Run.verify_checkpoint(bad, changed)
      assert {:error, _} = Run.resume_plan(bad, changed)
    end
  end

  test "successful checkpoint revalidation needs task evidence not a completion flag" do
    good = batch()
    assert {:ok, execution} = Executor.execute_batch(good)
    assert {:ok, checkpoint} = Run.checkpoint_checked(good, execution: execution)
    assert {:ok, %{"next_action" => "archive"}} = Run.resume_plan(good, checkpoint)

    for {field, value} <- [
          {"completion_evidence", nil},
          {"completion_evidence", %{}},
          {"completion_verified", false},
          {"incomplete_case_ids", ["case-a"]},
          {"ok_count", 1},
          {"batch_digest", String.duplicate("0", 64)},
          {"readiness_counts", %{"blocked" => 2}}
        ] do
      changed = checkpoint |> put_in(["execution", field], value) |> rehash()
      assert {:error, _} = Run.verify_checkpoint(good, changed)
      assert {:error, _} = Run.resume_plan(good, changed)
    end
  end

  test "completed task evidence rejects foreign duplicate and mismatched identities after rehash" do
    good = batch()
    assert {:ok, execution} = Executor.execute_batch(good)
    assert {:ok, checkpoint} = Run.checkpoint_checked(good, execution: execution)
    rows = checkpoint["execution"]["completion_evidence"]["completed_tasks"]
    assert is_list(rows)

    for invalid <- [
          [hd(rows), hd(rows)],
          [Map.put(hd(rows), "case_id", "foreign") | tl(rows)],
          [Map.put(hd(rows), "task_digest", String.duplicate("0", 64)) | tl(rows)]
        ] do
      changed =
        checkpoint
        |> put_in(["execution", "completion_evidence", "completed_tasks"], invalid)
        |> rehash()

      assert {:error, _} = Run.resume_plan(good, changed)
    end
  end

  test "missing results cannot be upgraded to completion by editing and rehashing the summary" do
    good = batch()
    assert {:ok, execution} = Executor.execute_batch(good)

    assert {:ok, checkpoint} =
             Run.checkpoint_checked(good, execution: Map.put(execution, "results", nil))

    assert {:ok, %{"target_case_ids" => ["case-a", "case-b"]}} = Run.resume_plan(good, checkpoint)

    changed =
      checkpoint
      |> put_in(["execution", "completion_verified"], true)
      |> put_in(["execution", "incomplete_case_ids"], [])
      |> Map.put("resume_policy", %{"status" => "complete", "next_action" => "archive"})
      |> rehash()

    assert {:error, _} = Run.verify_checkpoint(good, changed)
  end

  test "unknown cases remain recovery targets alongside an ordinary task failure" do
    bad = put_in(batch(), ["tasks", Access.at(1), "task_ir", "config", "tampered"], true)
    assert {:ok, execution} = Executor.execute_batch(bad)
    failed = Enum.filter(execution["results"], &(&1["status"] == "error"))

    assert {:ok, checkpoint} =
             Run.checkpoint_checked(bad, execution: Map.put(execution, "results", failed))

    assert {:ok, plan} = Run.resume_plan(bad, checkpoint)
    assert plan["next_action"] == "resolve_incomplete_cases"
    assert plan["target_case_ids"] == ~w(case-a case-b)
  end

  test "foreign run evidence cannot remove successful peers from recovery targets" do
    bad = put_in(batch(), ["tasks", Access.at(1), "task_ir", "config", "tampered"], true)
    assert {:ok, execution} = Executor.execute_batch(bad)

    for {key, value} <- [{"batch_digest", String.duplicate("0", 64)}, {"run_phase", "prepare"}] do
      assert {:ok, checkpoint} =
               Run.checkpoint_checked(bad, execution: Map.put(execution, key, value))

      assert {:ok, plan} = Run.resume_plan(bad, checkpoint)
      assert plan["target_case_ids"] == ~w(case-a case-b)
      refute plan["next_action"] == "retry_failed_cases"
    end
  end

  test "checkpoint size does not grow with result payload and caller proof is not adopted" do
    good = batch()
    assert {:ok, execution} = Executor.execute_batch(good)
    assert {:ok, small} = Run.checkpoint_checked(good, execution: execution, created_at: "fixed")

    large =
      execution
      |> put_in(
        ["results", Access.at(0), "result", "large"],
        String.duplicate("payload", 100_000)
      )
      |> Map.put("completion_evidence", %{"completed_tasks" => ["not-evidence"]})

    assert {:ok, checkpoint} = Run.checkpoint_checked(good, execution: large, created_at: "fixed")
    assert checkpoint == small
    assert {:ok, %{"next_action" => "archive"}} = Run.resume_plan(good, checkpoint)
    refute Map.has_key?(checkpoint["execution"], "results")
  end

  test "empty batch preparation and execution both retain verifiable evidence" do
    empty = Map.put(batch(), "tasks", [])
    assert {:ok, preparation} = Executor.prepare_batch(empty)
    assert {:ok, execution} = Executor.execute_batch(empty)

    for opts <- [[preparation: preparation], [execution: execution]] do
      assert {:ok, checkpoint} = Run.checkpoint_checked(empty, opts)
      assert {:ok, _} = Run.verify_checkpoint(empty, checkpoint)
    end
  end

  test "missing or noninteger completion counters never certify archival" do
    good = batch()
    assert {:ok, execution} = Executor.execute_batch(good)

    for field <-
          ~w(task_count executed_count ok_count error_count attempted_count blocked_count skipped_count),
        invalid <- [
          Map.delete(execution, field),
          Map.put(execution, field, 0.0),
          Map.put(execution, field, -1)
        ] do
      assert {:ok, checkpoint} = Run.checkpoint_checked(good, execution: invalid)
      assert {:ok, plan} = Run.resume_plan(good, checkpoint)
      assert plan["next_action"] == "resolve_incomplete_cases"
      assert plan["target_case_ids"] == ~w(case-a case-b)
    end

    assert {:ok, checkpoint} =
             Run.checkpoint_checked(good,
               execution: Map.put(execution, "readiness_counts", %{"executed" => 2.0})
             )

    assert {:ok, %{"next_action" => "resolve_incomplete_cases"}} =
             Run.resume_plan(good, checkpoint)
  end

  test "malformed persisted evidence fails safely even with a recomputed checksum" do
    good = batch()
    assert {:ok, prepared} = Executor.prepare_batch(good)
    assert {:ok, execution} = Executor.execute_batch(good)

    assert {:ok, checkpoint} =
             Run.checkpoint_checked(good, preparation: prepared, execution: execution)

    for path <- [
          ["preparation", "task_evidence"],
          ["execution", "completion_evidence"],
          ["execution", "completion_evidence", "completed_tasks"],
          ["execution", "completion_evidence", "result_count"]
        ],
        value <- [nil, 42, false, "bad", %{}, [42]] do
      changed = checkpoint |> put_in(path, value) |> rehash()
      assert {:error, _} = Run.verify_checkpoint(good, changed)
      assert {:error, _} = Run.resume_plan(good, changed)
    end
  end

  test "preparation failure receipts must match their own task and reason" do
    bad = put_in(batch(), ["tasks", Access.at(1), "task_ir", "config", "tampered"], true)
    assert {:ok, prepared} = Executor.prepare_batch(bad)
    assert {:ok, checkpoint} = Run.checkpoint_checked(bad, preparation: prepared)

    for field <- ~w(case_id task_id task_digest operator_id reason_code) do
      invalid = put_in(prepared, ["failure_receipts", Access.at(0), field], "wrong")
      assert {:error, _} = Run.checkpoint_checked(bad, preparation: invalid)

      changed =
        checkpoint
        |> put_in(["preparation", "failure_receipts", Access.at(0), field], "wrong")
        |> rehash()

      assert {:error, _} = Run.resume_plan(bad, changed)
    end
  end

  test "foreign error-only summaries require inspection rather than direct retry" do
    bad =
      Map.update!(batch(), "tasks", fn tasks ->
        Enum.map(tasks, &put_in(&1, ["task_ir", "config", "tampered"], true))
      end)

    assert {:ok, execution} = Executor.execute_batch(bad)
    assert execution["error_count"] == 2
    foreign = Map.put(execution, "batch_digest", String.duplicate("0", 64))
    assert {:ok, checkpoint} = Run.checkpoint_checked(bad, execution: foreign)
    assert {:ok, plan} = Run.resume_plan(bad, checkpoint)
    assert plan["next_action"] == "resolve_incomplete_cases"
    assert plan["target_case_ids"] == ~w(case-a case-b)

    changed =
      checkpoint
      |> put_in(["execution", "completion_evidence", "batch_bound"], true)
      |> rehash()

    assert {:error, _} = Run.resume_plan(bad, changed)

    assert {:ok, corrected} = Run.checkpoint_checked(bad, execution: execution)
    assert {:ok, corrected_plan} = Run.resume_plan(bad, corrected)
    assert corrected_plan["recovery_evidence_verified"] == true
    assert corrected_plan["repair_required_case_ids"] == ~w(case-a case-b)
    assert corrected_plan["recovery_actions"] == ["rebuild_task_ir_and_recompute_digest"]
  end

  defp batch do
    tasks =
      for id <- ~w(a b) do
        {:ok, task} = IR.build("export.summary_json", %{"value" => 1}, %{}, task_id: "task-#{id}")
        %{"case_id" => "case-#{id}", "task_ir" => task}
      end

    %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => tasks
    }
  end

  defp rehash(checkpoint) do
    digest =
      checkpoint
      |> Map.delete("checkpoint_digest")
      |> legacy_json()
      |> CanonicalJson.encode!()
      |> then(&:crypto.hash(:sha256, &1))
      |> Base.encode16(case: :lower)

    Map.put(checkpoint, "checkpoint_digest", digest)
  end

  defp legacy_json(value) when is_map(value),
    do: Map.new(value, fn {key, nested} -> {key, legacy_json(nested)} end)

  defp legacy_json([]), do: %{}
  defp legacy_json(value) when is_list(value), do: Enum.map(value, &legacy_json/1)
  defp legacy_json(value), do: value
end
