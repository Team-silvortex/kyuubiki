defmodule KyuubikiWeb.Api.ProjectExecutionChainLiveTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  import KyuubikiWeb.TestSupport.ProjectExecutionApi

  @moduletag :native_agent
  @moduletag skip: is_nil(System.get_env("KYUUBIKI_PROJECT_CHAIN_AGENT_PORT"))

  setup do
    port = System.fetch_env!("KYUUBIKI_PROJECT_CHAIN_AGENT_PORT") |> String.to_integer()
    assert port in 1..65_535

    # An explicitly supplied, isolated Agent is required; never borrow installed runtime agents.
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "project-chain-agent", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()
    :ok
  end

  test "saved input executes directly and through a catalog, exports, reconstructs, and reruns" do
    {project, model} = create_study()
    version_id = model["latest_version_id"]
    saved = request(:get, "/api/v1/model-versions/#{version_id}", nil, 200)["version"]
    working = rod(6_000)
    request(:patch, "/api/v1/models/#{model["model_id"]}", %{"payload" => working}, 200)

    # Version IDs record provenance. Clients explicitly submit the selected checkpoint payload.
    direct = submit_direct(saved["payload"], context(project, model)) |> completed()
    catalog = submit_catalog(saved["payload"], context(project, model)) |> completed()
    assert_lineage(direct["job"], project, version_id)
    assert_lineage(catalog["job"], project, version_id)
    assert_solution(direct["result"], 2_000)
    assert_solution(summary(catalog), 2_000)

    assert Enum.sort(catalog["result"]["completed_nodes"]) ==
             Enum.sort([
               "truss_2d_model",
               "solve_main",
               "extract_summary",
               "export_json",
               "json_output"
             ])

    {unrelated, unrelated_model} = create_study("Unrelated research")
    other = submit_catalog(rod(8_000), context(unrelated, unrelated_model)) |> completed()
    exported = bundle(project) |> Jason.encode!() |> Jason.decode!()
    owned_ids = Enum.sort([direct["job"]["job_id"], catalog["job"]["job_id"]])
    assert Enum.sort(Enum.map(exported["jobs"], & &1["job_id"])) == owned_ids
    assert Enum.sort(Enum.map(exported["results"], & &1["job_id"])) == owned_ids
    assert hd(exported["models"])["payload"] == working
    assert hd(exported["model_versions"])["payload"] == saved["payload"]

    for original <- [direct, catalog] do
      entry = Enum.find(exported["results"], &(&1["job_id"] == original["job"]["job_id"]))
      archived = request(:get, "/api/v1/results/#{entry["job_id"]}", nil, 200)
      assert entry["result"] == archived["result"]
      assert archived["result"] == original["result"]
    end

    # This is API-driven reconstruction of model data, not an unimplemented archive import API.
    reopened =
      request(:post, "/api/v1/projects", %{"name" => "Reopened research"}, 201)["project"]

    reconstructed =
      request(
        :post,
        "/api/v1/projects/#{reopened["project_id"]}/models",
        Map.take(hd(exported["model_versions"]), [
          "name",
          "kind",
          "material",
          "model_schema_version",
          "payload"
        ]),
        201
      )["model"]

    refute reconstructed["model_id"] == model["model_id"]
    refute reconstructed["latest_version_id"] == version_id
    assert bundle(reopened)["results"] == []

    rerun =
      submit_catalog(reconstructed["payload"], context(reopened, reconstructed)) |> completed()

    assert_lineage(rerun["job"], reopened, reconstructed["latest_version_id"])
    assert summary(rerun) == summary(catalog)
    assert length(bundle(reopened)["results"]) == 1

    for id <- owned_ids do
      request(:delete, "/api/v1/jobs/#{id}", nil, 200)
      request(:get, "/api/v1/jobs/#{id}", nil, 404)
      request(:get, "/api/v1/results/#{id}", nil, 404)
    end

    assert bundle(project)["jobs"] == []
    assert bundle(project)["results"] == []
    request(:delete, "/api/v1/projects/#{project["project_id"]}", nil, 200)
    request(:get, "/api/v1/model-versions/#{version_id}", nil, 404)
    assert hd(bundle(unrelated)["results"])["job_id"] == other["job"]["job_id"]
    assert hd(bundle(reopened)["results"])["job_id"] == rerun["job"]["job_id"]
  end

  test "singular native solve keeps failure diagnostics, not a successful artifact, and permits a corrected retry" do
    {project, model} = create_study()

    singular =
      Map.update!(rod(), "nodes", &Enum.map(&1, fn node -> Map.put(node, "fix_x", false) end))

    invalid =
      request(
        :post,
        "/api/v1/models/#{model["model_id"]}/versions",
        %{"payload" => singular},
        201
      )["version"]

    invalid_context = %{
      "project_id" => project["project_id"],
      "model_version_id" => invalid["version_id"]
    }

    failed = submit_catalog(singular, invalid_context) |> terminal()
    assert failed["job"]["status"] == "failed", inspect(failed)
    assert failed["job"]["message"] not in [nil, ""]
    assert_lineage(failed["job"], project, invalid["version_id"])
    assert failed["job"]["has_result"]
    diagnostics = request(:get, "/api/v1/results/#{failed["job"]["job_id"]}", nil, 200)["result"]
    assert diagnostics["recovery"]["state"] == "failed"
    assert diagnostics["recovery"]["history"] != []
    refute "solve_main" in diagnostics["completed_nodes"]
    refute Map.has_key?(diagnostics["artifacts"], "json_output.json")
    assert hd(bundle(project)["results"])["result"]["recovery"]["state"] == "failed"

    valid =
      request(:post, "/api/v1/models/#{model["model_id"]}/versions", %{"payload" => rod()}, 201)[
        "version"
      ]

    valid_context = %{
      "project_id" => project["project_id"],
      "model_version_id" => valid["version_id"]
    }

    retry = submit_catalog(valid["payload"], valid_context) |> completed()
    assert_solution(summary(retry), 2_000)
    assert_lineage(retry["job"], project, valid["version_id"])
    assert length(bundle(project)["jobs"]) == 2
    results = bundle(project)["results"]
    assert length(results) == 2
    successful = Enum.filter(results, &(&1["result"]["recovery"]["state"] == "completed"))
    assert Enum.map(successful, & &1["job_id"]) == [retry["job"]["job_id"]]
  end

  test "direct and catalog results both match the analytical load sweep" do
    {project, model} = create_study()

    for load <- [0, 1_500, 6_000, 40_000] do
      version =
        request(
          :post,
          "/api/v1/models/#{model["model_id"]}/versions",
          %{"payload" => rod(load)},
          201
        )["version"]

      context = %{
        "project_id" => project["project_id"],
        "model_version_id" => version["version_id"]
      }

      direct = submit_direct(version["payload"], context) |> completed()
      catalog = submit_catalog(version["payload"], context) |> completed()
      assert_solution(direct["result"], load)
      assert_solution(summary(catalog), load)
      assert_lineage(direct["job"], project, version["version_id"])
      assert_lineage(catalog["job"], project, version["version_id"])
    end

    assert length(bundle(project)["jobs"]) == 8
    assert length(bundle(project)["results"]) == 8
  end

  defp submit_direct(model, context),
    do: request(:post, "/api/v1/fem/truss-2d/jobs", Map.merge(model, context), 202)

  defp submit_catalog(model, context),
    do: request(:post, catalog_path(), catalog_input(model, context), 202)

  defp assert_solution(result, load) do
    # Uniform axial rod: u = F L / (E A), sigma = F / A. SI units throughout.
    displacement = load * 2 / (200.0e9 * 0.002)
    stress = load / 0.002

    assert_in_delta result["max_displacement"],
                    displacement,
                    max(1.0e-14, abs(displacement) * 1.0e-10)

    assert_in_delta result["max_stress"], stress, max(1.0e-8, abs(stress) * 1.0e-10)
  end
end
