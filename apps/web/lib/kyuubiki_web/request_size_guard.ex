defmodule KyuubikiWeb.RequestSizeGuard do
  @moduledoc false

  import Plug.Conn

  @max_inline_json_bytes 8_000_000

  def max_inline_json_bytes, do: @max_inline_json_bytes

  def init(options), do: options

  def call(conn, _options) do
    if inline_json?(conn) do
      case content_length(conn) do
        length when is_integer(length) and length > @max_inline_json_bytes -> reject(conn, length)
        _ -> conn
      end
    else
      conn
    end
  end

  def descriptor do
    %{
      "max_inline_json_bytes" => @max_inline_json_bytes,
      "oversize_status" => 413,
      "large_payload_mode" => "model_or_artifact_reference_required"
    }
  end

  defp content_length(conn) do
    case get_req_header(conn, "content-length") do
      [value] ->
        case Integer.parse(value) do
          {length, ""} -> length
          _ -> :invalid
        end

      _ ->
        :unknown
    end
  end

  defp inline_json?(conn) do
    conn
    |> get_req_header("content-type")
    |> Enum.any?(fn value ->
      value |> String.split(";", parts: 2) |> hd() |> String.trim() == "application/json"
    end)
  end

  defp reject(conn, length) do
    payload = %{
      "error" => "inline_json_payload_too_large",
      "payload_bytes" => length,
      "max_inline_json_bytes" => @max_inline_json_bytes,
      "recommended_transport" => "model_or_artifact_reference"
    }

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(413, Jason.encode!(payload))
    |> halt()
  end
end
