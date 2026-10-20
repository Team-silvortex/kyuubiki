defmodule KyuubikiWeb.Storage.SqliteQueryCacheTest do
  use ExUnit.Case, async: true
  import Ecto.Query
  alias KyuubikiWeb.SqliteRepo, as: Repo
  alias KyuubikiWeb.Storage.{DatabaseMigrations, ProjectRecord}
  alias KyuubikiWeb.TestSupport.DatabaseFixture, as: Fixture

  setup do
    path = Path.join(Fixture.directory(), "query-cache.sqlite3")
    {:ok, db} = Exqlite.Sqlite3.open(path)

    try do
      :ok = Exqlite.Sqlite3.execute(db, "PRAGMA journal_mode=WAL")
    after
      Exqlite.Sqlite3.close(db)
    end

    repo =
      start_supervised!(
        {Repo, name: nil, database: path, pool_size: 2, journal_mode: :wal, busy_timeout: 5_000}
      )

    Repo.put_dynamic_repo(repo)
    DatabaseMigrations.boot!(Repo, :sqlite)
    %{repo: repo}
  end

  test "cached queries, bulk writes and streaming preserve their results" do
    project = Repo.insert!(%ProjectRecord{project_id: "cache-policy", name: "Original"})
    query = from(p in ProjectRecord, where: p.project_id == ^project.project_id)

    for _ <- 1..3 do
      assert Repo.get!(ProjectRecord, project.project_id, log: false) == project
      assert Repo.all(query) == [project]
      assert Repo.aggregate(query, :count) == 1
      assert Repo.exists?(query)
    end

    assert {1, nil} = Repo.update_all(query, set: [name: "Updated"])
    assert Repo.get!(ProjectRecord, project.project_id).name == "Updated"

    assert {:ok, [%ProjectRecord{name: "Updated"}]} =
             Repo.transaction(fn -> Repo.stream(query, max_rows: 1) |> Enum.to_list() end)

    assert {1, nil} = Repo.delete_all(query)
    refute Repo.exists?(query)
  end

  test "two checked-out connections reuse the query shape without mixing parameters", ctx do
    projects =
      for index <- 1..2 do
        Repo.insert!(%ProjectRecord{project_id: "connection-#{index}", name: "Project #{index}"})
      end

    parent = self()

    tasks =
      for project <- projects do
        Task.async(fn ->
          Repo.put_dynamic_repo(ctx.repo)

          Repo.checkout(fn ->
            assert Repo.get!(ProjectRecord, project.project_id) == project
            send(parent, {:checked_out, self()})

            receive do
              :query_again -> :ok
            after
              5_000 -> flunk("second connection was not checked out")
            end

            for _ <- 1..8 do
              assert Repo.get!(ProjectRecord, project.project_id) == project
              assert :erlang.garbage_collect()
            end
          end)
        end)
      end

    # Neither checkout is returned until both have executed the same query shape.
    for task <- tasks do
      pid = task.pid
      assert_receive {:checked_out, ^pid}, 5_000
    end

    Enum.each(tasks, &send(&1.pid, :query_again))
    Enum.each(tasks, &Task.await(&1, 10_000))
  end

  @tag timeout: 30_000
  test "native handle destruction cannot stall the writer's scheduler and expire its competitor" do
    run_contention_child(:cached_handle)
  end

  @tag timeout: 30_000
  test "query cleanup after returning a checkout cannot block a later borrower" do
    run_contention_child(:returned_checkout)
  end

  defp run_contention_child(scenario) do
    directory = Fixture.directory()
    paths = :code.get_path() |> Enum.flat_map(&["-pa", &1 |> List.to_string() |> Path.expand()])

    args =
      ["--erl", "+S 1:1 +SDcpu 1 +SDio 2"] ++
        paths ++
        [
          "-r",
          "test/support/sqlite_contention_child.exs",
          "-e",
          "KyuubikiWeb.TestSupport.SqliteContentionChild.run(#{inspect(directory)}, #{inspect(scenario)})"
        ]

    port =
      Port.open({:spawn_executable, System.find_executable("elixir")}, [
        :binary,
        :exit_status,
        :stderr_to_stdout,
        {:args, args},
        {:env, KyuubikiWeb.TestSupport.BeamChild.port_env()},
        {:cd, File.cwd!()}
      ])

    try do
      {output, status} = await_exit(port, "", System.monotonic_time(:millisecond) + 20_000)
      assert status == 0, output

      assert Regex.scan(~r/SQLITE_CONTENTION_ROUND (\d+)/, output, capture: :all_but_first) ==
               Enum.map(1..5, &[Integer.to_string(&1)])
    after
      if Port.info(port), do: Port.close(port)
    end

    for round <- 1..5 do
      connection = Fixture.open!(Path.join(directory, "contention-#{round}.sqlite3"))

      try do
        assert [["writer", "waiter"]] ==
                 KyuubikiWeb.Storage.MigrationQuery.rows!(
                   connection,
                   "SELECT name, description FROM kyuubiki_projects"
                 )
      after
        Fixture.close!(connection)
      end
    end
  end

  defp await_exit(port, output, deadline) do
    remaining = max(deadline - System.monotonic_time(:millisecond), 0)

    receive do
      {^port, {:data, bytes}} -> await_exit(port, output <> bytes, deadline)
      {^port, {:exit_status, status}} -> {output, status}
    after
      remaining -> flunk("SQLite contention child timed out: #{output}")
    end
  end
end
