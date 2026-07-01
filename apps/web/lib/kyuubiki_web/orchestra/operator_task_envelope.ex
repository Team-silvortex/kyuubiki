defmodule KyuubikiWeb.Orchestra.OperatorTaskEnvelope do
  @moduledoc """
  API-facing helpers for operator TaskIR envelopes.
  """

  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor

  @task_schema "kyuubiki.operator-task-ir/v1"

  @spec prepare(map()) :: {:ok, map()} | {:error, term()}
  def prepare(%{"task" => %{"schema_version" => @task_schema} = task}) do
    with :ok <- validate_task_digest(task) do
      {:ok, prepare_summary(task)}
    end
  end

  def prepare(%{"task" => task}) when is_map(task), do: {:error, :invalid_operator_task_ir}
  def prepare(_payload), do: {:error, :missing_operator_task}

  @spec execute(map()) :: {:ok, map()} | {:error, term()}
  def execute(%{"task" => %{"schema_version" => @task_schema} = task}) do
    with :ok <- validate_task_digest(task),
         {:ok, result} <- OperatorTaskExecutor.execute(task) do
      {:ok, Map.put(prepare_summary(task), "status", "executed") |> Map.put("result", result)}
    end
  end

  def execute(%{"task" => task}) when is_map(task), do: {:error, :invalid_operator_task_ir}
  def execute(_payload), do: {:error, :missing_operator_task}

  defp validate_task_digest(%{"integrity" => %{"task_digest" => task_digest}} = task)
       when is_binary(task_digest) and task_digest != "" do
    actual = OperatorTaskIR.compute_task_digest(task)

    if actual == task_digest do
      :ok
    else
      {:error, {:operator_task_digest_mismatch, %{expected: task_digest, actual: actual}}}
    end
  end

  defp validate_task_digest(_task), do: {:error, :missing_operator_task_digest}

  defp prepare_summary(task) do
    %{
      "status" => "verified",
      "task_digest" => OperatorTaskIR.compute_task_digest(task),
      "task_id" => Map.get(task, "task_id", ""),
      "operator_id" => get_in(task, ["operator", "id"]) || "",
      "operator_kind" => get_in(task, ["operator", "kind"]) || "",
      "program_id" => get_in(task, ["execution_program", "program_id"]) || "",
      "runtime_protocol" => get_in(task, ["execution_program", "runtime_protocol"]) || "",
      "package_ref" => get_in(task, ["execution_program", "package_ref"]) || ""
    }
  end
end
