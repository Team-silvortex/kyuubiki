defmodule KyuubikiWeb.TestSupport.DisposableDatabaseChild do
  @moduledoc false

  import ExUnit.Assertions

  def run(mode, caller_database) do
    # Bound a broken child without depending on a platform-specific kill command.
    spawn(fn ->
      Process.sleep(25_000)
      System.halt(99)
    end)

    Mix.start()
    Mix.env(:test)
    System.put_env("KYUUBIKI_STORAGE_BACKEND", "sqlite")

    if caller_database do
      System.put_env("SQLITE_DATABASE_PATH", caller_database)
    else
      System.delete_env("SQLITE_DATABASE_PATH")
    end

    for path <- ["config/config.exs", "config/runtime.exs"] do
      Application.put_all_env(Config.Reader.read!(path, env: :test))
    end

    Logger.configure(level: :warning)
    {:ok, _} = Application.ensure_all_started(:kyuubiki_web)
    root = Application.fetch_env!(:kyuubiki_web, :operator_dispatch_journal_test_root)
    IO.puts("CLEANUP_ROOT #{root}")

    journal = Path.join(root, "operator-task-dispatches")
    File.mkdir_p!(journal)
    record = Path.join(journal, String.duplicate("a", 32) <> ".json")
    File.write!(record, "{}")
    File.write!(record <> ".next", "{unfinished")

    query!("CREATE TABLE cleanup_probe (round INTEGER NOT NULL)")
    query!("INSERT INTO cleanup_probe VALUES (0)")
    :persistent_term.put({__MODULE__, :mode}, mode)
    :persistent_term.put({__MODULE__, :supervisor}, Process.whereis(KyuubikiWeb.Supervisor))

    :ok = Application.ensure_loaded(:ex_unit)
    ExUnit.configure(autorun: mode != :explicit, repeat_until_failure: 2)
    Code.require_file("test/test_helper.exs")

    if mode == :load_error, do: raise("intentional test loading failure")

    Code.eval_quoted(
      quote do
        defmodule KyuubikiWeb.DisposableDatabaseChildProbe do
          use ExUnit.Case, async: false

          test "the same live application and database survive every round" do
            KyuubikiWeb.TestSupport.DisposableDatabaseChild.check_round!()
          end
        end
      end
    )

    if mode == :explicit do
      assert %{failures: 0} = ExUnit.run()
      # An embedded caller may still need the application after ExUnit.run/0.
      assert Process.whereis(KyuubikiWeb.Supervisor)
      assert %{rows: [[3]]} = query!("SELECT round FROM cleanup_probe")
    end
  end

  def check_round! do
    assert Process.whereis(KyuubikiWeb.Supervisor) ==
             :persistent_term.get({__MODULE__, :supervisor})

    assert %{rows: [[previous]]} = query!("SELECT round FROM cleanup_probe")
    query!("UPDATE cleanup_probe SET round = round + 1")
    IO.puts("CLEANUP_ROUND #{previous + 1}")

    if :persistent_term.get({__MODULE__, :mode}) == :failure and previous == 1,
      do: flunk("intentional second-round failure")
  end

  defp query!(sql), do: Ecto.Adapters.SQL.query!(KyuubikiWeb.SqliteRepo, sql, [])
end
