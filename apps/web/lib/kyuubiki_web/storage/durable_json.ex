defmodule KyuubikiWeb.Storage.DurableJson do
  @moduledoc false

  # Persistence keeps Jason's lossless float spelling, unlike TaskIR's fixed-decimal digest.
  def encode!(value) do
    value
    |> ordered_value()
    |> Jason.encode!(maps: :strict)
  end

  defp ordered_value(value) when is_map(value) do
    value
    |> Enum.map(fn {key, item} -> {json_key(key), ordered_value(item)} end)
    |> Enum.sort_by(&elem(&1, 0))
    |> Jason.OrderedObject.new()
  end

  defp ordered_value(value) when is_list(value), do: Enum.map(value, &ordered_value/1)
  defp ordered_value(value), do: value

  # Jason uses Atom.to_string for object keys; String.Chars renders nil as "".
  defp json_key(key) when is_atom(key), do: Atom.to_string(key)
  defp json_key(key), do: to_string(key)
end
