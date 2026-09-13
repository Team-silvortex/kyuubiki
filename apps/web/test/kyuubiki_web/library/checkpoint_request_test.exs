defmodule KyuubikiWeb.Library.CheckpointRequestTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  setup do
    {:ok, project} = Library.create_project(%{"name" => "Retry qualification"})
    %{project: project}
  end

  test "create-model retry returns its original receipt without rewinding a newer checkpoint", %{
    project: project
  } do
    input = attrs("initial-request-0001")
    {:ok, created} = Library.create_model(project["project_id"], input)

    {:ok, newer} =
      Library.create_version(created["model_id"], %{"name" => "Newer", "payload" => %{"h" => 2}})

    assert Library.create_model(project["project_id"], input) == {:ok, created}
    {:ok, current} = Library.get_model(created["model_id"])
    assert current["latest_version_id"] == newer["version_id"]
    assert current["payload"] == %{"h" => 2}
    assert {:ok, [_]} = Library.list_models(project["project_id"])
    assert {:ok, [_, _]} = Library.list_versions(created["model_id"])
  end

  test "checkpoint replay preserves ids and conflict rejects changed content", %{project: project} do
    {:ok, model} = Library.create_model(project["project_id"], attrs())
    input = attrs("version-request-0001")
    {:ok, version} = Library.create_version(model["model_id"], input)
    assert Library.create_version(model["model_id"], input) == {:ok, version}
    before = Library.get_model(model["model_id"])

    assert Library.create_version(model["model_id"], Map.put(input, "payload", %{"h" => 9})) ==
             {:error, :checkpoint_request_conflict}

    assert Library.get_model(model["model_id"]) == before
    assert {:ok, [_, _]} = Library.list_versions(model["model_id"])
  end

  test "concurrent identical retries create exactly one model and one checkpoint", %{
    project: project
  } do
    results =
      for _ <- 1..8,
          do:
            Task.async(fn ->
              Library.create_model(project["project_id"], attrs("concurrent-request-0001"))
            end)

    models = Enum.map(results, &Task.await(&1, 15_000))
    assert length(Enum.uniq(models)) == 1
    {:ok, model} = hd(models)

    results =
      for _ <- 1..8,
          do:
            Task.async(fn ->
              Library.create_version(model["model_id"], attrs("concurrent-request-0002"))
            end)

    assert length(Enum.uniq(Enum.map(results, &Task.await(&1, 15_000)))) == 1
    assert {:ok, [_]} = Library.list_models(project["project_id"])
    assert {:ok, [_, _]} = Library.list_versions(model["model_id"])
  end

  test "request keys are scoped to their parent and operation", %{project: project} do
    input = attrs("shared-request-id-0001")
    {:ok, other} = Library.create_project(%{"name" => "Other project"})
    {:ok, a} = Library.create_model(project["project_id"], input)
    {:ok, b} = Library.create_model(other["project_id"], input)
    assert a["model_id"] != b["model_id"]
    {:ok, av} = Library.create_version(a["model_id"], input)
    {:ok, bv} = Library.create_version(b["model_id"], input)
    assert av["version_id"] != bv["version_id"]
  end

  test "deleted checkpoints and models cannot be resurrected by an old request", %{
    project: project
  } do
    input = attrs("deleted-request-0001")
    {:ok, model} = Library.create_model(project["project_id"], input)
    {:ok, version} = Library.create_version(model["model_id"], input)
    assert {:ok, _} = Library.delete_version(version["version_id"])

    assert Library.create_version(model["model_id"], input) ==
             {:error, :checkpoint_result_deleted}

    assert {:ok, _} = Library.delete_model(model["model_id"])

    assert Library.create_model(project["project_id"], input) ==
             {:error, :checkpoint_result_deleted}

    assert Library.list_models(project["project_id"]) == {:ok, []}
  end

  test "HTTP conflict is explicit and invalid keys cannot write records", %{project: project} do
    path = "/api/v1/projects/#{project["project_id"]}/models"
    input = attrs("http-request-key-0001")
    first = post_json(path, input)
    assert first.status == 201
    assert post_json(path, input).resp_body == first.resp_body
    conflict = post_json(path, Map.put(input, "name", "Different"))
    assert conflict.status == 409
    assert Jason.decode!(conflict.resp_body)["error"] == "checkpoint_request_conflict"

    for key <- ["", "short", "contains spaces 12345", String.duplicate("a", 129), %{}, 42] do
      assert post_json(path, Map.put(input, "request_id", key)).status == 422
    end

    assert {:ok, [_]} = Library.list_models(project["project_id"])
  end

  test "calls without request ids still create intentionally distinct checkpoints", %{
    project: project
  } do
    {:ok, model} = Library.create_model(project["project_id"], attrs())
    {:ok, a} = Library.create_version(model["model_id"], attrs())
    {:ok, b} = Library.create_version(model["model_id"], attrs())
    assert a["version_id"] != b["version_id"]
  end

  test "request fingerprints ignore object key ordering but preserve list ordering" do
    alias KyuubikiWeb.Library.CheckpointRequest

    a = %{
      "model_id" => "model",
      "request_id" => "ordered-request-0001",
      "payload" => %{"b" => 2, "a" => [1, 2]}
    }

    b = Map.put(a, "payload", Map.new([{"a", [1, 2]}, {"b", 2}]))
    assert CheckpointRequest.identify(:version, a) == CheckpointRequest.identify(:version, b)

    refute CheckpointRequest.identify(:version, a) ==
             CheckpointRequest.identify(
               :version,
               Map.put(b, "payload", %{"a" => [2, 1], "b" => 2})
             )
  end

  defp attrs(id \\ nil),
    do: %{
      "request_id" => id,
      "name" => "Checkpoint",
      "kind" => "truss_3d",
      "material" => "210",
      "payload" => %{"h" => 1.5, "nodes" => []}
    }

  defp post_json(path, input) do
    conn(:post, path, Jason.encode!(input))
    |> put_req_header("content-type", "application/json")
    |> Router.call(@opts)
  end
end
