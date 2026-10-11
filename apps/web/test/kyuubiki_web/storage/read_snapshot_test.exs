defmodule KyuubikiWeb.Storage.ReadSnapshotTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.ReadSnapshot

  test "successful and domain-error replies retain their original shape" do
    assert %{"projects" => []} = ReadSnapshot.run(fn -> %{"projects" => []} end)
    assert {:ok, []} = ReadSnapshot.run(fn -> {:ok, []} end)
    assert {:error, :project_not_found} = ReadSnapshot.run(fn -> {:error, :project_not_found} end)
  end

  test "unexpected programming exceptions are not masked and do not retain a connection" do
    assert_raise RuntimeError, "snapshot programming failure", fn ->
      ReadSnapshot.run(fn ->
        assert {:ok, []} = Library.list_projects()
        raise "snapshot programming failure"
      end)
    end

    assert {:ok, []} = ReadSnapshot.run(&Library.list_projects/0)
  end

  @tag skip: not Storage.sql?()
  test "a query failure discards partial data and releases the transaction" do
    assert {:error, :analysis_store_unavailable} =
             ReadSnapshot.run(fn ->
               assert {:ok, []} = Library.list_projects()
               SQL.query!(Storage.repo_module!(), "SELECT * FROM snapshot_missing_table", [])
               %{"partial" => true}
             end)

    assert {:ok, _} = Library.create_project(%{"name" => "Write after rollback"})
    assert {:ok, [_]} = ReadSnapshot.run(&Library.list_projects/0)
  end

  @tag skip: not Storage.sql?()
  test "an existing transaction cannot silently downgrade snapshot isolation" do
    repo = Storage.repo_module!()

    assert {:ok, :checked} =
             apply(repo, :transaction, [
               fn ->
                 assert {:ok, []} = Library.list_projects()

                 assert {:error, :analysis_store_unavailable} =
                          ReadSnapshot.run(fn -> flunk("must not run in ambient transaction") end)

                 :checked
               end
             ])

    assert {:ok, []} = ReadSnapshot.run(&Library.list_projects/0)
  end

  @tag skip: not Storage.sql?()
  test "the snapshot uses the selected dynamic repository and restores normal reads" do
    repo = Storage.repo_module!()
    selected = start_supervised!({repo, name: nil, pool_size: 1}, id: :snapshot_selected_repo)
    previous = apply(repo, :put_dynamic_repo, [selected])

    try do
      assert :checked =
               ReadSnapshot.run(fn ->
                 assert apply(repo, :get_dynamic_repo, []) == selected
                 assert apply(repo, :in_transaction?, [])
                 assert {:ok, []} = Library.list_projects()
                 assert_isolation(selected)
                 :checked
               end)

      refute apply(repo, :in_transaction?, [])
      assert {:ok, _} = Library.create_project(%{"name" => "Normal write outside snapshot"})
    after
      apply(repo, :put_dynamic_repo, [previous])
    end
  end

  @tag skip: not Storage.postgres?()
  test "PostgreSQL rejects accidental writes within the read-only snapshot" do
    assert {:error, :analysis_store_unavailable} =
             ReadSnapshot.run(fn -> Library.create_project(%{"name" => "Must not publish"}) end)

    assert {:ok, []} = Library.list_projects()
    assert {:ok, _} = Library.create_project(%{"name" => "Writable again"})
  end

  defp assert_isolation(repo) do
    if Storage.postgres?() do
      assert SQL.query!(repo, "SHOW transaction_isolation", []).rows == [["repeatable read"]]
      assert SQL.query!(repo, "SHOW transaction_read_only", []).rows == [["on"]]
    else
      assert SQL.query!(repo, "PRAGMA journal_mode", []).rows == [["wal"]]
    end
  end
end
