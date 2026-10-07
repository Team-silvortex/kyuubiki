defmodule KyuubikiWeb.Orchestra.OperatorTaskRequestBudget do
  @moduledoc """
  Bounded waiting policy outside the signed TaskIR. These are Agent queue/RPC
  wait limits, not CPU termination guarantees or authorization to replay work.
  """
  @schema "kyuubiki.operator-task-request-budget/v1"
  @keys ~w(schema_version queue_timeout_ms request_timeout_ms)
  @max_phase_ms 600_000

  @spec options(map()) :: {:ok, keyword()} | {:error, atom()}
  def options(payload) when is_map(payload) do
    case Map.fetch(payload, "execution_budget") do
      :error -> {:ok, []}
      {:ok, budget} -> validate(budget)
    end
  end

  defp validate(
         %{
           "schema_version" => @schema,
           "queue_timeout_ms" => queue,
           "request_timeout_ms" => execution
         } = budget
       )
       when is_integer(queue) and queue in 1..@max_phase_ms and
              is_integer(execution) and execution in 1..@max_phase_ms do
    if Enum.sort(Map.keys(budget)) == Enum.sort(@keys) do
      {:ok,
       [
         queue_timeout_ms: queue,
         request_timeout_ms: execution,
         retry_safety: :checkpoint_required
       ]}
    else
      {:error, :operator_task_execution_budget_invalid}
    end
  end

  defp validate(_budget), do: {:error, :operator_task_execution_budget_invalid}
end
