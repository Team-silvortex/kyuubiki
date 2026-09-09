defmodule KyuubikiWeb.TestSupport.ProjectExecutionApi do
  @moduledoc false

  import ExUnit.Assertions
  import Plug.Conn
  import Plug.Test

  alias KyuubikiWeb.Router
  alias KyuubikiWeb.TestSupport.WorkflowApi

  @workflow "workflow.truss-2d-summary-json"

  def request(method, path, payload, status) do
    response = route(method, path, payload)

    assert response.status == status,
           "#{method} #{path}: #{response.status} #{response.resp_body}"

    Jason.decode!(response.resp_body)
  end

  def route(method, path, payload) do
    request =
      if is_nil(payload),
        do: conn(method, path),
        else:
          conn(method, path, Jason.encode!(payload))
          |> put_req_header("content-type", "application/json")

    Router.call(request, Router.init([]))
  end

  def create_study(name \\ "Project execution chain") do
    project = request(:post, "/api/v1/projects", %{"name" => name}, 201)["project"]

    model =
      request(
        :post,
        "/api/v1/projects/#{project["project_id"]}/models",
        %{
          "name" => "Axial rod",
          "kind" => "truss_2d",
          "payload" => rod()
        },
        201
      )["model"]

    {project, model}
  end

  def rod(load \\ 2_000) do
    %{
      "kind" => "truss_2d",
      "model_schema_version" => "kyuubiki.model/v1",
      "nodes" => [
        %{
          "id" => "left",
          "x" => 0,
          "y" => 0,
          "fix_x" => true,
          "fix_y" => true,
          "load_x" => 0,
          "load_y" => 0
        },
        %{
          "id" => "right",
          "x" => 2,
          "y" => 0,
          "fix_x" => false,
          "fix_y" => true,
          "load_x" => load,
          "load_y" => 0
        }
      ],
      "elements" => [
        %{
          "id" => "rod",
          "node_i" => 0,
          "node_j" => 1,
          "area" => 0.002,
          "youngs_modulus" => 200.0e9
        }
      ]
    }
  end

  def context(project, model),
    do: %{"project_id" => project["project_id"], "model_version_id" => model["latest_version_id"]}

  def catalog_path, do: "/api/v1/workflows/catalog/#{@workflow}/jobs"

  def catalog_input(model, context),
    do: Map.put(context, "input_artifacts", %{"truss_2d_model" => model})

  def completed(submission) do
    payload = terminal(submission)
    assert payload["job"]["status"] == "completed", inspect(payload["job"])
    payload
  end

  def terminal(submission),
    do: WorkflowApi.wait_for_job(submission["job"]["job_id"], Router.init([]), 2_000)

  def bundle(project),
    do: request(:get, "/api/v1/projects/#{project["project_id"]}/bundle", nil, 200)

  def assert_lineage(job, project, version_id) do
    assert job["project_id"] == project["project_id"]
    assert job["model_version_id"] == version_id
    if version_id, do: assert(job["simulation_case_id"] == version_id)
  end

  def summary(payload) do
    payload["result"]["artifacts"]["json_output.json"]["content"] |> Jason.decode!()
  end
end
