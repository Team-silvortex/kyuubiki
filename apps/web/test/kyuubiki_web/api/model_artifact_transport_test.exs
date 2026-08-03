defmodule KyuubikiWeb.Api.ModelArtifactTransportTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.ModelArtifactStore
  alias KyuubikiWeb.Persistence

  test "streams a model artifact, exposes metadata, and resolves its immutable reference" do
    model = %{
      "nodes" => [%{"id" => "n0", "x" => 0.0, "y" => 0.0}],
      "elements" => [%{"id" => "e0", "node_i" => 0, "node_j" => 0}]
    }

    encoded_model = Jason.encode!(model)

    upload =
      :post
      |> conn("/api/v1/model-artifacts", encoded_model)
      |> put_req_header("content-type", ModelArtifactStore.media_type())
      |> put_req_header("content-length", Integer.to_string(byte_size(encoded_model)))
      |> Router.call(@opts)

    assert upload.status == 201
    artifact = Jason.decode!(upload.resp_body)["artifact"]
    artifact_id = artifact["artifact_id"]
    assert artifact["sha256"] == artifact_id
    assert artifact["size_bytes"] == byte_size(encoded_model)
    assert byte_size(artifact_id) == 64
    on_exit(fn -> File.rm(artifact_path(artifact_id)) end)

    metadata =
      :get
      |> conn("/api/v1/model-artifacts/#{artifact_id}")
      |> Router.call(@opts)

    assert metadata.status == 200
    assert Jason.decode!(metadata.resp_body)["artifact"]["artifact_id"] == artifact_id

    assert {:ok, resolved} =
             ModelArtifactStore.resolve_model_params(%{
               "model_artifact_ref" => artifact,
               "project_id" => "project-artifact"
             })

    assert resolved["nodes"] == model["nodes"]
    assert resolved["elements"] == model["elements"]
    assert resolved["project_id"] == "project-artifact"
  end

  test "rejects oversized artifact declarations before reading the body" do
    declared_size = ModelArtifactStore.descriptor()["max_artifact_bytes"] + 1

    conn =
      conn(:post, "/api/v1/model-artifacts", "{}")
      |> delete_req_header("content-length")
      |> put_req_header("content-length", Integer.to_string(declared_size))
      |> put_req_header("content-type", ModelArtifactStore.media_type())
      |> Router.call(@opts)

    assert conn.status == 413
    assert Jason.decode!(conn.resp_body)["error"] == "model_artifact_too_large"
  end

  defp artifact_path(artifact_id) do
    Persistence.data_dir()
    |> Path.join("model-artifacts/sha256")
    |> Path.join(String.slice(artifact_id, 0, 2))
    |> Path.join("#{artifact_id}.json")
  end
end
