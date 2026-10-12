defmodule KyuubikiWeb.Orchestra.OperatorTaskBatchContract do
  @moduledoc false

  @contract "kyuubiki.quality_execution_batch/v1"
  @rpc_method "run_operator_task_ir"
  @objects [
    ~w(operator),
    ~w(operator execution),
    ~w(integrity),
    ~w(execution_program),
    ~w(execution_program abi),
    ~w(execution_program entrypoint),
    ~w(runtime_hints),
    ~w(node),
    ~w(orchestration_context),
    ~w(input_artifact),
    ~w(config),
    ~w(dataset_contract)
  ]
  @texts [
    ~w(schema_version),
    ~w(operator id),
    ~w(operator kind),
    ~w(execution_program schema_version),
    ~w(execution_program program_id),
    ~w(execution_program program_kind),
    ~w(execution_program runtime_protocol),
    ~w(execution_program package_ref),
    ~w(execution_program package_version),
    ~w(execution_program abi kind),
    ~w(execution_program entrypoint kind),
    ~w(execution_program entrypoint name),
    ~w(execution_program entrypoint operator_kind),
    ~w(runtime_hints authority_mode),
    ~w(runtime_hints execution_mode),
    ~w(runtime_hints cache_scope),
    ~w(runtime_hints operator_kind),
    ~w(runtime_hints package_ref),
    ~w(runtime_hints package_version),
    ~w(operator execution package_ref)
  ]

  def validate(%{"quality_execution_batch_contract" => @contract, "tasks" => tasks} = batch)
      when is_list(tasks) do
    object(batch, "batch")
    validate_count(batch, tasks)

    Enum.reduce(Enum.with_index(tasks), {MapSet.new(), MapSet.new()}, fn {entry, index},
                                                                         {cases, ids} ->
      path = "batch.tasks[#{index}]"
      object(entry, path)
      case_id = identity(entry["case_id"], "#{path}.case_id")
      task = object(entry["task_ir"], "#{path}.task_ir")
      task_id = identity(task["task_id"], "#{path}.task_ir.task_id")

      for {kind, value, seen} <- [{"case_id", case_id, cases}, {"task_id", task_id, ids}] do
        if MapSet.member?(seen, value),
          do:
            fail(
              {:operator_task_batch_identity_conflict, %{field: kind, value: value, index: index}}
            )
      end

      for keys <- @objects do
        optional(task, keys, fn value ->
          object(value, "#{path}.task_ir.#{Enum.join(keys, ".")}")
        end)
      end

      for keys <- @texts do
        optional(task, keys, fn value ->
          if value != nil && !is_binary(value),
            do: invalid("#{path}.task_ir.#{Enum.join(keys, ".")}", "must be text or null")
        end)
      end

      {MapSet.put(cases, case_id), MapSet.put(ids, task_id)}
    end)

    validate_headers(batch, tasks)
    :ok
  catch
    {:batch_contract_invalid, reason} -> {:error, reason}
  end

  def validate(%{"quality_execution_batch_contract" => contract}) when contract != @contract,
    do: {:error, {:invalid_operator_task_batch_contract, contract}}

  def validate(_batch), do: {:error, :invalid_operator_task_batch}

  # Call only after validation; every recovery identity is then present and unique.
  def case_index(%{"tasks" => tasks}) do
    Enum.map(tasks, fn entry ->
      task = entry["task_ir"]

      %{
        "case_id" => entry["case_id"],
        "task_id" => task["task_id"],
        "task_digest" => get_in(task, ["integrity", "task_digest"])
      }
    end)
  end

  defp validate_count(%{"task_count" => count}, tasks) when count != length(tasks),
    do: fail({:operator_task_batch_count_mismatch, %{declared: count, actual: length(tasks)}})

  defp validate_count(_batch, _tasks), do: :ok

  defp validate_headers(batch, tasks) do
    if Map.has_key?(batch, "agent_rpc_method") && batch["agent_rpc_method"] != @rpc_method,
      do: fail({:operator_task_batch_rpc_method_mismatch, batch["agent_rpc_method"]})

    if Map.has_key?(batch, "operator_id") do
      identity(batch["operator_id"], "batch.operator_id")

      for entry <- tasks,
          get_in(entry, ["task_ir", "operator", "id"]) != batch["operator_id"],
          do: fail({:operator_task_batch_operator_mismatch, entry["case_id"]})
    end

    case Map.fetch(batch, "case_index") do
      :error ->
        :ok

      {:ok, index} when is_list(index) ->
        if index !== case_index(batch), do: fail(:operator_task_batch_case_index_mismatch)

      _ ->
        fail(:invalid_operator_task_batch_case_index)
    end
  end

  defp optional(object, [key], check) do
    case Map.fetch(object, key) do
      {:ok, value} -> check.(value)
      :error -> :ok
    end
  end

  defp optional(object, [key | rest], check) do
    case Map.fetch(object, key) do
      {:ok, value} -> optional(value, rest, check)
      :error -> :ok
    end
  end

  defp object(value, path) when is_map(value) and not is_struct(value) do
    if Enum.all?(Map.keys(value), &is_binary/1),
      do: value,
      else: invalid(path, "must have string keys")
  end

  defp object(_value, path), do: invalid(path, "must be an object")

  defp identity(value, path) when is_binary(value) do
    if String.trim(value) != "", do: value, else: invalid(path, "must be nonblank text")
  end

  defp identity(_value, path), do: invalid(path, "must be nonblank text")

  defp invalid(path, reason), do: fail({:invalid_operator_task_batch_structure, path, reason})
  defp fail(reason), do: throw({:batch_contract_invalid, reason})
end
