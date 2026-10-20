defmodule KyuubikiWeb.Library.ProjectRequestTransactionTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.DatabaseMigrations
  alias KyuubikiWeb.TestSupport.DatabaseFixture, as: Fixture
  alias KyuubikiWeb.SqliteRepo, as: Repo
  @moduletag skip: not Storage.sqlite?()

  test "receipt failure rolls back project creation, and retry after repair commits once" do
    repo = Storage.repo_module!()

    SQL.query!(
      repo,
      "CREATE TRIGGER reject_project_receipt BEFORE INSERT ON kyuubiki_project_requests BEGIN SELECT RAISE(ABORT, 'project_receipt_failure'); END",
      []
    )

    on_exit(fn -> SQL.query!(repo, "DROP TRIGGER IF EXISTS reject_project_receipt", []) end)

    assert_raise Exqlite.Error, ~r/project_receipt_failure/, fn ->
      Library.create_project(input())
    end

    assert Library.list_projects() == {:ok, []}
    assert SQL.query!(repo, "SELECT count(*) FROM kyuubiki_project_requests", []).rows == [[0]]
    SQL.query!(repo, "DROP TRIGGER reject_project_receipt", [])
    {:ok, project} = Library.create_project(input())
    assert Library.create_project(input()) == {:ok, project}
    assert {:ok, [_]} = Library.list_projects()
    [[json]] = SQL.query!(repo, "SELECT response_meta FROM kyuubiki_project_requests", []).rows
    assert Jason.decode!(json) == project
    refute Map.has_key?(project, "request_id")
    refute Map.has_key?(project, "payload")
  end

  for deleted <- [false, true] do
    test "reopening the database retains a #{if deleted, do: "deleted", else: "committed"} receipt" do
      path = Path.join(Fixture.directory(), "project-receipt.sqlite3")

      created =
        with_repo(path, fn ->
          DatabaseMigrations.boot!(Repo, :sqlite)
          {:ok, created} = Library.create_project(input())
          if unquote(deleted), do: Library.delete_project(created["project_id"])
          created
        end)

      with_repo(path, fn ->
        DatabaseMigrations.boot!(Repo, :sqlite)

        expected =
          if unquote(deleted), do: {:error, :project_result_deleted}, else: {:ok, created}

        assert Library.create_project(input()) == expected
        {:ok, projects} = Library.list_projects()
        assert length(projects) == if(unquote(deleted), do: 0, else: 1)
      end)
    end
  end

  test "independent SQLite connections serialize the same project request" do
    path = Path.join(Fixture.directory(), "concurrent.sqlite3")

    with_repo(path, fn ->
      DatabaseMigrations.boot!(Repo, :sqlite)
      repo = Repo.get_dynamic_repo()

      tasks =
        for _ <- 1..8,
            do:
              Task.async(fn ->
                Repo.put_dynamic_repo(repo)
                Library.create_project(input())
              end)

      assert [{:ok, _}] = tasks |> Enum.map(&Task.await(&1, 15_000)) |> Enum.uniq()
      assert {:ok, [_]} = Library.list_projects()
      assert SQL.query!(repo, "SELECT count(*) FROM kyuubiki_project_requests", []).rows == [[1]]
    end)
  end

  @tag :capture_log
  test "a busy writer leaves no partial project or receipt, and explicit retry commits once" do
    path = Path.join(Fixture.directory(), "busy-writer.sqlite3")

    with_repo(path, [pool_size: 1, busy_timeout: 0], fn ->
      DatabaseMigrations.boot!(Repo, :sqlite)
      repo = Repo.get_dynamic_repo()
      {:sqlite, lock} = connection = Fixture.open!(path)

      try do
        :ok = Exqlite.Sqlite3.execute(lock, "BEGIN IMMEDIATE")

        try do
          assert_raise Exqlite.Error, ~r/(locked|busy)/i, fn ->
            Library.create_project(input())
          end

          assert {:ok, []} = Library.list_projects()

          assert SQL.query!(repo, "SELECT count(*) FROM kyuubiki_project_requests", []).rows == [
                   [0]
                 ]
        after
          :ok = Exqlite.Sqlite3.execute(lock, "ROLLBACK")
        end
      after
        Fixture.close!(connection)
      end

      assert {:ok, created} = Library.create_project(input())
      assert Library.create_project(input()) == {:ok, created}
      assert {:ok, [listed]} = Library.list_projects()
      assert Map.delete(listed, "models") == created
      assert listed["models"] == []
      assert SQL.query!(repo, "SELECT count(*) FROM kyuubiki_project_requests", []).rows == [[1]]
    end)
  end

  defp with_repo(path, options \\ [], callback) do
    # Exercise concurrent runtime connections in WAL mode, not the offline copy-upgrade mode.
    # Set the persistent journal mode before competing pool connections configure the new file.
    {:ok, db} = Exqlite.Sqlite3.open(path)

    try do
      :ok = Exqlite.Sqlite3.execute(db, "PRAGMA journal_mode=WAL")
    after
      Exqlite.Sqlite3.close(db)
    end

    {:ok, pid} =
      Repo.start_link(
        Keyword.merge(
          [
            name: nil,
            database: path,
            pool_size: 2,
            journal_mode: :wal,
            busy_timeout: 5_000
          ],
          options
        )
      )

    previous = Repo.put_dynamic_repo(pid)

    try do
      callback.()
    after
      Repo.put_dynamic_repo(previous)
      Supervisor.stop(pid)
    end
  end

  defp input, do: %{"request_id" => "durable-project-request-0001", "name" => "Durable study"}
end
