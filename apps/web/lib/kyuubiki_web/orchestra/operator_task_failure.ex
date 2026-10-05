defmodule KyuubikiWeb.Orchestra.OperatorTaskFailure do
  @moduledoc "Task-bound Agent failure contract; recovery advice never authorizes automatic replay."

  @schema "kyuubiki.agent-operator-task-failure/v1"
  @identities ~w(task_id task_digest operator_id)
  @fields ~w(schema_version failure_owner failure_stage reason_code message task_id task_digest operator_id program_id recovery)

  @spec agent_rpc_result(map(), term(), term(), term()) :: {:ok, map()} | {:error, term()}
  def agent_rpc_result(summary, code, message, failure) do
    with {:ok, failure} <- validate(summary, failure, code, message) do
      {:ok,
       summary
       |> Map.take(@identities ++ ["program_id"])
       |> Map.put("operator_task_ir_status", "failed")
       |> Map.put("error_code", failure["reason_code"])
       |> Map.put("failure_receipt", failure)
       |> Map.put("execution_readiness", readiness(failure))}
    end
  end

  @spec validate(map(), term(), term(), term()) :: {:ok, map()} | {:error, term()}
  def validate(summary, failure, code, message) when is_map(failure) do
    with :ok <- equal(failure["schema_version"], @schema, "schema_version"),
         :ok <- equal(failure["failure_owner"], "agent_runtime", "failure_owner"),
         :ok <- identities(summary, failure),
         :ok <- text(failure["reason_code"], 128, "reason_code"),
         :ok <- equal(failure["reason_code"], code, "reason_code"),
         :ok <- text(failure["failure_stage"], 128, "failure_stage"),
         :ok <- text(failure["message"], 4096, "message"),
         :ok <- equal(failure["message"], message, "message"),
         {:ok, recovery} <- recovery(failure["recovery"]) do
      {:ok, failure |> Map.take(@fields) |> Map.put("recovery", recovery)}
    end
  end

  def validate(_summary, _failure, _code, _message), do: invalid("failure_receipt")

  @spec validate_readiness(term(), map()) :: :ok | {:error, term()}
  def validate_readiness(actual, failure) when is_map(actual) do
    expected = readiness(failure)

    if Map.take(actual, Map.keys(expected)) == expected,
      do: :ok,
      else: invalid("failure_receipt.execution_readiness")
  end

  def validate_readiness(_actual, _failure), do: invalid("failure_receipt.execution_readiness")

  defp identities(summary, failure) do
    Enum.reduce_while(@identities ++ ["program_id"], :ok, fn field, :ok ->
      if field == "program_id" and not Map.has_key?(failure, field) do
        {:cont, :ok}
      else
        with :ok <- text(failure[field], 1024, field),
             :ok <- equal(failure[field], summary[field], field) do
          {:cont, :ok}
        else
          error -> {:halt, error}
        end
      end
    end)
  end

  defp recovery(recovery) when is_map(recovery) do
    with :ok <- boolean(recovery["retryable"], "recovery.retryable"),
         :ok <-
           boolean(
             recovery["safe_to_continue_other_tasks"],
             "recovery.safe_to_continue_other_tasks"
           ),
         :ok <- text(recovery["required_action"], 128, "recovery.required_action") do
      {:ok, Map.take(recovery, ~w(retryable safe_to_continue_other_tasks required_action))}
    end
  end

  defp recovery(_recovery), do: invalid("recovery")
  defp boolean(value, _field) when is_boolean(value), do: :ok
  defp boolean(_value, field), do: invalid(field)

  defp text(value, limit, _field)
       when is_binary(value) and byte_size(value) > 0 and byte_size(value) <= limit,
       do: :ok

  defp text(_value, _limit, field), do: invalid(field)
  defp equal(value, value, _field), do: :ok
  defp equal(_value, _expected, field), do: invalid(field)

  defp readiness(failure) do
    %{
      "status" => "blocked",
      "requested_mode" => "execute",
      "ready_to_dispatch" => false,
      "current_stage" => failure["failure_stage"],
      "blocking_stage" => failure["failure_stage"],
      "blocking_reason" => failure["reason_code"],
      "blocking_owner" => "agent_runtime",
      "required_action" => failure["recovery"]["required_action"]
    }
  end

  # Reject values by field name only; never echo an untrusted receipt into an error.
  defp invalid(field), do: {:error, {:operator_task_execution_receipt_invalid, field}}
end
