defmodule KyuubikiSdk.Auth do
  @moduledoc "Header-based auth descriptor for Kyuubiki SDK clients."

  @derive {Inspect, only: [:header_name]}
  defstruct [:header_name, :header_value]

  def access_token(token) when is_binary(token) do
    %__MODULE__{header_name: "x-kyuubiki-token", header_value: token}
  end

  def validate(nil), do: :ok

  def validate(%__MODULE__{header_name: name, header_value: value}) do
    cond do
      not valid_header_name?(name) -> {:error, "invalid authentication header name"}
      not valid_header_value?(value) -> {:error, "invalid authentication header value"}
      true -> :ok
    end
  end

  defp valid_header_name?(name) do
    is_binary(name) and byte_size(name) in 1..128 and
      String.match?(name, ~r/\A[a-zA-Z0-9-]+\z/)
  end

  defp valid_header_value?(value) do
    is_binary(value) and String.valid?(value) and byte_size(value) in 1..8192 and
      value
      |> String.to_charlist()
      |> Enum.all?(&(&1 in 0x21..0x7E))
  end
end
