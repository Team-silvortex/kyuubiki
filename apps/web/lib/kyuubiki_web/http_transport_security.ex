defmodule KyuubikiWeb.HttpTransportSecurity do
  @moduledoc """
  Explicit protocol limits for the Orchestra HTTP server.
  """

  @http_1_options [
    max_request_line_length: 10_000,
    max_header_length: 10_000,
    max_header_count: 50
  ]
  @http_2_options [
    max_header_block_size: 50_000,
    max_reset_stream_rate: {500, 10_000}
  ]
  @websocket_options [
    max_frame_size: 8_000_000,
    max_fragmented_message_size: 8_000_000,
    max_inflate_ratio: 25
  ]

  def server_options do
    [
      http_1_options: @http_1_options,
      http_2_options: @http_2_options,
      websocket_options: @websocket_options
    ]
  end

  def descriptor do
    %{
      "schema_version" => "kyuubiki.http-transport-security/v1",
      "adapter" => "bandit",
      "response_header_validation" => "plug_conn",
      "legacy_adapter_dependencies" => "removed",
      "resolved_by_dependency_removal" => ["CVE-2026-43966", "CVE-2026-43971"],
      "limits" => %{
        "http_1_max_header_bytes" => @http_1_options[:max_header_length],
        "http_1_max_header_count" => @http_1_options[:max_header_count],
        "http_2_max_header_block_bytes" => @http_2_options[:max_header_block_size],
        "websocket_max_frame_bytes" => @websocket_options[:max_frame_size]
      }
    }
  end
end
