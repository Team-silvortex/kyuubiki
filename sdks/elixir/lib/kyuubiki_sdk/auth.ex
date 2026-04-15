defmodule KyuubikiSdk.Auth do
  @moduledoc "Header-based auth descriptor for Kyuubiki SDK clients."

  defstruct [:header_name, :header_value]

  def access_token(token) when is_binary(token) do
    %__MODULE__{header_name: "x-kyuubiki-token", header_value: token}
  end
end
