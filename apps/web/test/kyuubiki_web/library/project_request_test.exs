defmodule KyuubikiWeb.Library.ProjectRequestTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "a lost create response can be retried without creating another project" do
    input = attrs()
    {:ok, created} = Library.create_project(input)
    assert Library.create_project(input) == {:ok, created}
    assert {:ok, [_]} = Library.list_projects()
    refute Map.has_key?(created, "request_id")
  end

  test "replay returns the creation receipt without reverting subsequent edits or children" do
    input = attrs()
    {:ok, created} = Library.create_project(input)
    {:ok, updated} = Library.update_project(created["project_id"], %{"name" => "Renamed"})

    {:ok, model} =
      Library.create_model(created["project_id"], %{
        "name" => "Truss",
        "kind" => "truss_3d",
        "payload" => %{"nodes" => []}
      })

    assert Library.create_project(input) == {:ok, created}
    {:ok, current} = Library.get_project(created["project_id"])
    assert current["name"] == updated["name"]
    assert [%{"model_id" => id}] = current["models"]
    assert id == model["model_id"]
  end

  test "changing either project field under an existing request conflicts without a write" do
    input = attrs()
    {:ok, _} = Library.create_project(input)
    before = Library.list_projects()

    for field <- ["name", "description"] do
      assert Library.create_project(Map.put(input, field, "Different")) ==
               {:error, :project_request_conflict}
    end

    assert Library.list_projects() == before
  end

  test "concurrent identical requests return one committed project" do
    results = for _ <- 1..8, do: Task.async(fn -> Library.create_project(attrs()) end)
    replies = Enum.map(results, &Task.await(&1, 15_000))
    assert [{:ok, _}] = Enum.uniq(replies)
    assert {:ok, [_]} = Library.list_projects()
  end

  test "concurrent conflicting requests commit only the winner" do
    results =
      for name <- ["A", "B"],
          do:
            Task.async(fn ->
              Library.create_project(Map.put(attrs(), "name", name))
            end)

    replies = Enum.map(results, &Task.await(&1, 15_000))
    assert Enum.count(replies, &match?({:ok, _}, &1)) == 1
    assert Enum.count(replies, &(&1 == {:error, :project_request_conflict})) == 1
    assert {:ok, [_]} = Library.list_projects()
  end

  test "deleted projects are never recreated by an old request" do
    input = attrs()
    {:ok, project} = Library.create_project(input)
    assert {:ok, _} = Library.delete_project(project["project_id"])
    assert Library.create_project(input) == {:error, :project_result_deleted}
    assert Library.list_projects() == {:ok, []}
  end

  test "normalization is stable and calls without request ids remain intentionally distinct" do
    {:ok, original} = Library.create_project(attrs())

    assert Library.create_project(Map.put(attrs(), "name", "  Recovery study  ")) ==
             {:ok, original}

    for input <- [
          Map.delete(attrs(), "request_id"),
          Map.put(attrs(), "request_id", "other-project-request-0001")
        ] do
      assert {:ok, created} = Library.create_project(input)
      refute created["project_id"] == original["project_id"]
    end

    assert {:ok, [_, _, _]} = Library.list_projects()
  end

  test "invalid request ids cannot create any records" do
    for key <- ["", "short", "contains spaces 12345", String.duplicate("a", 129), %{}, 42] do
      assert post_json(Map.put(attrs(), "request_id", key)).status == 422
    end

    assert Library.list_projects() == {:ok, []}
  end

  test "creation and replay both require write authentication" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "project-request-test-secret",
      protect_reads?: true
    )

    assert post_json(attrs()).status == 401
    assert Library.list_projects() == {:ok, []}

    authorized = fn ->
      conn(:post, "/api/v1/projects", Jason.encode!(attrs()))
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "project-request-test-secret")
      |> Router.call(@opts)
    end

    first = authorized.()
    assert first.status == 201
    assert post_json(attrs()).status == 401
    assert authorized.().resp_body == first.resp_body
    assert {:ok, [_]} = Library.list_projects()
  end

  test "HTTP retries preserve the envelope and return explicit conflicts and tombstones" do
    input = attrs()
    first = post_json(input)
    assert first.status == 201
    assert post_json(input).resp_body == first.resp_body
    conflict = post_json(Map.put(input, "description", "Changed"))
    assert conflict.status == 409
    assert Jason.decode!(conflict.resp_body)["error"] == "project_request_conflict"
    project = Jason.decode!(first.resp_body)["project"]
    Library.delete_project(project["project_id"])
    deleted = post_json(input)
    assert deleted.status == 409
    assert Jason.decode!(deleted.resp_body)["error"] == "project_result_deleted"
  end

  defp attrs,
    do: %{
      "request_id" => "project-create-request-0001",
      "name" => "Recovery study",
      "description" => "Imported project"
    }

  defp post_json(input) do
    conn(:post, "/api/v1/projects", Jason.encode!(input))
    |> put_req_header("content-type", "application/json")
    |> Router.call(@opts)
  end
end
