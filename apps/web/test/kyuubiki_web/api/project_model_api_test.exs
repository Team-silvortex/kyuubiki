defmodule KyuubikiWeb.Api.ProjectModelApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "supports CRUD for projects, models, and model versions" do
    create_project_conn =
      :post
      |> conn(
        "/api/v1/projects",
        Jason.encode!(%{"name" => "Bridge Study", "description" => "macOS local"})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create_project_conn.status == 201
    project = Jason.decode!(create_project_conn.resp_body)["project"]
    project_id = project["project_id"]

    create_model_conn =
      :post
      |> conn(
        "/api/v1/projects/#{project_id}/models",
        Jason.encode!(%{
          "name" => "Roof Truss",
          "kind" => "truss_2d",
          "material" => "Steel",
          "model_schema_version" => "kyuubiki.model/v1",
          "payload" => %{
            "kind" => "truss_2d",
            "model_schema_version" => "kyuubiki.model/v1",
            "name" => "Roof Truss",
            "material" => "Steel",
            "youngs_modulus_gpa" => 210,
            "nodes" => [],
            "elements" => []
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create_model_conn.status == 201
    model = Jason.decode!(create_model_conn.resp_body)["model"]
    model_id = model["model_id"]
    assert model["latest_version_number"] == 1

    create_version_conn =
      :post
      |> conn(
        "/api/v1/models/#{model_id}/versions",
        Jason.encode!(%{
          "name" => "Checkpoint A",
          "payload" => %{
            "kind" => "truss_2d",
            "model_schema_version" => "kyuubiki.model/v1",
            "name" => "Roof Truss v2",
            "material" => "Steel",
            "youngs_modulus_gpa" => 210,
            "nodes" => [%{"id" => "n0", "x" => 0.0, "y" => 0.0}],
            "elements" => []
          }
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert create_version_conn.status == 201
    version = Jason.decode!(create_version_conn.resp_body)["version"]
    assert version["version_number"] == 2

    projects_conn =
      :get
      |> conn("/api/v1/projects")
      |> Router.call(@opts)

    assert projects_conn.status == 200
    projects = Jason.decode!(projects_conn.resp_body)["projects"]
    assert length(projects) == 1
    assert hd(projects)["models"] |> length() == 1

    get_model_conn =
      :get
      |> conn("/api/v1/models/#{model_id}")
      |> Router.call(@opts)

    assert get_model_conn.status == 200
    returned_model = Jason.decode!(get_model_conn.resp_body)["model"]
    assert length(returned_model["versions"]) == 2

    update_project_conn =
      :patch
      |> conn(
        "/api/v1/projects/#{project_id}",
        Jason.encode!(%{"name" => "Bridge Study Updated"})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert update_project_conn.status == 200

    assert Jason.decode!(update_project_conn.resp_body)["project"]["name"] ==
             "Bridge Study Updated"

    delete_version_conn =
      :delete
      |> conn("/api/v1/model-versions/#{version["version_id"]}")
      |> Router.call(@opts)

    assert delete_version_conn.status == 200

    delete_model_conn =
      :delete
      |> conn("/api/v1/models/#{model_id}")
      |> Router.call(@opts)

    assert delete_model_conn.status == 200

    delete_project_conn =
      :delete
      |> conn("/api/v1/projects/#{project_id}")
      |> Router.call(@opts)

    assert delete_project_conn.status == 200
  end
end
