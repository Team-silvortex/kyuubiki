defmodule KyuubikiSdk.ModelResearchFrontier do
  @moduledoc "Verified cross-turn progression for model-assisted research jobs."

  alias KyuubikiSdk.Error
  alias KyuubikiSdk.ModelCollaboration
  alias KyuubikiSdk.ModelResearchExecution

  @frontier_schema "kyuubiki.model-research-frontier/v1"
  @submit_actions ["fem_submit", "workflow_submit_catalog", "workflow_submit_graph"]

  def schema_version, do: @frontier_schema

  def start(receipt, receipt_verifier) when is_function(receipt_verifier, 1) do
    with :ok <- validate_receipt(receipt),
         :ok <- verify_receipt(receipt, receipt_verifier),
         {:ok, record} <- last_record(receipt) do
      start_from_record(receipt, record)
    end
  end

  def start(_receipt, _receipt_verifier),
    do: validation_error("receipt verifier must be a one-argument function")

  def advance(current, receipt, frontier_verifier, receipt_verifier)
      when is_function(frontier_verifier, 1) and is_function(receipt_verifier, 1) do
    with :ok <- validate_frontier(current),
         :ok <- verify_frontier(current, frontier_verifier),
         :ok <- validate_receipt(receipt),
         :ok <- verify_receipt(receipt, receipt_verifier),
         :ok <- validate_identity(current, receipt),
         {:ok, expected} <- next_action(current),
         {:ok, record} <- last_record(receipt) do
      advance_from_record(current, receipt, record, expected)
    end
  end

  def advance(_current, _receipt, _frontier_verifier, _receipt_verifier),
    do: validation_error("frontier and receipt verifiers must be one-argument functions")

  def build_proposal(current, frontier_verifier) when is_function(frontier_verifier, 1) do
    with :ok <- validate_frontier(current),
         :ok <- verify_frontier(current, frontier_verifier),
         {:ok, action} <- next_action(current),
         {:ok, job_id} <- required_string(current, "job_id") do
      {:ok,
       %{
         "schema_version" => ModelCollaboration.proposal_schema_version(),
         "session_id" => current["session_id"],
         "summary" => "Advance verified research frontier with #{action}",
         "calls" => [
           %{
             "id" => "frontier-#{current["transition_count"] + 1}-#{action}",
             "action" => action,
             "payload" => %{"job_id" => job_id},
             "reason" => "Use the job identifier retained from verified execution evidence"
           }
         ]
       }}
    end
  end

  def build_proposal(_current, _frontier_verifier),
    do: validation_error("frontier verifier must be a one-argument function")

  defp start_from_record(%{"status" => "failed"} = receipt, record),
    do: {:ok, blocked_frontier(receipt, record, nil, 1)}

  defp start_from_record(%{"status" => "completed"} = receipt, record) do
    cond do
      record["action"] not in @submit_actions ->
        validation_error("initial research receipt must end with a supported job submission")

      true ->
        case get_in(record, ["output", "job", "job_id"]) do
          job_id when is_binary(job_id) and job_id != "" ->
            {:ok,
             frontier(receipt, record,
               stage: "waiting_for_job",
               job_id: job_id,
               next_action: "job_wait",
               transition_count: 1
             )}

          _ ->
            validation_error("job submission receipt did not contain job.job_id")
        end
    end
  end

  defp advance_from_record(current, %{"status" => "failed"} = receipt, record, _expected) do
    {:ok,
     blocked_frontier(
       receipt,
       record,
       current["job_id"],
       current["transition_count"] + 1
     )}
  end

  defp advance_from_record(current, receipt, record, expected) do
    cond do
      record["action"] != expected ->
        validation_error(
          "research receipt ended with #{inspect(record["action"])}; frontier requires #{expected}"
        )

      record["job_id"] != current["job_id"] ->
        validation_error("research receipt job_id does not match frontier binding")

      expected == "job_wait" ->
        advance_wait(current, receipt, record)

      expected == "result_fetch" ->
        {:ok,
         frontier(receipt, record,
           stage: "ready_to_validate",
           job_id: current["job_id"],
           next_action: nil,
           transition_count: current["transition_count"] + 1
         )}

      true ->
        validation_error("unsupported frontier next action: #{expected}")
    end
  end

  defp advance_wait(current, receipt, record) do
    case get_in(record, ["output", "terminal", "job", "status"]) do
      "completed" = status ->
        {:ok,
         frontier(receipt, record,
           stage: "ready_to_fetch_result",
           job_id: current["job_id"],
           next_action: "result_fetch",
           transition_count: current["transition_count"] + 1,
           job_status: status
         )}

      status when status in ["failed", "cancelled"] ->
        {:ok,
         frontier(receipt, record,
           stage: "blocked",
           job_id: current["job_id"],
           next_action: nil,
           transition_count: current["transition_count"] + 1,
           job_status: status,
           blocking_reason: "job reached terminal status #{status}"
         )}

      nil ->
        validation_error("job_wait receipt did not contain terminal.job.status")

      status ->
        validation_error("job_wait returned non-terminal status #{inspect(status)}")
    end
  end

  defp frontier(receipt, record, opts) do
    %{
      "schema_version" => @frontier_schema,
      "session_id" => receipt["session_id"],
      "workflow_id" => receipt["workflow_id"],
      "stage" => Keyword.fetch!(opts, :stage),
      "job_id" => Keyword.get(opts, :job_id),
      "next_action" => Keyword.get(opts, :next_action),
      "transition_count" => Keyword.fetch!(opts, :transition_count),
      "evidence" => %{
        "approval_id" => receipt["approval_id"],
        "action" => record["action"],
        "record_index" => record["index"],
        "authority" => record["authority"],
        "job_status" => Keyword.get(opts, :job_status)
      },
      "blocking_reason" => Keyword.get(opts, :blocking_reason)
    }
  end

  defp blocked_frontier(receipt, record, job_id, transition_count) do
    frontier(receipt, record,
      stage: "blocked",
      job_id: job_id,
      next_action: nil,
      transition_count: transition_count,
      blocking_reason: record["error"] || "research execution failed"
    )
  end

  defp validate_receipt(receipt) when is_map(receipt) do
    cond do
      receipt["schema_version"] != ModelResearchExecution.receipt_schema_version() or
          receipt["execution_authority"] != "kyuubiki-headless-sdk" ->
        validation_error("unsupported or untrusted research execution receipt")

      not present_string?(receipt["session_id"]) or
        not present_string?(receipt["workflow_id"]) or
        not is_list(receipt["records"]) or receipt["records"] == [] ->
        validation_error("research execution receipt is incomplete")

      receipt["status"] not in ["completed", "failed"] ->
        validation_error("research execution receipt has an unsupported status")

      not valid_final_record?(receipt["status"], List.last(receipt["records"])) ->
        validation_error("research execution receipt has an invalid final record")

      true ->
        :ok
    end
  end

  defp validate_receipt(_receipt),
    do: validation_error("research execution receipt must be a JSON object")

  defp validate_frontier(current) when is_map(current) do
    cond do
      current["schema_version"] != @frontier_schema or
        not present_string?(current["session_id"]) or
        not present_string?(current["workflow_id"]) or
        not is_integer(current["transition_count"]) or current["transition_count"] <= 0 ->
        validation_error("research frontier is incomplete or uses an unsupported schema")

      not valid_frontier_state?(current) ->
        validation_error("research frontier stage and next action are inconsistent")

      true ->
        :ok
    end
  end

  defp validate_frontier(_current),
    do: validation_error("research frontier must be a JSON object")

  defp verify_receipt(receipt, verifier) do
    case verifier.(receipt) do
      true ->
        :ok

      :ok ->
        :ok

      {:ok, _evidence} ->
        :ok

      {:error, reason} ->
        validation_error("caller receipt verifier rejected receipt: #{inspect(reason)}")

      _ ->
        validation_error("caller receipt verifier rejected research execution receipt")
    end
  end

  defp verify_frontier(current, verifier) do
    case verifier.(current) do
      true ->
        :ok

      :ok ->
        :ok

      {:ok, _evidence} ->
        :ok

      {:error, reason} ->
        validation_error("caller frontier verifier rejected frontier: #{inspect(reason)}")

      _ ->
        validation_error("caller frontier verifier rejected research frontier")
    end
  end

  defp validate_identity(current, receipt) do
    if receipt["session_id"] == current["session_id"] and
         receipt["workflow_id"] == current["workflow_id"],
       do: :ok,
       else: validation_error("research receipt does not match frontier session and workflow")
  end

  defp next_action(current) do
    case current["next_action"] do
      value when is_binary(value) and value != "" -> {:ok, value}
      _ -> validation_error("research frontier has no executable next action")
    end
  end

  defp required_string(value, key) do
    case value[key] do
      item when is_binary(item) and item != "" -> {:ok, item}
      _ -> validation_error("research frontier has no bound #{key}")
    end
  end

  defp last_record(receipt) do
    case List.last(receipt["records"]) do
      record when is_map(record) -> {:ok, record}
      _ -> validation_error("research execution receipt has an invalid final record")
    end
  end

  defp present_string?(value), do: is_binary(value) and String.trim(value) != ""

  defp valid_final_record?("completed", record) when is_map(record),
    do:
      is_nil(record["error"]) and not is_nil(record["output"]) and is_binary(record["authority"])

  defp valid_final_record?("failed", record) when is_map(record),
    do: is_binary(record["error"])

  defp valid_final_record?(_status, _record), do: false

  defp valid_frontier_state?(current) do
    has_job = present_string?(current["job_id"])

    case current["stage"] do
      "waiting_for_job" ->
        has_job and current["next_action"] == "job_wait" and is_nil(current["blocking_reason"])

      "ready_to_fetch_result" ->
        has_job and current["next_action"] == "result_fetch" and
          is_nil(current["blocking_reason"])

      "ready_to_validate" ->
        has_job and is_nil(current["next_action"]) and is_nil(current["blocking_reason"])

      "blocked" ->
        is_nil(current["next_action"]) and present_string?(current["blocking_reason"])

      _ ->
        false
    end
  end

  defp validation_error(message), do: {:error, Error.model_research_execution([message])}
end
