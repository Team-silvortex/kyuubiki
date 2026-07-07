defmodule KyuubikiWeb.Orchestra.OperatorTaskExecutor do
  @moduledoc """
  Local executor for the operator task IR contract.

  Rust agents can implement the same dispatch contract behind
  `run_operator_task_ir`; this module keeps Elixir-side tests executable.
  """

  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRun
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutionSummary
  alias KyuubikiWeb.WorkflowOperatorRuntime

  @schema_version "kyuubiki.operator-task-ir/v1"
  @batch_contract "kyuubiki.quality_execution_batch/v1"
  @batch_execution_contract "kyuubiki.operator_task_batch_execution/v1"
  @agent_rpc_method "run_operator_task_ir"

  @spec execute(map()) :: {:ok, map()} | {:error, term()}
  def execute(%{"schema_version" => @schema_version} = task_ir) do
    with :ok <- OperatorTaskExecutionSummary.validate_digest(task_ir),
         {:ok, summary} <- OperatorTaskExecutionSummary.build(task_ir),
         {:ok, input} <- input_artifact(task_ir),
         config <- config(task_ir),
         node <- execution_node(task_ir, summary["operator_id"]) do
      dispatch(summary["operator_kind"], summary["operator_id"], input, config, node)
    end
  end

  def execute(_task_ir), do: {:error, :invalid_operator_task_ir}

  @spec execute_batch(map(), keyword()) :: {:ok, map()} | {:error, term()}
  def execute_batch(batch, opts \\ [])

  def execute_batch(
        %{"quality_execution_batch_contract" => _contract, "tasks" => tasks} = batch,
        opts
      )
      when is_list(tasks) and is_list(opts) do
    with :ok <- validate_batch_contract(batch, tasks) do
      strict = Keyword.get(opts, :strict, false)
      run = OperatorTaskBatchRun.start(batch, "execute", opts)

      tasks
      |> Enum.with_index()
      |> Enum.reduce_while({[], 0}, fn {entry, index}, {results, ok_count} ->
        result = execute_batch_entry(entry, index)
        next_ok_count = if result["status"] == "ok", do: ok_count + 1, else: ok_count
        next_results = [result | results]

        if strict and result["status"] != "ok" do
          {:halt, {next_results, next_ok_count}}
        else
          {:cont, {next_results, next_ok_count}}
        end
      end)
      |> batch_result(tasks, run, opts)
    end
  end

  def execute_batch(_batch, _opts), do: {:error, :invalid_operator_task_batch}

  @spec prepare_batch(map()) :: {:ok, map()} | {:error, term()}
  def prepare_batch(%{"quality_execution_batch_contract" => _contract, "tasks" => tasks} = batch)
      when is_list(tasks) do
    with :ok <- validate_batch_contract(batch, tasks) do
      run = OperatorTaskBatchRun.start(batch, "prepare")

      summaries =
        tasks
        |> Enum.with_index()
        |> Enum.map(fn {entry, index} -> prepare_batch_entry(entry, index) end)

      verified_count = Enum.count(summaries, &(&1["status"] == "verified"))
      error_code_counts = error_code_counts(summaries)

      result = %{
        "operator_task_batch_preparation_contract" =>
          "kyuubiki.operator_task_batch_preparation/v1",
        "quality_execution_batch_contract" => @batch_contract,
        "task_count" => length(tasks),
        "verified_count" => verified_count,
        "error_count" => length(summaries) - verified_count,
        "error_codes" => Map.keys(error_code_counts),
        "error_code_counts" => error_code_counts,
        "summaries" => summaries
      }

      {:ok, OperatorTaskBatchRun.finish(result, run)}
    end
  end

  def prepare_batch(_batch), do: {:error, :invalid_operator_task_batch}

  defp dispatch("solver", operator_id, input, _config, node),
    do: WorkflowOperatorRuntime.run_solve_operator(operator_id, input, node)

  defp dispatch("transform", operator_id, input, config, _node),
    do: WorkflowOperatorRuntime.run_transform_operator(operator_id, input, config)

  defp dispatch("workflow_bridge", operator_id, input, config, _node),
    do: WorkflowOperatorRuntime.run_transform_operator(operator_id, input, config)

  defp dispatch("extract", operator_id, input, config, _node),
    do: WorkflowOperatorRuntime.run_extract_operator(operator_id, input, config)

  defp dispatch("export", operator_id, input, config, _node),
    do: WorkflowOperatorRuntime.run_export_operator(operator_id, input, config)

  defp dispatch(kind, _operator_id, _input, _config, _node),
    do: {:error, {:unsupported_operator_task_kind, kind}}

  defp execute_batch_entry(
         %{"task_ir" => %{"schema_version" => @schema_version} = task_ir} = entry,
         _index
       ) do
    with :ok <- validate_batch_entry_contract(entry, task_ir),
         {:ok, result} <- execute(task_ir) do
      %{
        "case_id" => Map.get(entry, "case_id"),
        "task_id" => Map.get(task_ir, "task_id"),
        "task_digest" => get_in(task_ir, ["integrity", "task_digest"]),
        "operator_id" => get_in(task_ir, ["operator", "id"]),
        "status" => "ok",
        "result" => result
      }
    else
      {:error, reason} -> failed_batch_entry(entry, task_ir, reason)
    end
  end

  defp execute_batch_entry(entry, index) when is_map(entry),
    do: failed_batch_entry(entry, %{"task_id" => "batch-entry-#{index}"}, :missing_task_ir)

  defp execute_batch_entry(_entry, index),
    do: failed_batch_entry(%{}, %{"task_id" => "batch-entry-#{index}"}, :invalid_task_entry)

  defp prepare_batch_entry(
         %{"task_ir" => %{"schema_version" => @schema_version} = task_ir} = entry,
         _index
       ) do
    with :ok <- validate_batch_entry_contract(entry, task_ir),
         :ok <- OperatorTaskExecutionSummary.validate_digest(task_ir),
         {:ok, summary} <- OperatorTaskExecutionSummary.build(task_ir) do
      summary
      |> Map.put("case_id", Map.get(entry, "case_id"))
      |> Map.put("status", "verified")
    else
      {:error, reason} -> failed_batch_entry(entry, task_ir, reason)
    end
  end

  defp prepare_batch_entry(entry, index) when is_map(entry),
    do: failed_batch_entry(entry, %{"task_id" => "batch-entry-#{index}"}, :missing_task_ir)

  defp prepare_batch_entry(_entry, index),
    do: failed_batch_entry(%{}, %{"task_id" => "batch-entry-#{index}"}, :invalid_task_entry)

  defp failed_batch_entry(entry, task_ir, reason) do
    %{
      "case_id" => Map.get(entry, "case_id"),
      "task_id" => Map.get(task_ir, "task_id"),
      "task_digest" => get_in(task_ir, ["integrity", "task_digest"]),
      "operator_id" => get_in(task_ir, ["operator", "id"]),
      "status" => "error",
      "error" => inspect(reason),
      "error_code" => error_code(reason)
    }
  end

  defp error_code({:operator_task_digest_mismatch, _mismatch}),
    do: "operator_task_digest_mismatch"

  defp error_code({:operator_task_mirror_mismatch, _mismatch}),
    do: "operator_task_mirror_mismatch"

  defp error_code(:missing_operator_task_digest), do: "operator_task_digest_missing"

  defp error_code(:operator_task_execution_abi_mismatch),
    do: "operator_task_execution_abi_mismatch"

  defp error_code(:operator_task_program_mismatch), do: "operator_task_program_mismatch"
  defp error_code(:operator_task_entrypoint_mismatch), do: "operator_task_entrypoint_mismatch"

  defp error_code({:operator_task_batch_entry_mismatch, _field, _actual, _expected}),
    do: "operator_task_batch_entry_mismatch"

  defp error_code(:operator_task_batch_entry_rpc_mirror_mismatch),
    do: "operator_task_batch_entry_rpc_mirror_mismatch"

  defp error_code(:operator_task_batch_entry_rpc_missing_task_ir),
    do: "operator_task_batch_entry_rpc_missing_task_ir"

  defp error_code({:operator_task_batch_entry_rpc_method_mismatch, _method}),
    do: "operator_task_batch_entry_rpc_method_mismatch"

  defp error_code(:missing_task_ir), do: "operator_task_batch_entry_missing_task_ir"
  defp error_code(:invalid_task_entry), do: "operator_task_batch_entry_invalid"
  defp error_code(reason) when is_atom(reason), do: Atom.to_string(reason)
  defp error_code(_reason), do: "operator_task_batch_entry_error"

  defp batch_result({results, ok_count}, tasks, run, opts) do
    results = Enum.reverse(results)
    error_code_counts = error_code_counts(results)

    result = %{
      "operator_task_batch_execution_contract" => @batch_execution_contract,
      "task_count" => length(tasks),
      "executed_count" => length(results),
      "ok_count" => ok_count,
      "error_count" => length(results) - ok_count,
      "error_codes" => Map.keys(error_code_counts),
      "error_code_counts" => error_code_counts,
      "failed_case_ids" => failed_case_ids(results),
      "results" => results
    }

    {:ok, OperatorTaskBatchRun.finish(result, run, opts)}
  end

  defp failed_case_ids(results) do
    results
    |> Enum.filter(&(&1["status"] == "error"))
    |> Enum.map(&Map.get(&1, "case_id"))
    |> Enum.filter(&(is_binary(&1) and &1 != ""))
  end

  defp error_code_counts(entries) do
    entries
    |> Enum.reduce(%{}, fn entry, counts ->
      case Map.get(entry, "error_code") do
        code when is_binary(code) and code != "" -> Map.update(counts, code, 1, &(&1 + 1))
        _code -> counts
      end
    end)
    |> Enum.sort()
    |> Map.new()
  end

  defp input_artifact(%{"input_artifact" => input}) when is_map(input), do: {:ok, input}
  defp input_artifact(_task_ir), do: {:error, :missing_operator_task_input}

  defp config(%{"config" => config}) when is_map(config), do: config
  defp config(_task_ir), do: %{}

  defp execution_node(%{"node" => node, "orchestration_context" => context}, operator_id)
       when is_map(node) and is_map(context) do
    node
    |> Map.put_new("operator_id", operator_id)
    |> Map.put("orchestration_context", context)
  end

  defp execution_node(%{"node" => node}, operator_id) when is_map(node),
    do: Map.put_new(node, "operator_id", operator_id)

  defp execution_node(_task_ir, operator_id), do: %{"operator_id" => operator_id}

  defp validate_batch_contract(
         %{"quality_execution_batch_contract" => @batch_contract} = batch,
         tasks
       ) do
    with :ok <- validate_declared_task_count(batch, tasks),
         :ok <- validate_declared_agent_rpc_method(batch),
         :ok <- validate_declared_operator_id(batch, tasks),
         :ok <- validate_declared_case_index(batch, tasks) do
      :ok
    end
  end

  defp validate_batch_contract(%{"quality_execution_batch_contract" => contract}, _tasks),
    do: {:error, {:invalid_operator_task_batch_contract, contract}}

  defp validate_declared_task_count(%{"task_count" => count}, tasks) when count != length(tasks),
    do: {:error, {:operator_task_batch_count_mismatch, %{declared: count, actual: length(tasks)}}}

  defp validate_declared_task_count(_batch, _tasks), do: :ok

  defp validate_declared_agent_rpc_method(%{"agent_rpc_method" => method})
       when method != @agent_rpc_method,
       do: {:error, {:operator_task_batch_rpc_method_mismatch, method}}

  defp validate_declared_agent_rpc_method(_batch), do: :ok

  defp validate_declared_operator_id(%{"operator_id" => operator_id}, tasks) do
    tasks
    |> Enum.find(fn
      %{"task_ir" => task_ir} -> get_in(task_ir, ["operator", "id"]) != operator_id
      _entry -> false
    end)
    |> case do
      nil -> :ok
      entry -> {:error, {:operator_task_batch_operator_mismatch, Map.get(entry, "case_id")}}
    end
  end

  defp validate_declared_operator_id(_batch, _tasks), do: :ok

  defp validate_declared_case_index(%{"case_index" => case_index}, tasks)
       when is_list(case_index) do
    expected = Enum.map(tasks, &batch_index_entry/1)

    if case_index == expected do
      :ok
    else
      {:error, :operator_task_batch_case_index_mismatch}
    end
  end

  defp validate_declared_case_index(%{"case_index" => _case_index}, _tasks),
    do: {:error, :invalid_operator_task_batch_case_index}

  defp validate_declared_case_index(_batch, _tasks), do: :ok

  defp batch_index_entry(%{"case_id" => case_id, "task_ir" => task_ir}) when is_map(task_ir) do
    %{
      "case_id" => case_id,
      "task_id" => Map.get(task_ir, "task_id"),
      "task_digest" => get_in(task_ir, ["integrity", "task_digest"])
    }
  end

  defp batch_index_entry(entry) when is_map(entry),
    do: Map.take(entry, ["case_id", "task_id", "task_digest"])

  defp validate_batch_entry_contract(entry, task_ir) do
    with :ok <- validate_entry_field(entry, "task_id", Map.get(task_ir, "task_id")),
         :ok <-
           validate_entry_field(
             entry,
             "task_digest",
             get_in(task_ir, ["integrity", "task_digest"])
           ),
         :ok <- validate_entry_field(entry, "operator_id", get_in(task_ir, ["operator", "id"])),
         :ok <- validate_entry_rpc(entry, task_ir) do
      :ok
    end
  end

  defp validate_entry_field(entry, field, expected) do
    case Map.fetch(entry, field) do
      {:ok, ^expected} -> :ok
      {:ok, actual} -> {:error, {:operator_task_batch_entry_mismatch, field, actual, expected}}
      :error -> :ok
    end
  end

  defp validate_entry_rpc(%{"agent_rpc" => %{"method" => method}}, _task_ir)
       when method != @agent_rpc_method,
       do: {:error, {:operator_task_batch_entry_rpc_method_mismatch, method}}

  defp validate_entry_rpc(%{"agent_rpc" => %{"params" => %{"task_ir" => task_ir}}}, task_ir),
    do: :ok

  defp validate_entry_rpc(%{"agent_rpc" => %{"params" => %{"task_ir" => mirrored_task}}}, task_ir)
       when mirrored_task != task_ir,
       do: {:error, :operator_task_batch_entry_rpc_mirror_mismatch}

  defp validate_entry_rpc(%{"agent_rpc" => %{"params" => params}}, _task_ir) when is_map(params),
    do: {:error, :operator_task_batch_entry_rpc_missing_task_ir}

  defp validate_entry_rpc(%{"agent_rpc" => agent_rpc}, _task_ir) when is_map(agent_rpc), do: :ok

  defp validate_entry_rpc(%{"agent_rpc" => _agent_rpc}, _task_ir),
    do: {:error, :invalid_operator_task_batch_entry_rpc}

  defp validate_entry_rpc(_entry, _task_ir), do: :ok
end
