defmodule KyuubikiWeb.Api.LibraryRecordRecoveryApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Outage.setup()
    {:ok, project} = Library.create_project(%{"name" => "Library recovery"})
    {:ok, other} = Library.create_project(%{"name" => "Other project"})

    {:ok, model} =
      Library.create_model(project["project_id"], %{
        "name" => "Earlier",
        "kind" => "truss_2d",
        "payload" => %{"revision" => 1}
      })

    {:ok, version} =
      Library.create_version(model["model_id"], %{
        "name" => "Latest",
        "payload" => %{"revision" => 2}
      })

    {:ok, other_model} =
      Library.create_model(other["project_id"], %{
        "name" => "Other model",
        "kind" => "truss_2d",
        "payload" => %{}
      })

    %{
      project: project,
      model: model,
      version: version,
      other: other,
      other_model: other_model,
      execution: Fixture.terminal_job()
    }
  end

  test "project creation supplies a usable receipt without creating execution records", fixture do
    assert {201, %{"project" => project}} =
             request(:post, "/api/v1/projects", %{"name" => "Created"})

    assert is_binary(project["project_id"]) and project["project_id"] != ""
    assert project["name"] == "Created"
    usable_receipt(:project, project)
    assert {:ok, stored} = Library.get_project(project["project_id"])
    assert stored["project_id"] == project["project_id"]
    Fixture.unchanged(fixture.execution)
  end

  for kind <- [:project, :model, :version] do
    test "#{kind} metadata editing returns a matching receipt and leaves other work unchanged",
         fixture do
      kind = unquote(kind)
      id = fixture[kind]["#{kind}_id"]
      key = Atom.to_string(kind)
      assert {200, %{^key => record}} = request(:patch, path(kind, id), %{"name" => "Reviewed"})
      assert record["#{kind}_id"] == id
      assert record["name"] == "Reviewed"
      usable_receipt(kind, record)
      assert {200, %{^key => stored}} = request(:get, path(kind, id))
      assert stored["name"] == "Reviewed"
      unchanged_other(fixture)
    end

    test "#{kind} deletion returns the removed identity and cannot be repeated or recreated by editing",
         fixture do
      kind = unquote(kind)
      id = fixture[kind]["#{kind}_id"]
      key = Atom.to_string(kind)
      assert {200, %{^key => record}} = request(:delete, path(kind, id))
      assert record["#{kind}_id"] == id
      assert is_map(record)
      usable_receipt(kind, record)
      assert {404, _} = request(:get, path(kind, id))
      assert {404, _} = request(:delete, path(kind, id))
      assert {404, _} = request(:patch, path(kind, id), %{"name" => "Do not resurrect"})

      retained_parent(kind, fixture)
      unchanged_other(fixture)
    end

    test "confirmed #{kind} deletion is separate from a subsequent unavailable catalog read",
         fixture do
      kind = unquote(kind)
      id = fixture[kind]["#{kind}_id"]
      assert {200, _} = request(:delete, path(kind, id))
      assert {404, _} = request(:get, path(kind, id))
      unchanged_other(fixture)

      Outage.without(Storage.repo_module() || KyuubikiWeb.Library.MemoryBackend, fn ->
        assert {503, %{"error" => "analysis_store_unavailable"}} =
                 request(:get, "/api/v1/projects")
      end)

      # JSON's lightweight library restart is volatile; this is not durable recovery qualification.
      Fixture.unchanged(fixture.execution)
    end
  end

  defp unchanged_other(fixture) do
    assert {:ok, model} = Library.get_model(fixture.other_model["model_id"])
    assert model["name"] == fixture.other_model["name"]
    assert model["payload"] == fixture.other_model["payload"]
    assert {:ok, project} = Library.get_project(fixture.other["project_id"])
    assert project["name"] == fixture.other["name"]
    Fixture.unchanged(fixture.execution)
  end

  defp usable_receipt(kind, record) do
    assert is_binary(record["#{kind}_id"]) and String.trim(record["#{kind}_id"]) != ""
    assert is_binary(record["name"]) and String.trim(record["name"]) != ""

    if kind != :project do
      assert is_binary(record["project_id"]) and String.trim(record["project_id"]) != ""
      assert is_map(record["payload"])
    end

    if kind == :version do
      assert is_binary(record["model_id"]) and String.trim(record["model_id"]) != ""
      assert is_integer(record["version_number"]) and record["version_number"] >= 1
    end
  end

  defp retained_parent(:project, fixture) do
    assert {404, _} = request(:get, path(:version, fixture.version["version_id"]))
  end

  defp retained_parent(:model, fixture) do
    assert {200, _} = request(:get, path(:project, fixture.project["project_id"]))
    assert {404, _} = request(:get, path(:version, fixture.version["version_id"]))
  end

  defp retained_parent(:version, fixture) do
    assert {200, _} = request(:get, path(:project, fixture.project["project_id"]))
    assert {200, %{"model" => retained}} = request(:get, path(:model, fixture.model["model_id"]))
    assert retained["latest_version_id"] == fixture.model["latest_version_id"]
  end

  defp path(:project, id), do: "/api/v1/projects/#{id}"
  defp path(:model, id), do: "/api/v1/models/#{id}"
  defp path(:version, id), do: "/api/v1/model-versions/#{id}"

  defp request(method, path, attrs \\ nil) do
    connection =
      if is_nil(attrs),
        do: conn(method, path),
        else:
          conn(method, path, Jason.encode!(attrs))
          |> put_req_header("content-type", "application/json")

    response = Router.call(connection, @opts)
    {response.status, Jason.decode!(response.resp_body)}
  end
end
