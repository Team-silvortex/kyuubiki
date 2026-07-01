defmodule KyuubikiWeb.CanonicalJson do
  @moduledoc false

  @spec encode!(term()) :: String.t()
  def encode!(value) when is_map(value) do
    entries =
      value
      |> Map.to_list()
      |> Enum.sort_by(fn {key, _value} -> to_string(key) end)
      |> Enum.map(fn {key, nested} -> Jason.encode!(to_string(key)) <> ":" <> encode!(nested) end)

    "{" <> Enum.join(entries, ",") <> "}"
  end

  def encode!(values) when is_list(values) do
    "[" <> (values |> Enum.map(&encode!/1) |> Enum.join(",")) <> "]"
  end

  def encode!(value) when is_binary(value), do: Jason.encode!(value)
  def encode!(value) when is_integer(value), do: Integer.to_string(value)

  def encode!(value) when is_float(value) do
    :erlang.float_to_binary(value, [:compact, {:decimals, 15}])
  end

  def encode!(value) when is_boolean(value), do: Jason.encode!(value)
  def encode!(nil), do: "null"
end
