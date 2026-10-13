defmodule KyuubikiWeb.Orchestra.OperatorTaskBatchRecoveryEvidenceTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.CanonicalJson
  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRun, as: Run
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor, as: Executor
  alias KyuubikiWeb.Orchestra.OperatorTaskIR, as: IR

  test "nonretryable task failure stays isolated but requires repair before retry" do
    {batch, execution} = failed_execution()
    assert {:ok, checkpoint} = Run.checkpoint_checked(batch, execution: execution)
    assert {:ok, plan} = Run.resume_plan(batch, checkpoint)
    assert plan["next_action"] == "resolve_incomplete_cases"
    assert plan["target_case_ids"] == ["case-b"]
    assert plan["repair_required_case_ids"] == ["case-b"]
    assert plan["retryable_case_ids"] == []
    assert plan["recovery_evidence_verified"] == true
    assert plan["automatic_replay_allowed"] == false
    assert plan["continuation_blocked"] == false
    assert plan["recovery_actions"] == ["rebuild_task_ir_and_recompute_digest"]
  end

  test "failure receipt identity reason and recovery drift cannot authorize recovery" do
    {batch, execution} = failed_execution()

    mutations =
      Enum.map(
        ~w(task_id task_digest operator_id reason_code schema_version failure_owner failure_stage),
        &put_in(execution, ["failure_receipts", Access.at(0), &1], "wrong")
      ) ++
        [
          put_in(execution, ["failure_receipts", Access.at(0), "case_id"], "case-a"),
          put_in(execution, ["failure_receipts", Access.at(0), "recovery", "retryable"], true),
          put_in(execution, ["failure_receipts", Access.at(0), "recovery", "retryable"], "false"),
          put_in(
            execution,
            ["failure_receipts", Access.at(0), "recovery", "safe_to_continue_other_tasks"],
            1
          ),
          put_in(
            execution,
            ["failure_receipts", Access.at(0), "recovery", "required_action"],
            "untrusted-action"
          ),
          Map.put(execution, "failure_receipts", []),
          Map.put(
            execution,
            "failure_receipts",
            execution["failure_receipts"] ++ execution["failure_receipts"]
          )
        ]

    for invalid <- mutations do
      assert_unverified(batch, invalid)
    end
  end

  test "failed rows must match their own task and failure receipt" do
    {batch, execution} = failed_execution()

    for field <- ~w(task_id task_digest operator_id error_code) do
      assert_unverified(batch, put_in(execution, ["results", Access.at(1), field], "wrong"))
    end

    assert_unverified(batch, put_in(execution, ["results", Access.at(1), "case_id"], "case-a"))
    assert_unverified(batch, put_in(execution, ["results", Access.at(1), "failure_receipt"], nil))
  end

  test "counter and partition contradictions cannot yield verified recovery advice" do
    {batch, execution} = failed_execution()

    for {key, value} <- [
          {"task_count", 3},
          {"attempted_count", 1},
          {"executed_count", 2},
          {"ok_count", 0},
          {"blocked_count", 1},
          {"error_count", 2},
          {"error_count", 1.0},
          {"skipped_count", 1},
          {"skipped_case_ids", ["case-b"]},
          {"failed_case_ids", ["case-a"]},
          {"failed_case_ids", []},
          {"error_codes", []},
          {"error_code_counts", %{}},
          {"readiness_counts", %{"executed" => 2}},
          {"status", "executed"}
        ] do
      assert_unverified(batch, Map.put(execution, key, value))
    end
  end

  test "checkpoint verification rechecks failure permissions and counters after rehash" do
    {batch, execution} = failed_execution()
    assert {:ok, checkpoint} = Run.checkpoint_checked(batch, execution: execution)

    for {path, value} <- [
          {["execution", "failure_receipts", Access.at(0), "task_id"], "wrong"},
          {["execution", "failure_receipts", Access.at(0), "recovery", "required_action"],
           "wrong"},
          {["execution", "failure_receipts", Access.at(0), "recovery", "retryable"], true},
          {["execution", "error_count"], 2},
          {["execution", "recovery_verified"], false},
          {["execution", "recovery_evidence"], nil},
          {["execution", "recovery_evidence"], []},
          {["execution", "recovery_evidence", "failed_tasks"], [nil]},
          {["execution", "recovery_evidence", "blocked_tasks"], %{}},
          {["execution", "recovery_evidence_error"], "contradiction"}
        ] do
      changed = checkpoint |> put_in(path, value) |> rehash()
      assert {:error, _} = Run.verify_checkpoint(batch, changed)
      assert {:error, _} = Run.resume_plan(batch, changed)
    end
  end

  test "control plane failures cannot claim retryability even with both receipt mirrors changed" do
    {batch, execution} = failed_execution()

    changed =
      execution
      |> put_in(["failure_receipts", Access.at(0), "recovery", "retryable"], true)
      |> put_in(["results", Access.at(1), "failure_receipt", "recovery", "retryable"], true)

    assert_unverified(batch, changed)
  end

  test "foreign run and incomplete failure evidence suppress untrusted recovery actions" do
    {batch, execution} = failed_execution()

    for invalid <- [
          Map.put(execution, "batch_digest", String.duplicate("0", 64)),
          Map.put(execution, "results", [hd(execution["results"])])
        ] do
      assert_unverified(batch, invalid)
    end
  end

  test "strict stop retains skipped cases without granting replay permission" do
    good = batch()
    bad = put_in(good, ["tasks", Access.at(0), "task_ir", "config", "tampered"], true)
    assert {:ok, execution} = Executor.execute_batch(bad, strict: true)
    assert {:ok, checkpoint} = Run.checkpoint_checked(bad, execution: execution)
    assert {:ok, plan} = Run.resume_plan(bad, checkpoint)
    assert plan["recovery_evidence_verified"] == true
    assert plan["target_case_ids"] == ~w(case-a case-b)
    assert plan["repair_required_case_ids"] == ["case-a"]
    assert plan["automatic_replay_allowed"] == false
  end

  test "successful and empty runs retain verifiable recovery partitions without result copies" do
    for good <- [batch(), Map.put(batch(), "tasks", [])] do
      assert {:ok, execution} = Executor.execute_batch(good)
      assert {:ok, checkpoint} = Run.checkpoint_checked(good, execution: execution)
      checkpoint = checkpoint |> Jason.encode!() |> Jason.decode!()
      assert {:ok, plan} = Run.resume_plan(good, checkpoint)
      assert plan["next_action"] == "archive"
      assert plan["recovery_evidence_verified"] == true
      assert plan["retryable_case_ids"] == []
      refute Map.has_key?(checkpoint["execution"], "results")
    end
  end

  test "caller supplied recovery proof is replaced by checked row evidence" do
    {batch, execution} = failed_execution()
    invalid = execution |> Map.put("recovery_verified", true) |> Map.put("recovery_evidence", %{})
    assert {:ok, checkpoint} = Run.checkpoint_checked(batch, execution: invalid)
    assert {:ok, plan} = Run.resume_plan(batch, checkpoint)
    assert plan["recovery_evidence_verified"] == true
    assert plan["retryable_case_ids"] == []
  end

  test "unverified diagnostic checkpoints round trip without recovering rejected actions" do
    {batch, execution} = failed_execution()
    invalid = put_in(execution, ["failure_receipts", Access.at(0), "task_id"], "secret-value")
    assert {:ok, checkpoint} = Run.checkpoint_checked(batch, execution: invalid)
    checkpoint = checkpoint |> Jason.encode!() |> Jason.decode!()
    assert checkpoint["execution"]["recovery_evidence_error"] == "failure_receipts missing task"
    assert {:ok, plan} = Run.resume_plan(batch, checkpoint)
    assert plan["recovery_evidence_verified"] == false
    assert plan["recovery_actions"] == []
    assert plan["target_case_ids"] == ["case-b"]

    for value <- [nil, "", [], %{}, false] do
      changed = checkpoint |> put_in(["execution", "recovery_evidence_error"], value) |> rehash()

      assert {:error, {:operator_task_checkpoint_evidence_invalid, field}} =
               Run.resume_plan(batch, changed)

      refute field =~ "secret-value"
    end
  end

  test "blocked task identity and required actions are checked independently from failures" do
    batch = batch()
    assert {:ok, execution} = Executor.execute_batch(batch)
    [first, second] = execution["results"]

    blocked =
      second
      |> Map.drop(["result"])
      |> Map.put("status", "blocked")
      |> Map.put("execution_readiness", %{
        "status" => "blocked",
        "ready_to_dispatch" => false,
        "required_action" => "resolve_package"
      })

    execution =
      Map.merge(execution, %{
        "status" => "blocked",
        "executed_count" => 1,
        "ok_count" => 1,
        "blocked_count" => 1,
        "readiness_counts" => %{"executed" => 1, "blocked" => 1},
        "results" => [first, blocked]
      })

    assert {:ok, checkpoint} = Run.checkpoint_checked(batch, execution: execution)
    assert {:ok, plan} = Run.resume_plan(batch, checkpoint)
    assert plan["recovery_evidence_verified"] == true
    assert plan["next_action"] == "resolve_blocked_cases"
    assert plan["recovery_actions"] == ["resolve_package"]

    for {path, value} <- [
          {["task_id"], "foreign"},
          {["task_digest"], "wrong"},
          {["failure_receipt"], %{}},
          {["error_code"], "unexpected"},
          {["execution_readiness", "status"], "executed"},
          {["execution_readiness", "ready_to_dispatch"], true},
          {["execution_readiness", "required_action"], " "}
        ] do
      assert_unverified(batch, put_in(execution, ["results", Access.at(1)] ++ path, value))
    end

    changed =
      checkpoint
      |> put_in(
        [
          "execution",
          "recovery_evidence",
          "blocked_tasks",
          Access.at(0),
          "readiness",
          "required_action"
        ],
        "untrusted"
      )
      |> rehash()

    assert {:error, _} = Run.resume_plan(batch, changed)
  end

  defp assert_unverified(batch, execution) do
    assert {:ok, checkpoint} = Run.checkpoint_checked(batch, execution: execution)
    assert {:ok, plan} = Run.resume_plan(batch, checkpoint)
    assert plan["recovery_evidence_verified"] == false
    assert plan["next_action"] == "resolve_incomplete_cases"
    refute plan["target_case_ids"] == []
    assert plan["retryable_case_ids"] == []
    assert plan["repair_required_case_ids"] == []
    assert plan["recovery_actions"] == []
    assert plan["automatic_replay_allowed"] == false
    assert plan["continuation_blocked"] == true
  end

  defp failed_execution do
    bad = put_in(batch(), ["tasks", Access.at(1), "task_ir", "config", "tampered"], true)
    {:ok, execution} = Executor.execute_batch(bad)
    {bad, execution}
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
