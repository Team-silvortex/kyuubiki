defmodule KyuubikiSdk.OperatorTasks do
  @moduledoc "Helpers for Operator TaskIR response envelopes."

  @receipt_keys ["failure_receipt", "operator_task_failure_receipt"]
  @receipt_schemas [
    "kyuubiki.headless-operator-task-failure/v1",
    "kyuubiki.agent-operator-task-failure/v1",
    "kyuubiki.control-plane-operator-task-failure/v1"
  ]

  def failure_receipts(payload) do
    payload
    |> collect_receipts([])
    |> Enum.reverse()
    |> Enum.uniq_by(&receipt_key/1)
  end

  def failure_actions(payload) do
    receipt_actions =
      payload
      |> failure_receipts()
      |> Enum.map(&get_in(&1, ["recovery", "required_action"]))

    (receipt_actions ++ recovery_action_values(payload))
    |> Enum.filter(&is_binary/1)
    |> Enum.reject(&(&1 == ""))
    |> Enum.uniq()
  end

  def recovery_summary(payload) do
    receipts = failure_receipts(payload)

    %{
      "next_action" => first_string_value(payload, "next_action"),
      "target_case_ids" => first_string_list_value(payload, "target_case_ids"),
      "blocked_case_ids" => first_string_list_value(payload, "blocked_case_ids"),
      "recovery_actions" => failure_actions(payload),
      "failure_receipt_count" => length(receipts),
      "failure_receipts" => receipts
    }
  end

  defp collect_receipts(%{} = payload, receipts) do
    receipts =
      Enum.reduce(@receipt_keys, receipts, fn key, acc ->
        case Map.get(payload, key) do
          %{} = receipt -> maybe_add_receipt(receipt, acc)
          _ -> acc
        end
      end)

    receipts = maybe_add_receipt(payload, receipts)

    payload
    |> Map.values()
    |> Enum.reduce(receipts, &collect_receipts/2)
  end

  defp collect_receipts(values, receipts) when is_list(values) do
    Enum.reduce(values, receipts, &collect_receipts/2)
  end

  defp collect_receipts(_value, receipts), do: receipts

  defp recovery_action_values(payload) do
    payload
    |> collect_recovery_actions([])
    |> Enum.reverse()
  end

  defp collect_recovery_actions(%{} = payload, actions) do
    actions =
      case Map.get(payload, "recovery_actions") do
        values when is_list(values) ->
          Enum.reduce(values, actions, fn action, acc -> [action | acc] end)

        _ ->
          actions
      end

    payload
    |> Map.values()
    |> Enum.reduce(actions, &collect_recovery_actions/2)
  end

  defp collect_recovery_actions(values, actions) when is_list(values) do
    Enum.reduce(values, actions, &collect_recovery_actions/2)
  end

  defp collect_recovery_actions(_value, actions), do: actions

  defp first_string_value(%{} = payload, key) do
    case Map.get(payload, key) do
      value when is_binary(value) and value != "" ->
        value

      _ ->
        payload
        |> Map.values()
        |> Enum.find_value(&first_string_value(&1, key))
    end
  end

  defp first_string_value(values, key) when is_list(values),
    do: Enum.find_value(values, &first_string_value(&1, key))

  defp first_string_value(_value, _key), do: nil

  defp first_string_list_value(%{} = payload, key) do
    case Map.get(payload, key) do
      values when is_list(values) ->
        Enum.filter(values, &is_binary/1)

      _ ->
        payload
        |> Map.values()
        |> Enum.find_value([], fn child ->
          case first_string_list_value(child, key) do
            [] -> nil
            values -> values
          end
        end)
    end
  end

  defp first_string_list_value(values, key) when is_list(values) do
    Enum.find_value(values, [], fn child ->
      case first_string_list_value(child, key) do
        [] -> nil
        values -> values
      end
    end)
  end

  defp first_string_list_value(_value, _key), do: []

  defp maybe_add_receipt(%{"schema_version" => schema} = receipt, receipts)
       when schema in @receipt_schemas do
    [receipt | receipts]
  end

  defp maybe_add_receipt(_value, receipts), do: receipts

  defp receipt_key(receipt) do
    {
      Map.get(receipt, "schema_version"),
      Map.get(receipt, "failure_stage"),
      Map.get(receipt, "reason_code"),
      Map.get(receipt, "task_id"),
      Map.get(receipt, "operator_id"),
      Map.get(receipt, "task_digest")
    }
  end
end
