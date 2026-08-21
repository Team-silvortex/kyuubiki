defmodule KyuubikiWeb.HttpTransportSecurityTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.HttpTransportSecurity
  alias KyuubikiWeb.Workloads

  test "Cowboy terminates responses containing injected header delimiters" do
    options = HttpTransportSecurity.protocol_options() |> Map.new()

    assert :ok ==
             :cowboy_http.validate_response_headers(
               %{"content-disposition" => "attachment; filename=study.json"},
               options
             )

    assert :error_terminate ==
             :cowboy_http.validate_response_headers(
               %{"content-disposition" => "attachment\r\nx-injected: true"},
               options
             )
  end

  test "project bundle filenames cannot inject response headers" do
    filename =
      Workloads.bundle_filename(%{
        "project_id" => "project-safe",
        "name" => "study\r\nx-injected: true"
      })

    assert filename == "study-x-injected-true.kyuubiki.json"
    refute filename =~ "\r"
    refute filename =~ "\n"
  end

  test "backend source does not invoke cowlib client cookie serialization" do
    references =
      "lib/**/*.ex"
      |> Path.wildcard()
      |> Enum.filter(fn path ->
        source = File.read!(path)
        String.contains?(source, [":cow_cookie", "cow_cookie:"])
      end)

    assert references == []
    assert HttpTransportSecurity.descriptor()["outbound_cookie_encoder"] == "disabled"
  end

  test "backend source does not invoke cowlib Link header serialization" do
    references =
      "lib/**/*.ex"
      |> Path.wildcard()
      |> Enum.filter(fn path ->
        source = File.read!(path)
        String.contains?(source, [":cow_link", "cow_link:"])
      end)

    assert references == []
    assert HttpTransportSecurity.descriptor()["outbound_link_encoder"] == "disabled"
    assert "CVE-2026-43971" in HttpTransportSecurity.descriptor()["mitigated_advisories"]
  end
end
