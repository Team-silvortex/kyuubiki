defmodule KyuubikiWeb.Orchestra.OperatorTaskBatchRecoveryEvidence do
  @moduledoc false

  alias KyuubikiWeb.CanonicalJson
  alias KyuubikiWeb.Orchestra.OperatorTaskCompletion
  alias KyuubikiWeb.Orchestra.OperatorTaskFailure
  @identity ~w(case_id task_id task_digest operator_id program_id)
  @receipt_fields ~w(schema_version failure_owner failure_stage case_id task_id task_digest
                     operator_id program_id reason_code message recovery)
  @readiness_fields ~w(status ready_to_dispatch required_action)

  def build(batch, raw, summary, completion) do
    known = identities(batch)
    rows = array(raw["results"], "results")

    failed =
      for row <- rows, is_map(row), row["status"] == "error" do
        expected = row_identity(row, known)
        receipt = failure(expected, row["error_code"], row["error"], row["failure_receipt"])
        readiness = object(row["execution_readiness"], "failed readiness")
        equal(readiness["status"], "blocked", "failed readiness.status")
        equal(readiness["ready_to_dispatch"], false, "failed readiness.ready_to_dispatch")

        if receipt["failure_owner"] == "agent_runtime" do
          if OperatorTaskFailure.validate_readiness(readiness, receipt) != :ok,
            do: invalid("failed readiness")

          case OperatorTaskCompletion.agent_receipt(expected, row["result"]) do
            {:ok, %{"status" => "failed", "failure_receipt" => nested}} when is_map(nested) ->
              equal(receipt_digest(nested), receipt_digest(receipt), "agent failure mirror")

            _ ->
              invalid("agent failure result")
          end
        end

        expected
        |> Map.put("error_code", row["error_code"])
        |> Map.put("receipt_digest", receipt_digest(receipt))
      end

    blocked =
      for row <- rows, is_map(row), row["status"] == "blocked" do
        expected = row_identity(row, known)

        if row["failure_receipt"] != nil or row["error_code"] != nil or row["error"] != nil,
          do: invalid("blocked failure fields")

        readiness = blocked_readiness(row["execution_readiness"])
        Map.put(expected, "readiness", readiness)
      end

    proof = %{"failed_tasks" => failed, "blocked_tasks" => blocked}
    validate(batch, summary, completion, proof)
    %{"recovery_evidence" => proof, "recovery_verified" => true, "recovery_evidence_error" => nil}
  catch
    {:recovery_invalid, field} ->
      %{
        "recovery_evidence" => nil,
        "recovery_verified" => false,
        "recovery_evidence_error" => field
      }
  end

  def verify(_batch, nil, _completion), do: :ok

  def verify(batch, summary, completion) do
    case summary["recovery_evidence"] do
      nil ->
        equal(summary["recovery_verified"], false, "recovery_verified")
        text(summary["recovery_evidence_error"], "recovery_evidence_error")

      proof ->
        validate(batch, summary, completion, proof)
        equal(summary["recovery_verified"], true, "recovery_verified")
        equal(summary["recovery_evidence_error"], nil, "recovery_evidence_error")
    end

    :ok
  catch
    {:recovery_invalid, field} ->
      {:error, {:operator_task_checkpoint_evidence_invalid, "execution.#{field}"}}
  end

  def advice(%{"recovery_verified" => true, "recovery_evidence" => proof} = summary) do
    receipts = Map.new(summary["failure_receipts"], &{&1["task_id"], &1})
    failed = proof["failed_tasks"]

    %{
      "recovery_evidence_verified" => true,
      "automatic_replay_allowed" => false,
      "retryable_case_ids" =>
        for(
          row <- failed,
          receipts[row["task_id"]]["recovery"]["retryable"] == true,
          do: row["case_id"]
        ),
      "repair_required_case_ids" =>
        for(
          row <- failed,
          receipts[row["task_id"]]["recovery"]["retryable"] == false,
          do: row["case_id"]
        ),
      "continuation_blocked" =>
        Enum.any?(
          failed,
          &(receipts[&1["task_id"]]["recovery"]["safe_to_continue_other_tasks"] == false)
        )
    }
  end

  def advice(_summary) do
    %{
      "recovery_evidence_verified" => false,
      "automatic_replay_allowed" => false,
      "retryable_case_ids" => [],
      "repair_required_case_ids" => [],
      "continuation_blocked" => true
    }
  end

  defp validate(batch, summary, completion, proof) do
    equal(completion["batch_bound"], true, "recovery batch binding")
    object(proof, "recovery_evidence")
    known = identities(batch)
    failed = array(proof["failed_tasks"], "recovery_evidence.failed_tasks")
    blocked = array(proof["blocked_tasks"], "recovery_evidence.blocked_tasks")
    completed = completion["completed_tasks"]
    skipped = array(summary["skipped_case_ids"], "skipped_case_ids")
    receipts = array(summary["failure_receipts"], "failure_receipts")
    Enum.each(receipts, &object(&1, "failure_receipts entry"))
    by_task = Map.new(receipts, &{&1["task_id"], &1})
    equal(map_size(by_task), length(receipts), "failure_receipts duplicate task")
    equal(length(receipts), length(failed), "failure_receipts coverage")

    for row <- failed do
      expected = row_identity(row, known)
      receipt = object(by_task[row["task_id"]], "failure_receipts missing task")
      receipt = failure(expected, row["error_code"], receipt["message"], receipt)
      equal(row["receipt_digest"], receipt_digest(receipt), "failure_receipts mirror")
    end

    for row <- blocked do
      row_identity(row, known)
      blocked_readiness(row["readiness"])
    end

    all = completed ++ failed ++ blocked
    ids = Enum.map(all, & &1["case_id"]) ++ skipped
    equal(length(ids), MapSet.size(MapSet.new(ids)), "recovery partitions overlap")
    equal(MapSet.new(ids), MapSet.new(Map.keys(known)), "recovery partition coverage")
    same_ids(summary["failed_case_ids"], Enum.map(failed, & &1["case_id"]), "failed_case_ids")

    same_ids(
      summary["blocked_readiness_case_ids"],
      Enum.map(blocked, & &1["case_id"]),
      "blocked_readiness_case_ids"
    )

    for {key, expected} <- [
          {"task_count", map_size(known)},
          {"attempted_count", length(all)},
          {"executed_count", length(completed)},
          {"ok_count", length(completed)},
          {"blocked_count", length(blocked)},
          {"error_count", length(failed)},
          {"skipped_count", length(skipped)}
        ],
        do: equal(summary[key], expected, key)

    equal(completion["result_count"], length(all), "result_count")
    codes = Enum.frequencies_by(failed, & &1["error_code"])
    equal(summary["error_code_counts"], codes, "error_code_counts")
    same_ids(summary["error_codes"], Map.keys(codes), "error_codes")

    readiness =
      Enum.map(completed, fn _ -> "executed" end) ++
        Enum.map(failed, fn _ -> "blocked" end) ++ Enum.map(blocked, & &1["readiness"]["status"])

    equal(summary["readiness_counts"], Enum.frequencies(readiness), "readiness_counts")
    actions = Enum.map(blocked, & &1["readiness"]["required_action"]) |> Enum.uniq()
    same_ids(summary["readiness_recovery_actions"], actions, "readiness_recovery_actions")

    equal(
      summary["status"],
      status(length(completed), length(blocked), length(failed), length(skipped)),
      "status"
    )
  end

  defp failure(expected, code, message, receipt) do
    object(receipt, "failure_receipt")
    text(code, "failure_receipt.reason_code")

    if Map.has_key?(receipt, "case_id"),
      do: equal(receipt["case_id"], expected["case_id"], "failure_receipt.case_id")

    for field <- ~w(task_id task_digest operator_id),
        do: equal(receipt[field], expected[field], "failure_receipt.#{field}")

    if Map.has_key?(receipt, "program_id"),
      do: equal(receipt["program_id"], expected["program_id"], "failure_receipt.program_id")

    case {receipt["schema_version"], receipt["failure_owner"]} do
      {"kyuubiki.agent-operator-task-failure/v1", "agent_runtime"} ->
        case OperatorTaskFailure.validate(expected, receipt, code, message) do
          {:ok, _} -> :ok
          _ -> invalid("failure_receipt agent contract")
        end

      {"kyuubiki.control-plane-operator-task-failure/v1", "control_plane"} ->
        equal(receipt["reason_code"], code, "failure_receipt.reason_code")
        equal(receipt["message"], message, "failure_receipt.message")
        text(message, "failure_receipt.message")
        text(receipt["failure_stage"], "failure_receipt.failure_stage")
        recovery = object(receipt["recovery"], "failure_receipt.recovery")
        equal(recovery["retryable"], false, "control plane recovery.retryable")

        if !is_boolean(recovery["safe_to_continue_other_tasks"]),
          do: invalid("recovery.safe_to_continue_other_tasks")

        text(recovery["required_action"], "recovery.required_action")

      _ ->
        invalid("failure_receipt owner/schema")
    end

    receipt
  end

  defp receipt_digest(receipt) do
    receipt
    |> Map.take(@receipt_fields)
    |> Map.update!(
      "recovery",
      &Map.take(&1, ~w(retryable safe_to_continue_other_tasks required_action))
    )
    |> CanonicalJson.encode!()
    |> then(&:crypto.hash(:sha256, &1))
    |> Base.encode16(case: :lower)
  end

  defp row_identity(row, known) do
    object(row, "recovery row")
    expected = object(known[row["case_id"]], "recovery case_id")

    for field <- @identity -- ["program_id"],
        do: equal(row[field], expected[field], "recovery row.#{field}")

    if Map.has_key?(row, "program_id"),
      do: equal(row["program_id"], expected["program_id"], "recovery row.program_id")

    expected
  end

  defp blocked_readiness(value) do
    object(value, "blocked readiness")

    if value["status"] not in ~w(blocked ready_for_package_resolution),
      do: invalid("blocked readiness.status")

    equal(value["ready_to_dispatch"], false, "blocked readiness.ready_to_dispatch")
    text(value["required_action"], "blocked readiness.required_action")
    Map.take(value, @readiness_fields)
  end

  defp identities(batch) do
    Map.new(batch["tasks"], fn entry ->
      task = entry["task_ir"]

      {entry["case_id"],
       %{
         "case_id" => entry["case_id"],
         "task_id" => task["task_id"],
         "task_digest" => get_in(task, ["integrity", "task_digest"]),
         "operator_id" => get_in(task, ["operator", "id"]),
         "program_id" => get_in(task, ["execution_program", "program_id"])
       }}
    end)
  end

  defp status(_ok, 0, 0, 0), do: "executed"
  defp status(_ok, _blocked, 0, _skipped), do: "blocked"
  defp status(0, 0, _errors, _skipped), do: "failed"
  defp status(_ok, _blocked, _errors, _skipped), do: "partial"

  defp same_ids(actual, expected, field) do
    actual = array(actual, field)
    actual_ids = MapSet.new(actual)
    equal(length(actual), length(expected), field)
    equal(MapSet.size(actual_ids), length(actual), field)
    equal(actual_ids, MapSet.new(expected), field)
  end

  defp array(value, _field) when is_list(value), do: value
  defp array(_value, field), do: invalid(field)
  defp object(value, _field) when is_map(value) and not is_struct(value), do: value
  defp object(_value, field), do: invalid(field)

  defp text(value, field) do
    if !is_binary(value) or String.trim(value) == "", do: invalid(field)
  end

  defp equal(actual, expected, _field) when actual === expected, do: :ok
  defp equal(_actual, _expected, field), do: invalid(field)
  defp invalid(field), do: throw({:recovery_invalid, field})
end
