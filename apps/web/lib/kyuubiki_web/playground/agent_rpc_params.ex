defmodule KyuubikiWeb.Playground.AgentRpcParams do
  @moduledoc false

  # Match JSON's control-envelope names before routing, probing and journaling.
  # Keep task/model payloads by reference rather than re-encoding large inputs.
  def normalize("run_operator_task_ir", params) when is_map(params) and not is_struct(params) do
    Enum.reduce_while(params, {:ok, %{}}, fn {key, value}, {:ok, normalized} ->
      with {:ok, key} <- parameter_key(key),
           false <- Map.has_key?(normalized, key) do
        {:cont, {:ok, Map.put(normalized, key, parameter_value(key, value))}}
      else
        true -> {:halt, invalid("ambiguous TaskIR parameter keys")}
        {:error, _} = error -> {:halt, error}
      end
    end)
  end

  def normalize("run_operator_task_ir", _params), do: invalid("invalid TaskIR parameter object")

  def normalize(_method, params), do: {:ok, params}

  defp parameter_key(key) when is_binary(key), do: {:ok, key}
  defp parameter_key(key) when is_atom(key), do: {:ok, Atom.to_string(key)}
  defp parameter_key(_key), do: invalid("invalid TaskIR parameter key")

  defp parameter_value("mode", value)
       when is_atom(value) and value not in [nil, true, false],
       do: Atom.to_string(value)

  defp parameter_value(_key, value), do: value

  defp invalid(reason), do: {:error, {:request_encoding_failed, reason}}
end
