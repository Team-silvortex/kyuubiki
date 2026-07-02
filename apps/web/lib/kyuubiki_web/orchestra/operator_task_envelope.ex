defmodule KyuubikiWeb.Orchestra.OperatorTaskEnvelope do
  @moduledoc """
  API-facing helpers for operator TaskIR envelopes.
  """

  alias KyuubikiWeb.Orchestra.OperatorTaskExecutionSummary
  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRun
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor

  @task_schema "kyuubiki.operator-task-ir/v1"

  @spec prepare(map()) :: {:ok, map()} | {:error, term()}
  def prepare(%{"task" => %{"schema_version" => @task_schema} = task}) do
    with :ok <- OperatorTaskExecutionSummary.validate_digest(task),
         {:ok, summary} <- OperatorTaskExecutionSummary.build(task) do
      {:ok, summary}
    end
  end

  def prepare(%{"task" => task}) when is_map(task), do: {:error, :invalid_operator_task_ir}
  def prepare(_payload), do: {:error, :missing_operator_task}

  @spec execute(map()) :: {:ok, map()} | {:error, term()}
  def execute(%{"task" => %{"schema_version" => @task_schema} = task}) do
    with :ok <- OperatorTaskExecutionSummary.validate_digest(task),
         {:ok, summary} <- OperatorTaskExecutionSummary.build(task),
         {:ok, result} <- OperatorTaskExecutor.execute(task) do
      {:ok, summary |> Map.put("status", "executed") |> Map.put("result", result)}
    end
  end

  def execute(%{"task" => task}) when is_map(task), do: {:error, :invalid_operator_task_ir}
  def execute(_payload), do: {:error, :missing_operator_task}

  @spec prepare_batch(map()) :: {:ok, map()} | {:error, term()}
  def prepare_batch(%{"batch" => %{"quality_execution_batch_contract" => _contract} = batch}) do
    with {:ok, result} <- OperatorTaskExecutor.prepare_batch(batch) do
      {:ok, Map.put(result, "status", "verified")}
    end
  end

  def prepare_batch(%{"batch" => batch}) when is_map(batch),
    do: {:error, :invalid_operator_task_batch}

  def prepare_batch(_payload), do: {:error, :missing_operator_task_batch}

  @spec execute_batch(map()) :: {:ok, map()} | {:error, term()}
  def execute_batch(%{"batch" => %{"quality_execution_batch_contract" => _contract} = batch}) do
    with {:ok, result} <- OperatorTaskExecutor.execute_batch(batch) do
      {:ok, Map.put(result, "status", "executed")}
    end
  end

  def execute_batch(%{"batch" => batch}) when is_map(batch),
    do: {:error, :invalid_operator_task_batch}

  def execute_batch(_payload), do: {:error, :missing_operator_task_batch}

  @spec checkpoint_batch(map()) :: {:ok, map()} | {:error, term()}
  def checkpoint_batch(
        %{"batch" => %{"quality_execution_batch_contract" => _contract} = batch} = payload
      ) do
    opts =
      []
      |> maybe_put_checkpoint_opt(:preparation, Map.get(payload, "preparation"))
      |> maybe_put_checkpoint_opt(:execution, Map.get(payload, "execution"))
      |> maybe_put_checkpoint_opt(:checkpoint_id, Map.get(payload, "checkpoint_id"))
      |> maybe_put_checkpoint_opt(:created_at, Map.get(payload, "created_at"))

    {:ok, OperatorTaskBatchRun.checkpoint(batch, opts)}
  end

  def checkpoint_batch(%{"batch" => batch}) when is_map(batch),
    do: {:error, :invalid_operator_task_batch}

  def checkpoint_batch(_payload), do: {:error, :missing_operator_task_batch}

  @spec verify_checkpoint_batch(map()) :: {:ok, map()} | {:error, term()}
  def verify_checkpoint_batch(%{
        "batch" => %{"quality_execution_batch_contract" => _contract} = batch,
        "checkpoint" => checkpoint
      })
      when is_map(checkpoint) do
    OperatorTaskBatchRun.verify_checkpoint(batch, checkpoint)
  end

  def verify_checkpoint_batch(%{"batch" => batch}) when is_map(batch),
    do: {:error, :missing_operator_task_batch_checkpoint}

  def verify_checkpoint_batch(_payload), do: {:error, :missing_operator_task_batch}

  @spec resume_plan_batch(map()) :: {:ok, map()} | {:error, term()}
  def resume_plan_batch(%{
        "batch" => %{"quality_execution_batch_contract" => _contract} = batch,
        "checkpoint" => checkpoint
      })
      when is_map(checkpoint) do
    OperatorTaskBatchRun.resume_plan(batch, checkpoint)
  end

  def resume_plan_batch(%{"batch" => batch}) when is_map(batch),
    do: {:error, :missing_operator_task_batch_checkpoint}

  def resume_plan_batch(_payload), do: {:error, :missing_operator_task_batch}

  defp maybe_put_checkpoint_opt(opts, _key, nil), do: opts
  defp maybe_put_checkpoint_opt(opts, key, value), do: Keyword.put(opts, key, value)
end
