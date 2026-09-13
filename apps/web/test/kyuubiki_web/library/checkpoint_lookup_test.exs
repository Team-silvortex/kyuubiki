defmodule KyuubikiWeb.Library.CheckpointLookupTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  defp get_json(path), do: conn(:get, path) |> Router.call(@opts)

  test "lookup follows the configured read authentication policy" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "receipt-read-test-secret",
      protect_reads?: true
    )

    path = "/api/v1/checkpoints/model/parent/receipt-lookup-auth-0001"
    assert get_json(path).status == 401

    response =
      conn(:get, path)
      |> put_req_header("x-kyuubiki-token", "receipt-read-test-secret")
      |> Router.call(@opts)

    assert response.status == 200
    assert Jason.decode!(response.resp_body) == %{"checkpoint" => %{"status" => "unknown"}}
  end

  test "read-only lookup reconciles a commit without its payload or a second write" do
    {:ok, project} = Library.create_project(%{"name" => "Lookup fixture"})
    id = "receipt-lookup-model-0001"

    input = %{
      "request_id" => id,
      "name" => "Private name",
      "kind" => "truss_3d",
      "payload" => %{"private_geometry" => [1, 2, 3]}
    }

    parent = project["project_id"]
    assert Library.get_checkpoint(:model, parent, id) == {:ok, %{"status" => "unknown"}}
    {:ok, model} = Library.create_model(parent, input)
    {:ok, checkpoint} = Library.get_checkpoint(:model, parent, id)

    assert checkpoint == %{
             "status" => "committed",
             "project_id" => parent,
             "model_id" => model["model_id"],
             "version_id" => model["latest_version_id"]
           }

    conn = get_json("/api/v1/checkpoints/model/#{parent}/#{id}")
    assert conn.status == 200
    assert get_resp_header(conn, "cache-control") == ["no-store"]
    assert Jason.decode!(conn.resp_body) == %{"checkpoint" => checkpoint}
    refute conn.resp_body =~ "private"
    assert Library.get_checkpoint(:version, parent, id) == {:ok, %{"status" => "unknown"}}

    assert Library.get_checkpoint(:model, "different-parent", id) ==
             {:ok, %{"status" => "unknown"}}

    assert {:ok, [_]} = Library.list_versions(model["model_id"])

    {:ok, version} = Library.create_version(model["model_id"], input)

    assert {:ok, %{"status" => "committed", "version_id" => version_id}} =
             Library.get_checkpoint(:version, model["model_id"], id)

    assert version_id == version["version_id"]
    {:ok, _} = Library.delete_version(version_id)

    assert {:ok, %{"status" => "deleted"}} =
             Library.get_checkpoint(:version, model["model_id"], id)

    assert {:ok, [_]} = Library.list_versions(model["model_id"])
    {:ok, _} = Library.delete_model(model["model_id"])
    assert {:ok, %{"status" => "deleted"}} = Library.get_checkpoint(:model, parent, id)
  end

  test "invalid lookup identity is rejected and never creates records" do
    for operation <- [:other, "model", nil],
        do:
          assert(
            {:error, :invalid_checkpoint_request} =
              Library.get_checkpoint(operation, "p", "valid-request-id-0001")
          )

    for id <- [nil, "short", "contains spaces 1234", String.duplicate("x", 129)],
        do: assert({:error, _} = Library.get_checkpoint(:model, "p", id))

    assert get_json("/api/v1/checkpoints/other/p/valid-request-id-0001").status == 422
    assert get_json("/api/v1/checkpoints/model/p/short").status == 422
    assert {:ok, []} = Library.list_projects()
  end
end
