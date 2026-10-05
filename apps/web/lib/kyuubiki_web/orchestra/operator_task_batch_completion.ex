defmodule KyuubikiWeb.Orchestra.OperatorTaskBatchCompletion do
  @moduledoc false

  alias KyuubikiWeb.Orchestra.OperatorTaskCompletion

  def completed_case_ids(batch, execution) do
    rows = Map.get(execution, "results", [])
    rows = if is_list(rows), do: Enum.filter(rows, &is_map/1), else: []
    by_case = Enum.group_by(rows, & &1["case_id"])

    unresolved =
      (List.wrap(execution["failed_case_ids"]) ++ List.wrap(execution["skipped_case_ids"]))
      |> MapSet.new()

    Enum.flat_map(Map.get(batch, "tasks", []), fn
      %{"case_id" => id, "task_ir" => task} when is_binary(id) and is_map(task) ->
        case by_case[id] do
          [row] ->
            if not MapSet.member?(unresolved, id) and completed_row?(row, task),
              do: [id],
              else: []

          _ ->
            []
        end

      _ ->
        []
    end)
    |> Enum.uniq()
  end

  def complete?(batch, execution, digest, completed) do
    tasks = Map.get(batch, "tasks", [])
    rows = Map.get(execution, "results", [])
    count = length(tasks)

    execution["task_count"] == count and execution["executed_count"] == count and
      execution["ok_count"] == count and execution["error_count"] == 0 and
      Map.get(execution, "attempted_count", count) == count and
      Map.get(execution, "blocked_count", 0) == 0 and
      Map.get(execution, "skipped_count", 0) == 0 and
      Map.get(execution, "batch_digest", digest) == digest and
      Map.get(execution, "run_phase", "execute") == "execute" and
      Map.get(execution, "status", "executed") == "executed" and
      Map.get(execution, "error_code_counts", %{}) == %{} and
      Map.get(execution, "readiness_counts", %{"executed" => count}) ==
        if(count == 0, do: %{}, else: %{"executed" => count}) and
      Enum.all?(
        ~w(failed_case_ids skipped_case_ids error_codes failure_receipts),
        &(Map.get(execution, &1, []) == [])
      ) and
      is_list(rows) and length(rows) == count and
      length(completed) == count
  end

  defp completed_row?(%{"status" => "ok", "result" => result} = row, task)
       when is_map(result) do
    expected = %{
      "task_id" => task["task_id"],
      "task_digest" => get_in(task, ["integrity", "task_digest"]),
      "operator_id" => get_in(task, ["operator", "id"]),
      "program_id" => get_in(task, ["execution_program", "program_id"])
    }

    Enum.all?(~w(task_id task_digest operator_id), &(row[&1] == expected[&1])) and
      Map.get(row, "program_id", expected["program_id"]) == expected["program_id"] and
      is_nil(row["error"]) and is_nil(row["error_code"]) and is_nil(row["failure_receipt"]) and
      OperatorTaskCompletion.executed_readiness?(row["execution_readiness"]) and
      completed_result?(expected, result)
  end

  defp completed_row?(_row, _task), do: false

  defp completed_result?(expected, %{"operator_task_ir_status" => _status} = result) do
    case OperatorTaskCompletion.agent_receipt(expected, result) do
      {:ok, %{"status" => "executed"}} -> true
      _ -> false
    end
  end

  defp completed_result?(_expected, _result), do: true
end
