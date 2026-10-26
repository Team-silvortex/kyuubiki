defmodule KyuubikiWeb.Playground.AgentRpcJson do
  @moduledoc false

  # Keep object pairs until uniqueness is checked, including decoded escaped keys.
  # Numbers stay in Jason's native representation; no JSON re-encoding is involved.
  @spec decode(binary()) :: {:ok, term()} | {:error, :invalid_json | :duplicate_json_key}
  def decode(payload) when is_binary(payload) do
    case Jason.decode(payload, objects: :ordered_objects) do
      {:ok, value} -> unique_value(value)
      {:error, _error} -> {:error, :invalid_json}
    end
  end

  defp unique_value(%Jason.OrderedObject{values: fields}) do
    Enum.reduce_while(fields, {:ok, %{}}, fn {key, value}, {:ok, fields} ->
      if Map.has_key?(fields, key) do
        {:halt, {:error, :duplicate_json_key}}
      else
        case unique_value(value) do
          {:ok, decoded} -> {:cont, {:ok, Map.put(fields, key, decoded)}}
          {:error, _} = error -> {:halt, error}
        end
      end
    end)
  end

  defp unique_value(values) when is_list(values) do
    case Enum.reduce_while(values, {:ok, []}, fn value, {:ok, values} ->
           case unique_value(value) do
             {:ok, decoded} -> {:cont, {:ok, [decoded | values]}}
             {:error, _} = error -> {:halt, error}
           end
         end) do
      {:ok, reversed} -> {:ok, Enum.reverse(reversed)}
      {:error, _} = error -> error
    end
  end

  defp unique_value(value), do: {:ok, value}
end
