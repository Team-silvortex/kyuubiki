defmodule KyuubikiSdk.Error do
  @moduledoc "Structured SDK error container."

  defexception [:type, :message, :status_code, :body, :code]

  def http(status_code, body) do
    %__MODULE__{
      type: :http,
      status_code: status_code,
      body: body,
      message: "http #{status_code}: #{body}"
    }
  end

  def rpc(message, opts \\ []) do
    code = Keyword.get(opts, :code)

    %__MODULE__{
      type: :rpc,
      code: code,
      message: if(is_binary(code), do: "#{code}: #{message}", else: message)
    }
  end

  def transport(message), do: %__MODULE__{type: :transport, message: message}
  def timeout(message), do: %__MODULE__{type: :timeout, message: message}
end
