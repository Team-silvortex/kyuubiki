defmodule KyuubikiWeb.Orchestra.OperatorTaskCheckpointAdmission do
  @moduledoc false
  @case_lists ~w(failed_case_ids skipped_case_ids incomplete_case_ids blocked_readiness_case_ids)
  @policies %{
    "prepare" => "draft",
    "execute" => "prepared",
    "archive" => "complete",
    "retry_failed_cases" => "partial",
    "fix_invalid_cases" => "blocked",
    "resolve_blocked_cases" => "blocked",
    "resolve_incomplete_cases" => ["partial", "blocked"]
  }

  def options(batch, opts) do
    if !Keyword.keyword?(opts), do: invalid("options", "must be keyword options")
    known = case_ids(batch)
    summary(Keyword.get(opts, :preparation), "preparation", known)
    summary(Keyword.get(opts, :execution), "execution", known)

    for key <- [:checkpoint_id, :created_at], value = Keyword.get(opts, key), value != nil do
      if key != :created_at || !is_struct(value, DateTime), do: text(value, Atom.to_string(key))
    end

    :ok
  catch
    {:checkpoint_invalid, reason} -> {:error, reason}
  end

  def manifest(batch, checkpoint) do
    known = case_ids(batch)
    summary(checkpoint["preparation"], "preparation", known)
    summary(checkpoint["execution"], "execution", known)
    policy = object(checkpoint["resume_policy"], "resume_policy")
    allowed = Map.get(@policies, policy["next_action"])

    if allowed == nil || policy["status"] not in List.wrap(allowed),
      do: invalid("resume_policy", "must contain a supported status/action pair")

    for key <- ~w(checkpoint_id created_at), do: text(checkpoint[key], key)

    if checkpoint["digest_algorithm"] != "sha256",
      do: invalid("digest_algorithm", "must be sha256")

    if checkpoint["quality_execution_batch_contract"] != batch["quality_execution_batch_contract"],
      do: invalid("quality_execution_batch_contract", "must match the batch")

    :ok
  catch
    {:checkpoint_invalid, reason} -> {:error, reason}
  end

  defp summary(nil, _path, _known), do: :ok

  defp summary(value, path, known) do
    object(value, path)

    for key <- @case_lists, Map.has_key?(value, key) do
      ids = value[key]
      if !is_list(ids), do: invalid("#{path}.#{key}", "must be an array")

      Enum.reduce(ids, MapSet.new(), fn id, seen ->
        if !is_binary(id) || !MapSet.member?(known, id) || MapSet.member?(seen, id),
          do: invalid("#{path}.#{key}", "must contain unique case IDs from this batch")

        MapSet.put(seen, id)
      end)
    end

    if Map.has_key?(value, "failure_receipts") do
      receipts = value["failure_receipts"]
      if !is_list(receipts), do: invalid("#{path}.failure_receipts", "must be an array")

      for {receipt, index} <- Enum.with_index(receipts) do
        row_path = "#{path}.failure_receipts[#{index}]"
        object(receipt, row_path)

        if Map.has_key?(receipt, "case_id") && !MapSet.member?(known, receipt["case_id"]),
          do: invalid("#{row_path}.case_id", "must belong to this batch")

        if receipt["recovery"] != nil do
          recovery = object(receipt["recovery"], "#{row_path}.recovery")

          if recovery["required_action"] != nil,
            do: text(recovery["required_action"], "#{row_path}.recovery.required_action")
        end
      end
    end

    if Map.has_key?(value, "readiness_recovery_actions") do
      actions = value["readiness_recovery_actions"]
      if !is_list(actions), do: invalid("#{path}.readiness_recovery_actions", "must be an array")
      Enum.each(actions, &text(&1, "#{path}.readiness_recovery_actions entry"))
    end

    :ok
  end

  defp case_ids(batch), do: MapSet.new(batch["tasks"], & &1["case_id"])

  defp object(value, path) when is_map(value) and not is_struct(value) do
    if Enum.all?(Map.keys(value), &is_binary/1),
      do: value,
      else: invalid(path, "must have string keys")
  end

  defp object(_value, path), do: invalid(path, "must be an object or absent")

  defp text(value, path) do
    if !is_binary(value) || String.trim(value) == "", do: invalid(path, "must be nonblank text")
  end

  defp invalid(path, reason),
    do:
      throw(
        {:checkpoint_invalid, {:invalid_operator_task_batch_checkpoint_structure, path, reason}}
      )
end
