defmodule KyuubikiWeb.RequestSizeGuardTest do
  use ExUnit.Case, async: true

  import Plug.Conn
  import Plug.Test

  alias KyuubikiWeb.RequestSizeGuard

  test "rejects an oversized declared body before parsing it" do
    declared_size = RequestSizeGuard.max_inline_json_bytes() + 1

    conn =
      conn(:post, "/api/v1/fem/heat-plane-quad-2d/jobs", "{}")
      |> delete_req_header("content-length")
      |> put_req_header("content-length", Integer.to_string(declared_size))
      |> put_req_header("content-type", "application/json")
      |> RequestSizeGuard.call([])

    assert conn.halted
    assert conn.status == 413

    assert %{
             "error" => "inline_json_payload_too_large",
             "payload_bytes" => ^declared_size,
             "max_inline_json_bytes" => 8_000_000,
             "recommended_transport" => "model_or_artifact_reference"
           } = Jason.decode!(conn.resp_body)
  end

  test "allows a body within the inline JSON contract" do
    conn =
      conn(:post, "/api/v1/fem/heat-plane-quad-2d/jobs", "{}")
      |> RequestSizeGuard.call([])

    refute conn.halted
    assert conn.status == nil
  end

  test "allows the dedicated large model artifact media type" do
    declared_size = RequestSizeGuard.max_inline_json_bytes() + 1

    conn =
      conn(:post, "/api/v1/model-artifacts", "{}")
      |> delete_req_header("content-length")
      |> put_req_header("content-length", Integer.to_string(declared_size))
      |> put_req_header("content-type", KyuubikiWeb.ModelArtifactStore.media_type())
      |> RequestSizeGuard.call([])

    refute conn.halted
    assert conn.status == nil
  end
end
