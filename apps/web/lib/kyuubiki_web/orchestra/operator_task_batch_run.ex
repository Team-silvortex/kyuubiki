defmodule KyuubikiWeb.Orchestra.OperatorTaskBatchRun do
  @moduledoc """
  Stable run metadata for operator task batches.

  The execution engine may be local, agent-backed, or distributed later; this
  metadata gives every batch run a replayable identity before persistence exists.
  """

  alias KyuubikiWeb.CanonicalJson

  @digest_fields [
    "quality_execution_batch_contract",
    "operator_id",
    "task_count",
    "case_index",
    "tasks"
  ]

  @spec batch_digest(map()) :: String.t()
  def batch_digest(batch) when is_map(batch) do
    batch
    |> Map.take(@digest_fields)
    |> digest_term()
  end

  @spec run_id(map(), String.t()) :: String.t()
  def run_id(batch, phase) when is_map(batch) and is_binary(phase) do
    suffix =
      batch
      |> batch_digest()
      |> String.slice(0, 16)

    "operator-task-batch:#{phase}:#{suffix}"
  end

  @spec start(map(), String.t(), keyword()) :: map()
  def start(batch, phase, opts \\ []) when is_map(batch) and is_binary(phase) do
    started_at = timestamp(opts, :started_at)

    %{
      "run_id" => Keyword.get(opts, :run_id, run_id(batch, phase)),
      "run_phase" => phase,
      "batch_digest" => batch_digest(batch),
      "digest_algorithm" => "sha256",
      "started_at" => started_at
    }
  end

  @spec finish(map(), map(), keyword()) :: map()
  def finish(result, run, opts \\ []) when is_map(result) and is_map(run) do
    finished_at = timestamp(opts, :finished_at)

    result
    |> Map.merge(run)
    |> Map.put("finished_at", finished_at)
  end

  @spec checkpoint(map(), keyword()) :: map()
  def checkpoint(batch, opts \\ []) when is_map(batch) and is_list(opts) do
    preparation = Keyword.get(opts, :preparation)
    execution = Keyword.get(opts, :execution)
    created_at = timestamp(opts, :created_at)
    digest = batch_digest(batch)

    manifest =
      %{
        "operator_task_batch_checkpoint_contract" => "kyuubiki.operator_task_batch_checkpoint/v1",
        "checkpoint_id" => Keyword.get(opts, :checkpoint_id, checkpoint_id(digest, created_at)),
        "batch_digest" => digest,
        "digest_algorithm" => "sha256",
        "created_at" => created_at,
        "quality_execution_batch_contract" => Map.get(batch, "quality_execution_batch_contract"),
        "task_count" => task_count(batch),
        "case_index" => case_index(batch),
        "preparation" => checkpoint_summary(preparation),
        "execution" => checkpoint_summary(execution),
        "resume_policy" => resume_policy(preparation, execution)
      }

    Map.put(manifest, "checkpoint_digest", checkpoint_digest(manifest))
  end

  @spec verify_checkpoint(map(), map()) :: {:ok, map()} | {:error, term()}
  def verify_checkpoint(batch, checkpoint) when is_map(batch) and is_map(checkpoint) do
    expected_batch_digest = batch_digest(batch)
    expected_checkpoint_digest = checkpoint_digest(checkpoint)

    with :ok <- verify_checkpoint_contract(checkpoint),
         :ok <- verify_field(checkpoint, "batch_digest", expected_batch_digest),
         :ok <- verify_field(checkpoint, "checkpoint_digest", expected_checkpoint_digest),
         :ok <- verify_field(checkpoint, "task_count", task_count(batch)),
         :ok <- verify_field(checkpoint, "case_index", case_index(batch)) do
      {:ok,
       %{
         "operator_task_batch_checkpoint_verification_contract" =>
           "kyuubiki.operator_task_batch_checkpoint_verification/v1",
         "status" => "verified",
         "batch_digest" => expected_batch_digest,
         "checkpoint_digest" => expected_checkpoint_digest,
         "resume_policy" => Map.get(checkpoint, "resume_policy", %{})
       }}
    end
  end

  def verify_checkpoint(_batch, _checkpoint),
    do: {:error, :invalid_operator_task_batch_checkpoint}

  @spec resume_plan(map(), map()) :: {:ok, map()} | {:error, term()}
  def resume_plan(batch, checkpoint) when is_map(batch) and is_map(checkpoint) do
    with {:ok, verification} <- verify_checkpoint(batch, checkpoint) do
      policy = Map.get(checkpoint, "resume_policy", %{})
      next_action = Map.get(policy, "next_action", "prepare")

      {:ok,
       %{
         "operator_task_batch_resume_plan_contract" =>
           "kyuubiki.operator_task_batch_resume_plan/v1",
         "plan_id" => plan_id(checkpoint, next_action),
         "status" => Map.get(policy, "status", "draft"),
         "next_action" => next_action,
         "batch_digest" => verification["batch_digest"],
         "checkpoint_digest" => verification["checkpoint_digest"],
         "target_case_ids" => target_case_ids(batch, checkpoint, next_action),
         "blocked_case_ids" => blocked_case_ids(batch, checkpoint, next_action),
         "resume_policy" => policy
       }}
    end
  end

  def resume_plan(_batch, _checkpoint), do: {:error, :invalid_operator_task_batch_checkpoint}

  defp timestamp(opts, key) do
    case Keyword.get(opts, key) do
      %DateTime{} = value -> DateTime.to_iso8601(value)
      value when is_binary(value) and value != "" -> value
      _ -> DateTime.utc_now(:second) |> DateTime.to_iso8601()
    end
  end

  defp digest_term(term) do
    term
    |> json_safe()
    |> CanonicalJson.encode!()
    |> then(&:crypto.hash(:sha256, &1))
    |> Base.encode16(case: :lower)
  end

  defp json_safe(value) when is_map(value) do
    value
    |> Map.to_list()
    |> Enum.map(fn {key, nested} -> {to_string(key), json_safe(nested)} end)
    |> Map.new()
  end

  defp json_safe(value) when is_list(value) do
    if Keyword.keyword?(value) do
      value
      |> Enum.map(fn {key, nested} -> {to_string(key), json_safe(nested)} end)
      |> Map.new()
    else
      Enum.map(value, &json_safe/1)
    end
  end

  defp json_safe(value), do: value

  defp checkpoint_id(digest, created_at) do
    suffix = String.slice(digest, 0, 16)
    time = String.replace(created_at, ~r/[^0-9A-Za-z]/, "")
    "operator-task-batch-checkpoint:#{suffix}:#{time}"
  end

  defp plan_id(checkpoint, next_action) do
    digest =
      checkpoint
      |> Map.get("checkpoint_digest", "")
      |> String.slice(0, 16)

    "operator-task-batch-resume:#{next_action}:#{digest}"
  end

  defp task_count(%{"task_count" => count}) when is_integer(count), do: count
  defp task_count(%{"tasks" => tasks}) when is_list(tasks), do: length(tasks)
  defp task_count(_batch), do: 0

  defp case_index(%{"case_index" => case_index}) when is_list(case_index), do: case_index

  defp case_index(%{"tasks" => tasks}) when is_list(tasks) do
    Enum.map(tasks, fn
      %{"case_id" => case_id, "task_ir" => task_ir} when is_map(task_ir) ->
        %{
          "case_id" => case_id,
          "task_id" => Map.get(task_ir, "task_id"),
          "task_digest" => get_in(task_ir, ["integrity", "task_digest"])
        }

      entry when is_map(entry) ->
        Map.take(entry, ["case_id", "task_id", "task_digest"])
    end)
  end

  defp case_index(_batch), do: []

  defp checkpoint_summary(nil), do: nil

  defp checkpoint_summary(%{"run_phase" => "execute"} = result) do
    Map.take(result, [
      "run_id",
      "run_phase",
      "batch_digest",
      "started_at",
      "finished_at",
      "task_count",
      "verified_count",
      "executed_count",
      "ok_count",
      "error_count",
      "failed_case_ids",
      "readiness_counts"
    ])
    |> Map.put("blocked_readiness_case_ids", blocked_readiness_case_ids(result))
  end

  defp checkpoint_summary(result) when is_map(result) do
    Map.take(result, [
      "run_id",
      "run_phase",
      "batch_digest",
      "started_at",
      "finished_at",
      "task_count",
      "verified_count",
      "executed_count",
      "ok_count",
      "error_count",
      "failed_case_ids",
      "readiness_counts"
    ])
  end

  defp blocked_readiness_case_ids(%{"results" => results}) when is_list(results) do
    results
    |> Enum.filter(&(get_in(&1, ["execution_readiness", "status"]) == "blocked"))
    |> Enum.map(&Map.get(&1, "case_id"))
    |> normalize_string_list()
  end

  defp blocked_readiness_case_ids(_result), do: []

  defp resume_policy(_preparation, %{
         "error_count" => 0,
         "readiness_counts" => %{"blocked" => blocked_count}
       })
       when is_integer(blocked_count) and blocked_count > 0,
       do: %{"status" => "blocked", "next_action" => "resolve_blocked_cases"}

  defp resume_policy(_preparation, %{"error_count" => 0}),
    do: %{"status" => "complete", "next_action" => "archive"}

  defp resume_policy(_preparation, %{"error_count" => count})
       when is_integer(count) and count > 0,
       do: %{"status" => "partial", "next_action" => "retry_failed_cases"}

  defp resume_policy(%{"error_count" => 0}, _execution),
    do: %{"status" => "prepared", "next_action" => "execute"}

  defp resume_policy(%{"error_count" => count}, _execution) when is_integer(count) and count > 0,
    do: %{"status" => "blocked", "next_action" => "fix_invalid_cases"}

  defp resume_policy(_preparation, _execution),
    do: %{"status" => "draft", "next_action" => "prepare"}

  defp checkpoint_digest(manifest) do
    manifest
    |> Map.delete("checkpoint_digest")
    |> digest_term()
  end

  defp verify_checkpoint_contract(%{
         "operator_task_batch_checkpoint_contract" => "kyuubiki.operator_task_batch_checkpoint/v1"
       }),
       do: :ok

  defp verify_checkpoint_contract(%{"operator_task_batch_checkpoint_contract" => contract}),
    do: {:error, {:invalid_operator_task_batch_checkpoint_contract, contract}}

  defp verify_checkpoint_contract(_checkpoint),
    do: {:error, :missing_operator_task_batch_checkpoint_contract}

  defp verify_field(checkpoint, field, expected) do
    case Map.fetch(checkpoint, field) do
      {:ok, ^expected} ->
        :ok

      {:ok, actual} ->
        {:error, {:operator_task_batch_checkpoint_mismatch, field, actual, expected}}

      :error ->
        {:error, {:missing_operator_task_batch_checkpoint_field, field}}
    end
  end

  defp target_case_ids(batch, _checkpoint, "execute"), do: all_case_ids(batch)
  defp target_case_ids(_batch, _checkpoint, "archive"), do: []

  defp target_case_ids(_batch, checkpoint, "retry_failed_cases") do
    checkpoint
    |> get_in(["execution", "failed_case_ids"])
    |> normalize_string_list()
  end

  defp target_case_ids(_batch, checkpoint, "resolve_blocked_cases") do
    checkpoint
    |> get_in(["execution", "blocked_readiness_case_ids"])
    |> normalize_string_list()
  end

  defp target_case_ids(_batch, _checkpoint, _next_action), do: []

  defp blocked_case_ids(batch, _checkpoint, "fix_invalid_cases"), do: all_case_ids(batch)

  defp blocked_case_ids(_batch, checkpoint, "resolve_blocked_cases") do
    checkpoint
    |> get_in(["execution", "blocked_readiness_case_ids"])
    |> normalize_string_list()
  end

  defp blocked_case_ids(_batch, _checkpoint, _next_action), do: []

  defp all_case_ids(batch) do
    batch
    |> case_index()
    |> Enum.map(&Map.get(&1, "case_id"))
    |> normalize_string_list()
  end

  defp normalize_string_list(values) when is_list(values) do
    values
    |> Enum.filter(&(is_binary(&1) and &1 != ""))
    |> Enum.uniq()
  end

  defp normalize_string_list(_values), do: []
end
