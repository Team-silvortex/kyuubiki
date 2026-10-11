defmodule KyuubikiWeb.TaskDigestV2 do
  @moduledoc false
  @encoding "kyuubiki.task-digest/typed-ieee754-v2"
  @min_integer -9_223_372_036_854_775_808
  @max_integer 18_446_744_073_709_551_615

  def encoding, do: @encoding

  def compute(task, fields) do
    hash = :crypto.hash_init(:sha256) |> :crypto.hash_update(@encoding <> <<0>>)
    digest = encode(hash, Map.take(task, fields), 0) |> :crypto.hash_final()
    {:ok, Base.encode16(digest, case: :lower)}
  rescue
    error in ArgumentError -> {:error, {:invalid_operator_task_digest_encoding, error.message}}
  end

  defp encode(_hash, _value, depth) when depth > 128,
    do: raise(ArgumentError, "typed TaskIR digest exceeds maximum depth 128")

  defp encode(hash, nil, _depth), do: :crypto.hash_update(hash, "n")
  defp encode(hash, true, _depth), do: :crypto.hash_update(hash, "t")
  defp encode(hash, false, _depth), do: :crypto.hash_update(hash, "f")
  defp encode(hash, value, _depth) when is_binary(value), do: bytes(hash, "s", value)

  defp encode(hash, value, _depth)
       when is_integer(value) and value >= @min_integer and value <= @max_integer,
       do: bytes(hash, "i", Integer.to_string(value))

  defp encode(hash, value, _depth) when is_float(value),
    do: :crypto.hash_update(hash, <<?d, value::float-64>>)

  defp encode(hash, values, depth) when is_list(values) do
    Enum.reduce(values, header(hash, "a", length(values)), &encode(&2, &1, depth + 1))
  end

  defp encode(hash, values, depth) when is_map(values) do
    values
    |> Enum.sort_by(fn {key, _value} -> key end)
    |> Enum.reduce(header(hash, "o", map_size(values)), fn {key, value}, hash ->
      unless is_binary(key), do: raise(ArgumentError, "typed TaskIR digest requires string keys")
      encode(bytes(hash, "s", key), value, depth + 1)
    end)
  end

  defp encode(_hash, _value, _depth),
    do: raise(ArgumentError, "typed TaskIR digest requires JSON values and int64/uint64 integers")

  defp header(hash, tag, count), do: :crypto.hash_update(hash, [tag, <<count::unsigned-64>>])

  defp bytes(hash, tag, value) do
    unless String.valid?(value), do: raise(ArgumentError, "typed TaskIR digest requires UTF-8")
    hash |> header(tag, byte_size(value)) |> :crypto.hash_update(value)
  end
end
