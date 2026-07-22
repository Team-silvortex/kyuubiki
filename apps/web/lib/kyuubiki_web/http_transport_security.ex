defmodule KyuubikiWeb.HttpTransportSecurity do
  @moduledoc """
  Security policy for HTTP response headers and optional outbound HTTP helpers.
  """

  @protocol_options [invalid_response_headers: :error_terminate]

  def protocol_options, do: @protocol_options

  def descriptor do
    %{
      "schema_version" => "kyuubiki.http-transport-security/v1",
      "invalid_response_headers" => "error_terminate",
      "outbound_cookie_encoder" => "disabled",
      "mitigated_advisories" => ["CVE-2026-43966", "CVE-2026-43969"]
    }
  end
end
