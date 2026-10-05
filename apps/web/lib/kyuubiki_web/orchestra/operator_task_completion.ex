defmodule KyuubikiWeb.Orchestra.OperatorTaskCompletion do
  @moduledoc "Task-bound completion gate for Agent execution receipts, not RPC delivery acknowledgements."

  @identity_fields ~w(task_id task_digest operator_id program_id)
  @pending_states ~w(verified_pending_execution verified_pending_engine_execution ready_for_package_resolution blocked)
  @failure_states ~w(failed error cancelled)
  @blocking_fields ~w(blocking_stage blocking_reason required_action)
  alias KyuubikiWeb.Orchestra.OperatorTaskFailure

  @spec agent_receipt(map(), term()) :: {:ok, map()} | {:error, term()}
  def agent_receipt(summary, result) when is_map(result) do
    with :ok <- validate_identity(summary, result, true),
         :ok <- validate_mirrors(summary, result),
         {:ok, status} <- execution_status(result),
         {:ok, result} <- validate_failure(summary, result, status) do
      {:ok,
       summary
       |> Map.put("status", status)
       |> Map.put("execution_readiness", result["execution_readiness"])
       |> Map.put("result", result)
       |> Map.merge(Map.take(result, ~w(error_code failure_receipt)))}
    end
  end

  def agent_receipt(_summary, _result), do: invalid("receipt")

  defp validate_failure(summary, %{"failure_receipt" => failure} = result, "failed") do
    code = Map.get(result, "error_code", if(is_map(failure), do: failure["reason_code"]))
    message = if is_map(failure), do: failure["message"]

    with {:ok, failure} <- OperatorTaskFailure.validate(summary, failure, code, message),
         :ok <- OperatorTaskFailure.validate_readiness(result["execution_readiness"], failure) do
      {:ok, result |> Map.put("failure_receipt", failure) |> Map.put("error_code", code)}
    end
  end

  defp validate_failure(_summary, %{"failure_receipt" => failure}, _status)
       when not is_nil(failure),
       do: invalid("failure_receipt")

  defp validate_failure(_summary, result, _status), do: {:ok, result}

  defp validate_identity(summary, receipt, required?) do
    Enum.reduce_while(@identity_fields, :ok, fn field, :ok ->
      case Map.fetch(receipt, field) do
        {:ok, value} when is_binary(value) and value != "" ->
          if value == summary[field], do: {:cont, :ok}, else: {:halt, invalid(field)}

        :error when not required? ->
          {:cont, :ok}

        _ ->
          {:halt, invalid(field)}
      end
    end)
  end

  defp validate_mirrors(summary, result) do
    Enum.reduce_while(~w(validation_receipt provenance_receipt), :ok, fn field, :ok ->
      case Map.fetch(result, field) do
        :error ->
          {:cont, :ok}

        {:ok, receipt} when is_map(receipt) ->
          with :ok <- validate_identity(summary, receipt, false),
               true <- Map.get(receipt, "digest_verified", true) == true do
            {:cont, :ok}
          else
            _ -> {:halt, invalid(field)}
          end

        _ ->
          {:halt, invalid(field)}
      end
    end)
  end

  defp execution_status(%{"operator_task_ir_status" => "executed"} = result) do
    cond do
      result["ok"] == false or not is_nil(result["error"]) or
        not is_nil(result["failure_receipt"]) or not is_nil(result["blocked_stage"]) ->
        invalid("completion")

      not is_map(result["result"]) ->
        invalid("result")

      not executed_readiness?(result["execution_readiness"]) ->
        invalid("execution_readiness")

      true ->
        {:ok, "executed"}
    end
  end

  defp execution_status(%{"operator_task_ir_status" => status} = result)
       when status in @pending_states do
    case result["execution_readiness"] do
      %{"status" => readiness, "ready_to_dispatch" => false}
      when readiness in ["blocked", "ready_for_package_resolution"] ->
        {:ok, "blocked"}

      _ ->
        invalid("execution_readiness")
    end
  end

  defp execution_status(%{"operator_task_ir_status" => status} = result)
       when status in @failure_states do
    case result["execution_readiness"] do
      %{"status" => "blocked", "ready_to_dispatch" => false} -> {:ok, "failed"}
      _ -> invalid("execution_readiness")
    end
  end

  defp execution_status(_result), do: invalid("operator_task_ir_status")

  def executed_readiness?(%{"status" => "executed", "ready_to_dispatch" => true} = readiness),
    do: Enum.all?(@blocking_fields, &is_nil(readiness[&1]))

  def executed_readiness?(_readiness), do: false

  # Rejected receipt values may contain secrets; errors expose only the contract field.
  defp invalid(field), do: {:error, {:operator_task_execution_receipt_invalid, field}}
end
