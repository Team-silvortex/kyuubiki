defmodule KyuubikiWeb.Api.ProjectVersionChainApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "creating a model preserves its name independently of the initial checkpoint label" do
    {project, model} = create_project_model()
    assert model["name"] == "Copper conductor"
    assert fetched_model(model)["name"] == "Copper conductor"

    [initial] = versions(model)
    assert initial["name"] == "Initial version"
    assert initial["payload"] == model["payload"]
    assert initial["version_id"] == model["latest_version_id"]
    assert hd(bundle(project)["models"])["name"] == "Copper conductor"
  end

  test "deleting the latest checkpoint repairs references without reverting the working model" do
    {project, model} = create_project_model()
    [initial] = versions(model)
    checkpoint = create_version(model, 2)
    working = model_payload(99)
    request(:patch, "/api/v1/models/#{model["model_id"]}", %{"payload" => working}, 200)

    request(:delete, "/api/v1/model-versions/#{checkpoint["version_id"]}", nil, 200)
    current = fetched_model(model)
    assert current["latest_version_id"] == initial["version_id"]
    assert current["latest_version_number"] == initial["version_number"]
    assert current["payload"] == working
    assert versions(model) == [initial]
    exported = bundle(project)
    assert exported["active_version_id"] == initial["version_id"]
    assert hd(exported["models"])["payload"] == working

    assert request(:get, "/api/v1/model-versions/#{checkpoint["version_id"]}", nil, 404) ==
             %{"error" => "version_not_found"}

    request(:delete, "/api/v1/model-versions/#{checkpoint["version_id"]}", nil, 404)
    assert fetched_model(model) == current
    next = create_version(model, 3)
    assert next["version_number"] == 2
    assert next["version_id"] != checkpoint["version_id"]
    assert bundle(project)["active_version_id"] == next["version_id"]
  end

  test "deleting all checkpoints leaves a usable unversioned model and allows saving again" do
    {project, model} = create_project_model()
    request(:delete, "/api/v1/model-versions/#{model["latest_version_id"]}", nil, 200)
    current = fetched_model(model)
    assert current["latest_version_id"] == nil
    assert current["latest_version_number"] == 0
    assert current["payload"] == model["payload"]
    assert versions(model) == []
    assert bundle(project)["active_model_id"] == model["model_id"]
    assert bundle(project)["active_version_id"] == nil

    saved = create_version(model, 2)
    assert saved["version_number"] == 1
    assert saved["version_id"] != model["latest_version_id"]
    assert fetched_model(model)["latest_version_id"] == saved["version_id"]
  end

  test "deleting an older checkpoint keeps the current checkpoint and subsequent numbering" do
    {_project, model} = create_project_model()
    older = create_version(model, 2)
    latest = create_version(model, 3)
    request(:delete, "/api/v1/model-versions/#{older["version_id"]}", nil, 200)
    assert fetched_model(model)["latest_version_id"] == latest["version_id"]
    assert fetched_model(model)["latest_version_number"] == 3
    assert create_version(model, 4)["version_number"] == 4
    assert Enum.map(versions(model), & &1["version_number"]) == [4, 3, 1]
  end

  test "project bundle never selects another model's checkpoint for an unversioned active model" do
    {project, _model} = create_project_model()
    create_model(project, "Second conductor", 2)
    exported = bundle(project)
    active = Enum.find(exported["models"], &(&1["model_id"] == exported["active_model_id"]))
    request(:delete, "/api/v1/model-versions/#{active["latest_version_id"]}", nil, 200)

    exported = bundle(project)
    assert exported["active_model_id"] == active["model_id"]
    assert length(exported["model_versions"]) == 1
    assert exported["active_version_id"] == nil
    current = Enum.find(exported["models"], &(&1["model_id"] == active["model_id"]))
    assert current["latest_version_id"] == nil
  end

  test "parallel checkpoint requests publish distinct ordered versions with a consistent latest pointer" do
    {project, model} = create_project_model()

    created =
      2..13
      |> Task.async_stream(&create_version(model, &1), max_concurrency: 6, timeout: 15_000)
      |> Enum.map(fn {:ok, version} -> version end)

    assert Enum.sort(Enum.map(created, & &1["version_number"])) == Enum.to_list(2..13)
    [latest | _] = versions(model)
    current = fetched_model(model)
    assert current["latest_version_id"] == latest["version_id"]
    assert current["latest_version_number"] == 13
    assert current["payload"] == latest["payload"]
    assert length(bundle(project)["model_versions"]) == 13
  end

  test "project export can rebuild model checkpoints without including a different project's results" do
    {project, model} = create_project_model()
    latest = create_version(model, 2)
    {other_project, other_model} = create_project_model("Unrelated study")
    seed_result(project, latest, "owned-job")
    seed_result(other_project, hd(versions(other_model)), "other-job")

    exported = bundle(project)
    assert exported["project_schema_version"] == "kyuubiki.project/v2"
    assert Enum.map(exported["jobs"], & &1["job_id"]) == ["owned-job"]
    assert Enum.map(exported["results"], & &1["job_id"]) == ["owned-job"]
    assert exported["active_model_id"] == model["model_id"]
    assert exported["active_version_id"] == latest["version_id"]

    imported_project =
      request(:post, "/api/v1/projects", %{"name" => "Imported study"}, 201)["project"]

    ordered = Enum.sort_by(exported["model_versions"], & &1["version_number"])

    imported_model =
      request(
        :post,
        "/api/v1/projects/#{imported_project["project_id"]}/models",
        Map.take(hd(ordered), ["kind", "material", "model_schema_version", "payload"])
        |> Map.put("name", hd(exported["models"])["name"]),
        201
      )["model"]

    for version <- tl(ordered) do
      request(:post, "/api/v1/models/#{imported_model["model_id"]}/versions", version, 201)
    end

    round_trip = bundle(imported_project)
    assert fetched_model(imported_model)["payload"] == fetched_model(model)["payload"]

    assert Enum.map(round_trip["model_versions"], & &1["payload"]) ==
             Enum.map(exported["model_versions"], & &1["payload"])

    assert round_trip["active_model_id"] != exported["active_model_id"]
    assert round_trip["active_version_id"] != exported["active_version_id"]
    assert round_trip["jobs"] == []
    assert round_trip["results"] == []

    request(:delete, "/api/v1/projects/#{project["project_id"]}", nil, 200)
    request(:get, "/api/v1/projects/#{project["project_id"]}/bundle", nil, 404)
    request(:get, "/api/v1/models/#{model["model_id"]}", nil, 404)
    request(:get, "/api/v1/model-versions/#{latest["version_id"]}", nil, 404)
    assert bundle(other_project)["results"] |> hd() |> Map.get("job_id") == "other-job"
    assert length(bundle(imported_project)["model_versions"]) == 2
  end

  defp create_project_model(name \\ "Version chain study") do
    project = request(:post, "/api/v1/projects", %{"name" => name}, 201)["project"]
    {project, create_model(project, "Copper conductor", 1)}
  end

  defp create_model(project, name, revision) do
    request(
      :post,
      "/api/v1/projects/#{project["project_id"]}/models",
      %{
        "name" => name,
        "kind" => "truss_2d",
        "material" => "Copper",
        "payload" => model_payload(revision)
      },
      201
    )["model"]
  end

  defp create_version(model, revision) do
    request(
      :post,
      "/api/v1/models/#{model["model_id"]}/versions",
      %{
        "name" => "Checkpoint #{revision}",
        "payload" => model_payload(revision)
      },
      201
    )["version"]
  end

  defp model_payload(revision) do
    %{
      "kind" => "truss_2d",
      "model_schema_version" => "kyuubiki.model/v1",
      "revision" => revision,
      "nodes" => [],
      "elements" => []
    }
  end

  defp versions(model),
    do: request(:get, "/api/v1/models/#{model["model_id"]}/versions", nil, 200)["versions"]

  defp fetched_model(model),
    do: request(:get, "/api/v1/models/#{model["model_id"]}", nil, 200)["model"]

  defp bundle(project),
    do: request(:get, "/api/v1/projects/#{project["project_id"]}/bundle", nil, 200)

  defp seed_result(project, version, job_id) do
    {:ok, _} =
      Store.create(%{
        job_id: job_id,
        project_id: project["project_id"],
        model_version_id: version["version_id"],
        simulation_case_id: "export-scope"
      })

    :ok = AnalysisResultStore.put(job_id, %{"kind" => "truss_2d", "nodes" => []})
  end

  defp request(method, path, payload, status) do
    request =
      if is_nil(payload),
        do: conn(method, path),
        else:
          conn(method, path, Jason.encode!(payload))
          |> put_req_header("content-type", "application/json")

    response = Router.call(request, @opts)

    assert response.status == status,
           "#{method} #{path}: #{response.status} #{response.resp_body}"

    Jason.decode!(response.resp_body)
  end
end
