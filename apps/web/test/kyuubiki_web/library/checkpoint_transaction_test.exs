defmodule KyuubikiWeb.Library.CheckpointTransactionTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.Storage

  @moduletag skip: not Storage.sql?()

  test "failed initial checkpoint publication rolls back the model and permits a clean retry" do
    {:ok, project} = Library.create_project(%{"name" => "Atomic creation"})
    install_publication_failure()

    assert_publication_failure(fn ->
      Library.create_model(project["project_id"], model_attrs())
    end)

    assert Library.list_models(project["project_id"]) == {:ok, []}
    assert SQL.query!(repo(), "SELECT COUNT(*) FROM kyuubiki_model_versions", []).rows == [[0]]

    remove_publication_failure()
    {:ok, model} = Library.create_model(project["project_id"], model_attrs())
    assert model["name"] == "Conductor"
    {:ok, [version]} = Library.list_versions(model["model_id"])
    assert model["latest_version_id"] == version["version_id"]
  end

  test "checkpoint create and delete roll back together with their latest pointer on failure" do
    {:ok, project} = Library.create_project(%{"name" => "Atomic checkpoints"})
    {:ok, model} = Library.create_model(project["project_id"], model_attrs())
    {:ok, version} = Library.create_version(model["model_id"], %{"payload" => %{"revision" => 2}})
    before_model = Library.get_model(model["model_id"])
    before_versions = Library.list_versions(model["model_id"])
    install_publication_failure()

    assert_publication_failure(fn ->
      Library.create_version(model["model_id"], %{
        "name" => "Uncommitted rename",
        "kind" => "truss_3d",
        "material" => "Aluminum",
        "payload" => %{"revision" => 3}
      })
    end)

    assert Library.get_model(model["model_id"]) == before_model
    assert Library.list_versions(model["model_id"]) == before_versions
    assert_publication_failure(fn -> Library.delete_version(version["version_id"]) end)
    assert Library.get_model(model["model_id"]) == before_model
    assert Library.list_versions(model["model_id"]) == before_versions

    remove_publication_failure()
    assert {:ok, _} = Library.delete_version(version["version_id"])

    {:ok, next} =
      Library.create_version(model["model_id"], %{
        "name" => "Committed rename",
        "kind" => "truss_3d",
        "material" => "Steel",
        "payload" => %{"revision" => 4}
      })

    assert next["version_number"] == 2
    {:ok, current} = Library.get_model(model["model_id"])
    assert current["latest_version_id"] == next["version_id"]
    assert current["payload"] == %{"revision" => 4}
    assert current["name"] == next["name"]
    assert current["kind"] == next["kind"]
    assert current["material"] == next["material"]
  end

  defp install_publication_failure do
    on_exit(&remove_publication_failure/0)

    if Storage.sqlite?() do
      SQL.query!(
        repo(),
        """
        CREATE TRIGGER checkpoint_test_reject_update
        BEFORE UPDATE OF latest_version_id ON kyuubiki_models
        BEGIN
          SELECT RAISE(ABORT, 'injected_checkpoint_failure');
        END
        """,
        []
      )
    else
      SQL.query!(
        repo(),
        """
        CREATE FUNCTION checkpoint_test_reject_update() RETURNS trigger AS $$
        BEGIN
          RAISE EXCEPTION 'injected_checkpoint_failure';
        END;
        $$ LANGUAGE plpgsql
        """,
        []
      )

      SQL.query!(
        repo(),
        """
        CREATE TRIGGER checkpoint_test_reject_update
        BEFORE UPDATE OF latest_version_id ON kyuubiki_models
        FOR EACH ROW EXECUTE FUNCTION checkpoint_test_reject_update()
        """,
        []
      )
    end
  end

  defp remove_publication_failure do
    if Storage.sqlite?() do
      SQL.query!(repo(), "DROP TRIGGER IF EXISTS checkpoint_test_reject_update", [])
    else
      SQL.query!(
        repo(),
        "DROP TRIGGER IF EXISTS checkpoint_test_reject_update ON kyuubiki_models",
        []
      )

      SQL.query!(repo(), "DROP FUNCTION IF EXISTS checkpoint_test_reject_update()", [])
    end
  end

  defp assert_publication_failure(callback) do
    exception = if Storage.sqlite?(), do: Exqlite.Error, else: Postgrex.Error
    assert_raise exception, ~r/injected_checkpoint_failure/, callback
  end

  defp model_attrs do
    %{"name" => "Conductor", "kind" => "truss_2d", "payload" => %{"revision" => 1}}
  end

  defp repo, do: Storage.repo_module!()
end
