defmodule KyuubikiWeb.Api.ResultArtifactTransportTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Persistence
  alias KyuubikiWeb.ResultArtifactStore

  test "streams an agent result artifact and exposes immutable content" do
    encoded_result = Jason.encode!(%{"nodes" => [%{"index" => 0, "temperature" => 30.0}]})

    upload =
      :post
      |> conn("/api/v1/result-artifacts", encoded_result)
      |> put_req_header("content-type", ResultArtifactStore.media_type())
      |> put_req_header("content-length", Integer.to_string(byte_size(encoded_result)))
      |> put_req_header("x-kyuubiki-agent-id", "result-artifact-test")
      |> put_req_header("x-kyuubiki-cluster-ts", System.system_time(:millisecond) |> to_string())
      |> put_req_header("x-kyuubiki-cluster-nonce", "result-artifact-upload")
      |> Router.call(@opts)

    assert upload.status == 201
    artifact = Jason.decode!(upload.resp_body)["artifact"]
    artifact_id = artifact["artifact_id"]
    assert artifact["schema_version"] == "kyuubiki.result-artifact-ref/v1"
    assert artifact["size_bytes"] == byte_size(encoded_result)
    on_exit(fn -> File.rm(artifact_path(artifact_id)) end)

    metadata =
      :get
      |> conn("/api/v1/result-artifacts/#{artifact_id}")
      |> Router.call(@opts)

    assert metadata.status == 200

    content =
      :get
      |> conn("/api/v1/result-artifacts/#{artifact_id}/content")
      |> Router.call(@opts)

    assert content.status == 200
    assert content.resp_body == encoded_result
    assert get_resp_header(content, "x-kyuubiki-sha256") == [artifact_id]
  end

  defp artifact_path(artifact_id) do
    Persistence.data_dir()
    |> Path.join("result-artifacts/sha256")
    |> Path.join(String.slice(artifact_id, 0, 2))
    |> Path.join("#{artifact_id}.json")
  end
end
