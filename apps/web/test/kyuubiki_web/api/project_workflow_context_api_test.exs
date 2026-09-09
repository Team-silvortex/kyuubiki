defmodule KyuubikiWeb.Api.ProjectWorkflowContextApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  import KyuubikiWeb.TestSupport.ProjectExecutionApi

  alias KyuubikiWeb.Analysis
  alias KyuubikiWeb.Orchestra.Engine

  setup do
    # Transport fixtures verify lineage, not the numerical solver (covered by the live suite).
    frame = %{
      "ok" => true,
      "result" => %{
        "nodes" => [],
        "elements" => [],
        "max_displacement" => 1.0e-5,
        "max_stress" => 1.0e6
      }
    }

    {:ok, pid} = start_fake_agent_sessions(List.duplicate([frame], 3))
    configure_fake_agent_pool(await_fake_agent_port())
    on_exit(fn -> if Process.alive?(pid), do: Process.exit(pid, :kill) end)
    :ok
  end

  test "catalog jobs retain selected project and checkpoint through execution and export" do
    {project, model} = create_study()
    submitted = request(:post, catalog_path(), catalog_input(rod(), context(project, model)), 202)
    finished = completed(submitted)
    assert_lineage(submitted["job"], project, model["latest_version_id"])
    assert_lineage(finished["job"], project, model["latest_version_id"])
    exported = bundle(project)
    assert Enum.map(exported["jobs"], & &1["job_id"]) == [submitted["job"]["job_id"]]
    assert Enum.map(exported["results"], & &1["job_id"]) == [submitted["job"]["job_id"]]
  end

  test "project-only catalog jobs do not require a checkpoint" do
    {project, _model} = create_study()
    input = catalog_input(rod(), %{"project_id" => project["project_id"]})
    finished = request(:post, catalog_path(), input, 202) |> completed()
    assert_lineage(finished["job"], project, nil)
    assert length(bundle(project)["results"]) == 1
  end

  test "checkpoint ownership is authoritative even when the caller supplies another project" do
    {project, model} = create_study()
    {other, _} = create_study("Unrelated project")
    input = catalog_input(rod(), context(other, model))
    finished = request(:post, catalog_path(), input, 202) |> completed()
    assert_lineage(finished["job"], project, model["latest_version_id"])
    assert length(bundle(project)["results"]) == 1
    assert bundle(other)["jobs"] == []
    assert bundle(other)["results"] == []
  end

  test "atom-key callers retain the same catalog context as JSON callers" do
    {project, model} = create_study()

    {:ok, submitted} =
      Analysis.submit_catalog_workflow("workflow.truss-2d-summary-json", %{
        input_artifacts: %{truss_2d_model: rod()},
        project_id: project["project_id"],
        model_version_id: model["latest_version_id"]
      })

    finished = completed(submitted)
    assert_lineage(finished["job"], project, model["latest_version_id"])
  end

  test "missing or deleted checkpoints reject before job creation and a valid retry succeeds" do
    {project, model} = create_study()
    {:ok, deleted} = Library.create_version(model["model_id"], %{"payload" => rod(4_000)})
    request(:delete, "/api/v1/model-versions/#{deleted["version_id"]}", nil, 200)
    before_jobs = Store.list()

    for version_id <- ["missing-checkpoint", deleted["version_id"]] do
      input =
        catalog_input(rod(), %{
          "project_id" => project["project_id"],
          "model_version_id" => version_id
        })

      response = route(:post, catalog_path(), input)

      # Settle incorrectly accepted work too, so a failing regression cannot leak into another test.
      if response.status == 202, do: terminal(Jason.decode!(response.resp_body))
      assert response.status == 422

      assert Jason.decode!(response.resp_body)["error"] ==
               inspect({:model_version_not_found, version_id})

      assert Store.list() == before_jobs
      assert bundle(project)["results"] == []
    end

    finished =
      request(:post, catalog_path(), catalog_input(rod(), context(project, model)), 202)
      |> completed()

    assert_lineage(finished["job"], project, model["latest_version_id"])
    assert length(Store.list()) == 1
    assert length(bundle(project)["results"]) == 1
  end

  @tag :missing_job
  test "missing job detail and polling endpoints use the same not-found contract" do
    for path <- ["/api/v1/jobs/missing", "/api/v1/jobs/missing/status"] do
      assert request(:get, path, nil, 404) == %{"error" => "job_not_found"}
    end

    assert Store.list() == []
  end

  @tag :missing_job
  test "deleting a completed workflow makes both poll endpoints not found and removes its result" do
    {project, model} = create_study()

    finished =
      request(:post, catalog_path(), catalog_input(rod(), context(project, model)), 202)
      |> completed()

    id = finished["job"]["job_id"]
    request(:delete, "/api/v1/jobs/#{id}", nil, 200)
    assert request(:get, "/api/v1/jobs/#{id}", nil, 404) == %{"error" => "job_not_found"}
    assert request(:get, "/api/v1/jobs/#{id}/status", nil, 404) == %{"error" => "job_not_found"}
    request(:get, "/api/v1/results/#{id}", nil, 404)
    request(:delete, "/api/v1/jobs/#{id}", nil, 404)
    assert bundle(project)["jobs"] == []
    assert bundle(project)["results"] == []
  end

  test "catalog and raw graph entry points preserve the same version context" do
    {project, model} = create_study()
    {:ok, graph} = Engine.workflow_graph_by_id("workflow.truss-2d-summary-json")
    input = catalog_input(rod(), context(project, model))
    catalog = request(:post, catalog_path(), input, 202) |> completed()
    {:ok, submitted} = Analysis.submit_workflow_graph(Map.put(input, "graph", graph))
    raw = completed(submitted)
    assert_lineage(raw["job"], project, model["latest_version_id"])
    assert_lineage(catalog["job"], project, model["latest_version_id"])
    assert summary(raw) == summary(catalog)
    assert length(bundle(project)["results"]) == 2
  end
end
