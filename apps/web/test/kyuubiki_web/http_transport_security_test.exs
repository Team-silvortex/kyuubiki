defmodule KyuubikiWeb.HttpTransportSecurityTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.HttpTransportSecurity
  alias KyuubikiWeb.Workloads

  test "Plug rejects response header delimiter injection before transport" do
    conn = Plug.Test.conn(:get, "/")

    assert_raise Plug.Conn.InvalidHeaderError, fn ->
      Plug.Conn.put_resp_header(
        conn,
        "content-disposition",
        "attachment\r\nx-injected: true"
      )
    end
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

  test "Orchestra declares finite HTTP and WebSocket protocol limits" do
    descriptor = HttpTransportSecurity.descriptor()

    assert descriptor["adapter"] == "bandit"
    assert descriptor["response_header_validation"] == "plug_conn"
    assert descriptor["limits"]["http_1_max_header_bytes"] == 10_000
    assert descriptor["limits"]["http_1_max_header_count"] == 50
    assert descriptor["limits"]["http_2_max_header_block_bytes"] == 50_000
    assert descriptor["limits"]["websocket_max_frame_bytes"] == 8_000_000
  end

  test "dependency lock excludes the Cowboy and Cowlib protocol stack" do
    lock = File.read!("mix.lock")

    assert lock =~ ~s("bandit":)

    for dependency <- ~w(cowboy cowboy_telemetry cowlib plug_cowboy ranch) do
      refute lock =~ ~s("#{dependency}":)
    end

    assert HttpTransportSecurity.descriptor()["legacy_adapter_dependencies"] == "removed"
  end
end
