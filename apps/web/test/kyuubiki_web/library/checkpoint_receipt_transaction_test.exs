defmodule KyuubikiWeb.Library.CheckpointReceiptTransactionTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.Storage
  @moduletag skip: not Storage.sqlite?()

  test "receipt insertion failure rolls back the model/version and a clean retry commits once" do
    repo = Storage.repo_module!()
    {:ok, project} = Library.create_project(%{"name" => "Receipt transaction"})

    input = %{
      "request_id" => "atomic-receipt-0001",
      "name" => "Truss",
      "kind" => "truss_3d",
      "payload" => %{"h" => 1}
    }

    {:ok, model} = Library.create_model(project["project_id"], Map.delete(input, "request_id"))
    before = Library.get_model(model["model_id"])

    SQL.query!(
      repo,
      "CREATE TRIGGER reject_receipt BEFORE INSERT ON kyuubiki_checkpoint_requests BEGIN SELECT RAISE(ABORT, 'receipt_failure'); END",
      []
    )

    on_exit(fn -> SQL.query!(repo, "DROP TRIGGER IF EXISTS reject_receipt", []) end)

    for write <- [
          fn -> Library.create_model(project["project_id"], input) end,
          fn -> Library.create_version(model["model_id"], input) end
        ] do
      assert_raise Exqlite.Error, ~r/receipt_failure/, write
      assert Library.get_model(model["model_id"]) == before
      assert {:ok, [_]} = Library.list_models(project["project_id"])

      assert [[0]] ==
               SQL.query!(repo, "SELECT count(*) FROM kyuubiki_checkpoint_requests", []).rows
    end

    SQL.query!(repo, "DROP TRIGGER reject_receipt", [])
    {:ok, version} = Library.create_version(model["model_id"], input)
    assert Library.create_version(model["model_id"], input) == {:ok, version}
    assert {:ok, [_, _]} = Library.list_versions(model["model_id"])

    [[metadata]] =
      SQL.query!(repo, "SELECT response_meta FROM kyuubiki_checkpoint_requests", []).rows

    refute Map.has_key?(Jason.decode!(metadata), "payload")
  end
end
