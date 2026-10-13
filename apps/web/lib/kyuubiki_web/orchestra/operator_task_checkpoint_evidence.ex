defmodule KyuubikiWeb.Orchestra.OperatorTaskCheckpointEvidence do
  @moduledoc false

  alias KyuubikiWeb.Orchestra.OperatorTaskBatchCompletion, as: Completion
  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRecoveryEvidence, as: Recovery
  @identity ~w(case_id task_id task_digest operator_id program_id)
  @preparation_fields ~w(run_id run_phase batch_digest started_at finished_at task_count
                        verified_count error_count error_codes error_code_counts failure_receipts)

  def preparation(_batch, nil, _digest), do: {:ok, nil}

  def preparation(batch, result, digest) do
    rows = array(result["summaries"], "preparation.summaries")

    evidence =
      Enum.map(rows, fn row ->
        object(row, "preparation.summaries entry")

        row
        |> Map.take(@identity ++ ~w(status error_code))
        |> Map.put("has_error", row["error"] != nil or row["failure_receipt"] != nil)
      end)

    summary = result |> Map.take(@preparation_fields) |> Map.put("task_evidence", evidence)
    validate_preparation(batch, summary, digest)
    {:ok, summary}
  catch
    {:checkpoint_evidence_invalid, reason} -> {:error, reason}
  end

  def execution(batch, result, digest) do
    bound? = result["batch_digest"] == digest and result["run_phase"] == "execute"

    completed =
      if bound?,
        do: MapSet.new(Completion.completed_case_ids(batch, result)),
        else: MapSet.new()

    rows = Enum.filter(identities(batch), &MapSet.member?(completed, &1["case_id"]))

    %{
      "batch_bound" => bound?,
      "result_count" => if(is_list(result["results"]), do: length(result["results"])),
      "completed_tasks" => rows
    }
  end

  def execution_state(batch, result, digest, evidence) do
    rows = evidence["completed_tasks"]
    completed = MapSet.new(rows, & &1["case_id"])
    ids = Enum.map(batch["tasks"], & &1["case_id"])

    complete? =
      result["recovery_verified"] == true and evidence["batch_bound"] == true and
        evidence["result_count"] === length(ids) and
        length(rows) == length(ids) and
        Completion.complete_metadata?(length(ids), result, digest)

    incomplete = Enum.reject(ids, &MapSet.member?(completed, &1))
    incomplete = if incomplete == [] and not complete?, do: ids, else: incomplete
    {complete?, incomplete}
  end

  def verify(batch, checkpoint, digest) do
    validate_preparation(batch, checkpoint["preparation"], digest)
    validate_execution(batch, checkpoint["execution"], digest)
    :ok
  catch
    {:checkpoint_evidence_invalid, reason} -> {:error, reason}
  end

  defp validate_preparation(_batch, nil, _digest), do: :ok

  defp validate_preparation(batch, summary, digest) do
    equal(summary["run_phase"], "prepare", "preparation.run_phase")
    equal(summary["batch_digest"], digest, "preparation.batch_digest")
    rows = array(summary["task_evidence"], "preparation.task_evidence")
    known = Map.new(identities(batch), &{&1["case_id"], &1})
    unique_rows(rows, known, "preparation.task_evidence", false)
    equal(length(rows), map_size(known), "preparation.task_evidence coverage")

    for row <- rows do
      case row["status"] do
        "verified" ->
          equal(row["has_error"], false, "preparation.task_evidence.has_error")
          equal(row["error_code"], nil, "preparation.task_evidence.error_code")
          equal(row["program_id"], known[row["case_id"]]["program_id"], "preparation.program_id")

        "error" ->
          if !is_binary(row["error_code"]) or row["error_code"] == "",
            do: invalid("preparation.task_evidence.error_code")

        _ ->
          invalid("preparation.task_evidence.status")
      end
    end

    errors = Enum.filter(rows, &(&1["status"] == "error"))
    equal(summary["task_count"], length(rows), "preparation.task_count")
    equal(summary["verified_count"], length(rows) - length(errors), "preparation.verified_count")
    equal(summary["error_count"], length(errors), "preparation.error_count")
    codes = Enum.frequencies_by(errors, & &1["error_code"])
    equal(summary["error_code_counts"], codes, "preparation.error_code_counts")
    actual_codes = array(summary["error_codes"], "preparation.error_codes")
    equal(Enum.sort(actual_codes), Enum.sort(Map.keys(codes)), "preparation.error_codes")
    receipts = array(summary["failure_receipts"], "preparation.failure_receipts")
    unique_rows(receipts, known, "preparation.failure_receipts", false)
    equal(length(receipts), length(errors), "preparation.failure_receipts coverage")
    failed = Map.new(errors, &{&1["case_id"], &1["error_code"]})

    for receipt <- receipts do
      if !Map.has_key?(failed, receipt["case_id"]),
        do: invalid("preparation.failure_receipts case")

      equal(
        receipt["reason_code"],
        failed[receipt["case_id"]],
        "preparation.failure_receipts reason"
      )
    end
  end

  defp validate_execution(_batch, nil, _digest), do: :ok

  defp validate_execution(batch, summary, digest) do
    evidence = object(summary["completion_evidence"], "execution.completion_evidence")
    rows = array(evidence["completed_tasks"], "execution.completion_evidence.completed_tasks")
    known = Map.new(identities(batch), &{&1["case_id"], &1})
    unique_rows(rows, known, "execution.completion_evidence.completed_tasks", true)
    count = evidence["result_count"]

    if not ((is_nil(count) and rows == []) or (is_integer(count) and count >= length(rows))),
      do: invalid("execution.completion_evidence.result_count")

    equal(
      evidence["batch_bound"],
      summary["batch_digest"] == digest and summary["run_phase"] == "execute",
      "execution.completion_evidence.batch_bound"
    )

    if rows != [] do
      equal(summary["run_phase"], "execute", "execution.run_phase")
      equal(summary["batch_digest"], digest, "execution.batch_digest")
    end

    unresolved =
      ~w(failed_case_ids skipped_case_ids blocked_readiness_case_ids)
      |> Enum.flat_map(&Map.get(summary, &1, []))
      |> MapSet.new()

    if Enum.any?(rows, &MapSet.member?(unresolved, &1["case_id"])),
      do: invalid("execution.completion_evidence unresolved overlap")

    case Recovery.verify(batch, summary, evidence) do
      :ok -> :ok
      {:error, reason} -> throw({:checkpoint_evidence_invalid, reason})
    end

    {complete?, incomplete} = execution_state(batch, summary, digest, evidence)
    equal(summary["completion_verified"], complete?, "execution.completion_verified")
    equal(summary["incomplete_case_ids"], incomplete, "execution.incomplete_case_ids")
  end

  defp identities(batch) do
    Enum.map(batch["tasks"], fn entry ->
      task = entry["task_ir"]

      %{
        "case_id" => entry["case_id"],
        "task_id" => task["task_id"],
        "task_digest" => get_in(task, ["integrity", "task_digest"]),
        "operator_id" => get_in(task, ["operator", "id"]),
        "program_id" => get_in(task, ["execution_program", "program_id"])
      }
    end)
  end

  defp unique_rows(rows, known, path, program?) do
    Enum.reduce(rows, MapSet.new(), fn row, seen ->
      object(row, path)
      id = row["case_id"]
      if !Map.has_key?(known, id) or MapSet.member?(seen, id), do: invalid(path)
      fields = if program?, do: @identity, else: @identity -- ["program_id"]
      for field <- fields, do: equal(row[field], known[id][field], "#{path}.#{field}")
      MapSet.put(seen, id)
    end)
  end

  defp array(value, _path) when is_list(value), do: value
  defp array(_value, path), do: invalid(path)
  defp object(value, _path) when is_map(value) and not is_struct(value), do: value
  defp object(_value, path), do: invalid(path)
  defp equal(actual, expected, _path) when actual === expected, do: :ok
  defp equal(_actual, _expected, path), do: invalid(path)

  defp invalid(path),
    do: throw({:checkpoint_evidence_invalid, {:operator_task_checkpoint_evidence_invalid, path}})
end
