defmodule KyuubikiWeb.TestSupport.SqliteContentionChild do
  @moduledoc false

  import ExUnit.Assertions
  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.SqliteRepo, as: Repo
  alias KyuubikiWeb.Storage.{DatabaseMigrations, ProjectRecord}

  def run(directory, scenario) do
    spawn(fn ->
      Process.sleep(25_000)
      System.halt(99)
    end)

    {:ok, _} = Application.ensure_all_started(:ecto_sqlite3)
    Logger.configure(level: :warning)
    assert :erlang.system_info(:schedulers_online) == 1

    for round <- 1..5 do
      path = Path.join(directory, "contention-#{round}.sqlite3")
      {:ok, db} = Exqlite.Sqlite3.open(path)

      try do
        :ok = Exqlite.Sqlite3.execute(db, "PRAGMA journal_mode=WAL")
      after
        Exqlite.Sqlite3.close(db)
      end

      {:ok, repo} =
        Repo.start_link(
          name: nil,
          database: path,
          pool_size: 2,
          journal_mode: :wal,
          busy_timeout: 5_000
        )

      previous = Repo.put_dynamic_repo(repo)

      try do
        DatabaseMigrations.boot!(Repo, :sqlite)
        Repo.insert!(%ProjectRecord{project_id: "lock-order", name: "original"})

        case scenario do
          :cached_handle -> interleave(repo)
          :returned_checkout -> returned_checkout(repo)
        end

        assert %ProjectRecord{name: "writer", description: "waiter"} = read_project()
        IO.puts("SQLITE_CONTENTION_ROUND #{round}")
      after
        Repo.put_dynamic_repo(previous)
        Supervisor.stop(repo)
      end
    end
  end

  defp interleave(repo) do
    parent = self()

    waiter =
      Task.async(fn ->
        Repo.put_dynamic_repo(repo)

        Repo.checkout(fn ->
          assert read_project().name == "original"
          :erlang.garbage_collect()
          send(parent, :warmed)
          await_signal(:compete)

          try do
            Repo.transaction(
              fn ->
                assert read_project().name == "writer"
                {1, nil} = Repo.update_all(ProjectRecord, set: [description: "waiter"])
                :waiter
              end,
              mode: :immediate
            )
          rescue
            error in Exqlite.Error -> {:error, Exception.message(error)}
          end
        end)
      end)

    await_signal(:warmed)

    writer =
      Task.async(fn ->
        Repo.put_dynamic_repo(repo)

        Repo.transaction(
          fn ->
            SQL.query!(repo, "UPDATE kyuubiki_projects SET name = 'writer'", [])
            send(parent, :locked)
            await_signal(:replace_handle)
            assert read_project().name == "writer"
            :erlang.garbage_collect()
            # Yield so deferred native resource destruction runs before COMMIT.
            Process.sleep(1)
            :writer
          end,
          mode: :immediate
        )
      end)

    await_signal(:locked)
    send(waiter.pid, :compete)
    await_busy(waiter.pid, System.monotonic_time(:millisecond) + 2_000)
    send(writer.pid, :replace_handle)
    assert {:ok, :writer} = Task.await(writer, 10_000)
    assert {:ok, :waiter} = Task.await(waiter, 10_000)
  end

  defp read_project, do: Repo.get!(ProjectRecord, "lock-order")

  defp returned_checkout(repo) do
    parent = self()

    writer =
      Task.async(fn ->
        Repo.put_dynamic_repo(repo)

        Repo.transaction(
          fn ->
            SQL.query!(repo, "UPDATE kyuubiki_projects SET name = 'writer'", [])
            send(parent, :locked)
            await_signal(:finish)
            :writer
          end,
          mode: :immediate
        )
      end)

    await_signal(:locked)

    reader =
      Task.async(fn ->
        # Keep dead query terms until after this connection has been lent again.
        Process.flag(:min_heap_size, 100_000)
        :erlang.garbage_collect()
        Repo.put_dynamic_repo(repo)
        Repo.checkout(fn -> assert read_project().name == "original" end)
        send(parent, :returned)
        await_signal(:collect)
        :erlang.garbage_collect()
        Process.sleep(1)
        send(parent, :collected)
      end)

    await_signal(:returned)

    waiter =
      Task.async(fn ->
        Repo.put_dynamic_repo(repo)

        Repo.transaction(
          fn ->
            assert read_project().name == "writer"
            {1, nil} = Repo.update_all(ProjectRecord, set: [description: "waiter"])
            :waiter
          end,
          mode: :immediate
        )
      end)

    await_busy(waiter.pid, System.monotonic_time(:millisecond) + 2_000)
    send(reader.pid, :collect)
    await_signal(:collected)
    send(writer.pid, :finish)
    Task.await(reader, 10_000)
    assert {:ok, :writer} = Task.await(writer, 10_000)
    assert {:ok, :waiter} = Task.await(waiter, 10_000)
  end

  defp await_signal(signal) do
    receive do
      ^signal -> :ok
    after
      5_000 -> flunk("SQLite contention barrier timed out: #{signal}")
    end
  end

  defp await_busy(pid, deadline) do
    if Process.info(pid, :current_function) ==
         {:current_function, {Exqlite.Sqlite3NIF, :execute, 2}} do
      :ok
    else
      assert System.monotonic_time(:millisecond) < deadline,
             "competing writer never entered the SQLite busy wait"

      Process.sleep(1)
      await_busy(pid, deadline)
    end
  end
end
