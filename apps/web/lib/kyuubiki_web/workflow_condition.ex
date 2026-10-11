defmodule KyuubikiWeb.WorkflowCondition do
  @moduledoc false
  @operators ~w(truthy falsy eq neq gt gte lt lte contains)
  @max_index 18_446_744_073_709_551_615

  def decide(payload, node) do
    with {:ok, predicate_result} <- evaluate(payload, Map.get(node, "config")),
         {:ok, output} <- select_output(node, predicate_result) do
      {:ok, %{"chosen_output" => output["id"], "predicate_result" => predicate_result}}
    end
  end

  def validate_decision(node, %{"chosen_output" => port, "predicate_result" => result} = decision)
      when is_boolean(result) and map_size(decision) == 2 do
    with {:ok, output} <- select_output(node, result),
         true <- output["id"] == port do
      {:ok, output}
    else
      _ -> {:error, :invalid_condition_decision}
    end
  end

  def validate_decision(_node, _decision), do: {:error, :invalid_condition_decision}

  def select_output(node, predicate_result) when is_map(node) and is_boolean(predicate_result) do
    outputs = Map.get(node, "outputs", [])

    if is_list(outputs) and
         Enum.all?(outputs, &(is_map(&1) and is_binary(&1["id"]) and &1["id"] != "")),
       do: choose_output(outputs, predicate_result),
       else: {:error, :invalid_condition_outputs}
  end

  def select_output(_node, _predicate_result), do: {:error, :invalid_condition_outputs}

  defp choose_output(outputs, predicate_result) do
    selected =
      Enum.find(outputs, fn output ->
        output_id = Map.get(output, "id")

        (predicate_result and output_id in ["if_true", "true"]) or
          (!predicate_result and output_id in ["if_false", "false"])
      end) ||
        if predicate_result do
          List.first(outputs)
        else
          Enum.at(outputs, 1) || List.first(outputs)
        end

    if selected, do: {:ok, selected}, else: {:error, :missing_condition_outputs}
  end

  def evaluate(payload, config) do
    with {:ok, predicate} <- predicate(config),
         {:ok, operator} <- operator(predicate),
         {:ok, target} <- target(payload, Map.get(predicate, "path")) do
      compare(operator, target, Map.get(predicate, "value"))
    end
  end

  defp predicate(nil), do: {:ok, %{}}

  defp predicate(config) when is_map(config) do
    case Map.get(config, "predicate", %{}) do
      value when is_map(value) -> {:ok, value}
      _ -> {:error, "condition config.predicate must be an object"}
    end
  end

  defp predicate(_), do: {:error, "condition config must be an object or null"}

  defp operator(predicate) do
    case Map.get(predicate, "operator", "gt") do
      value when value in @operators ->
        {:ok, value}

      value when is_binary(value) ->
        {:error, "unsupported condition operator at config.predicate.operator"}

      _ ->
        {:error, "condition config.predicate.operator must name a supported operator"}
    end
  end

  defp target(payload, nil), do: {:ok, payload}

  defp target(payload, path) when is_binary(path) do
    value =
      path
      |> String.split(".", trim: true)
      |> Enum.reduce(payload, fn segment, current ->
        cond do
          is_map(current) -> Map.get(current, segment)
          is_list(current) -> array_entry(current, segment)
          true -> nil
        end
      end)

    {:ok, value}
  end

  defp target(_, _), do: {:error, "condition config.predicate.path must be a string or null"}

  defp array_entry(items, segment) do
    case array_index(segment) do
      {:ok, index} -> Enum.at(items, index)
      _ -> nil
    end
  end

  # Reject negative indexes and overflow before constructing an arbitrary-size integer.
  defp array_index("+" <> rest), do: unsigned_index(rest)
  defp array_index(segment), do: unsigned_index(segment)

  defp unsigned_index(<<digit, rest::binary>>) when digit in ?0..?9,
    do: index_digits(rest, digit - ?0)

  defp unsigned_index(_), do: :error
  defp index_digits("", index), do: {:ok, index}

  defp index_digits(<<digit, rest::binary>>, index)
       when digit in ?0..?9 and index <= div(@max_index - (digit - ?0), 10),
       do: index_digits(rest, index * 10 + digit - ?0)

  defp index_digits(_, _), do: :error

  defp compare("truthy", target, _), do: {:ok, truthy?(target)}
  defp compare("falsy", target, _), do: {:ok, not truthy?(target)}
  defp compare("eq", target, value), do: {:ok, json_equal?(target, value)}
  defp compare("neq", target, value), do: {:ok, not json_equal?(target, value)}

  defp compare("contains", target, value) when is_binary(target) and is_binary(value),
    do: {:ok, String.contains?(target, value)}

  defp compare("contains", target, _) when is_binary(target),
    do: {:error, "condition config.predicate.value must be a string for string contains"}

  defp compare("contains", target, value) when is_list(target),
    do: {:ok, Enum.any?(target, &json_equal?(&1, value))}

  defp compare("contains", _, _),
    do: {:error, "condition operator contains expects string or array input"}

  defp compare(operator, left, right) when is_number(left) and is_number(right) do
    {:ok,
     case operator do
       "gt" -> left > right
       "gte" -> left >= right
       "lt" -> left < right
       "lte" -> left <= right
     end}
  end

  defp compare(_, left, _) when not is_number(left),
    do: {:error, "condition operator expects numeric input"}

  defp compare(_, _, _), do: {:error, "condition config.predicate.value must be numeric"}

  defp json_equal?(left, right) when is_number(left) and is_number(right), do: left == right

  defp json_equal?([], []), do: true

  defp json_equal?([a | left], [b | right]),
    do: json_equal?(a, b) and json_equal?(left, right)

  defp json_equal?(left, right) when is_map(left) and is_map(right) do
    map_size(left) == map_size(right) and
      Enum.all?(left, fn {key, value} ->
        case Map.fetch(right, key) do
          {:ok, other} -> json_equal?(value, other)
          :error -> false
        end
      end)
  end

  defp json_equal?(left, right), do: left === right

  defp truthy?(nil), do: false
  defp truthy?(false), do: false
  defp truthy?(""), do: false
  defp truthy?(value) when is_number(value), do: value != 0
  defp truthy?(value) when is_list(value), do: value != []
  defp truthy?(value) when is_map(value), do: map_size(value) > 0
  defp truthy?(_), do: true
end
