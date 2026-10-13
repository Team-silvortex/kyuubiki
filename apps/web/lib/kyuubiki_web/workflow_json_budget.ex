defmodule KyuubikiWeb.WorkflowJsonBudget do
  @moduledoc false

  # Keep these wire-data limits aligned with engine/workflow_security.rs.
  def validate(value, label, max_nodes), do: check(value, label, {max_nodes, 1_000_000})
  def validate_output(value, label), do: check(value, label, {500_000, 500_000})

  def within_encoded_budget?(value, max_bytes, max_nodes) do
    validate(value, "checkpoint", max_nodes) == :ok and
      match?({:ok, _}, encoded_size(value, 0, max_bytes))
  end

  # Stop before allocating a large JSON copy; punctuation is counted conservatively.
  defp encoded_size(_value, count, limit) when count > limit, do: :too_large

  defp encoded_size(value, count, limit) when is_map(value) do
    Enum.reduce_while(value, {:ok, count + 2}, fn {key, item}, {:ok, bytes} ->
      result =
        with {:ok, bytes} <- encoded_size(key, bytes + 2, limit),
             do: encoded_size(item, bytes, limit)

      size_step(result, limit)
    end)
    |> size_result(limit)
  end

  defp encoded_size(value, count, limit) when is_list(value) do
    Enum.reduce_while(value, {:ok, count + 2}, fn item, {:ok, bytes} ->
      size_step(encoded_size(item, bytes + 1, limit), limit)
    end)
    |> size_result(limit)
  end

  defp encoded_size(value, count, limit) do
    if is_binary(value) and byte_size(value) + count > limit,
      do: :too_large,
      else: size_result({:ok, count + byte_size(Jason.encode!(value))}, limit)
  end

  defp size_step(result, limit) do
    case size_result(result, limit) do
      {:ok, _} = result -> {:cont, result}
      :too_large -> {:halt, :too_large}
    end
  end

  defp size_result({:ok, bytes} = result, limit) when bytes <= limit, do: result
  defp size_result(_, _), do: :too_large

  defp check(value, label, budget) do
    case visit(value, 0, 0, budget) do
      {:ok, _count} -> :ok
      {:error, reason} -> {:error, "#{label} #{reason}"}
    end
  end

  defp visit(_value, depth, _count, _limit) when depth > 64,
    do: {:error, "exceeds JSON depth security budget"}

  defp visit(_value, _depth, count, {limit, _string_limit}) when count >= limit,
    do: {:error, "exceeds JSON node security budget"}

  defp visit(value, depth, count, limit) when is_map(value) and not is_struct(value) do
    Enum.reduce_while(value, {:ok, count + 1}, fn {key, item}, {:ok, count} ->
      result = with :ok <- text(key, "object key", 256), do: visit(item, depth + 1, count, limit)
      step(result)
    end)
  end

  defp visit(value, depth, count, limit) when is_list(value),
    do: visit_list(value, depth + 1, count + 1, limit)

  defp visit(value, _depth, count, {_nodes, string_limit}) when is_binary(value) do
    with :ok <- text(value, "string", string_limit), do: {:ok, count + 1}
  end

  defp visit(value, _depth, count, _limit)
       when is_number(value) or is_boolean(value) or is_nil(value),
       do: {:ok, count + 1}

  defp visit(_value, _depth, _count, _limit), do: {:error, "must contain only JSON values"}

  defp visit_list([], _depth, count, _limit), do: {:ok, count}

  defp visit_list([item | rest], depth, count, limit) do
    with {:ok, count} <- visit(item, depth, count, limit),
         do: visit_list(rest, depth, count, limit)
  end

  defp visit_list(_tail, _depth, _count, _limit), do: {:error, "must contain only JSON arrays"}

  defp text(value, kind, limit) when is_binary(value) do
    cond do
      byte_size(value) > limit -> {:error, "#{kind} exceeds length security budget"}
      :binary.match(value, <<0>>) != :nomatch -> {:error, "#{kind} contains NUL characters"}
      not String.valid?(value) -> {:error, "#{kind} must be valid UTF-8"}
      true -> :ok
    end
  end

  defp text(_value, kind, _limit), do: {:error, "#{kind} must be a string"}
  defp step({:ok, _} = result), do: {:cont, result}
  defp step({:error, _} = result), do: {:halt, result}
end
