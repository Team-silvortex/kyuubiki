defmodule KyuubikiWeb.Orchestra.OperatorTaskEnvelope do
  @moduledoc """
  API-facing helpers for operator TaskIR envelopes.
  """

  alias KyuubikiWeb.Orchestra.OperatorTaskExecutionSummary
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
end
