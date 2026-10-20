defmodule KyuubikiWeb.Storage.DisposableDatabaseLifecycleTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.TestSupport.DatabaseFixture, as: DB

  @moduletag timeout: 40_000

  for mode <- [:autorun, :explicit] do
    test "#{mode}: repeated suites retain the database until the process exits" do
      {output, status} = run_child(unquote(mode))
      assert status == 0, output
      assert rounds(output) == ["1", "2", "3"]
      assert_cleaned(output)
    end
  end

  test "a failing repeated suite keeps its failure and cleans up at exit" do
    {output, status} = run_child(:failure)
    assert status == 2, output
    assert output =~ "intentional second-round failure"
    assert rounds(output) == ["1", "2"]
    assert_cleaned(output)
  end

  test "a test loading failure still cleans up without starting autorun" do
    {output, status} = run_child(:load_error)
    assert status == 1, output
    assert output =~ "intentional test loading failure"
    assert rounds(output) == []
    assert_cleaned(output)
  end

  test "caller-owned SQLite files survive cleanup of the disposable journal" do
    database = Path.join(DB.directory(), "caller-owned.sqlite3")
    {output, status} = run_child(:autorun, database)
    assert status == 0, output
    assert rounds(output) == ["1", "2", "3"]
    assert_cleaned(output)
    assert File.regular?(database)

    {:sqlite, db} = connection = DB.open!(database)

    try do
      assert {:ok, statement} = Exqlite.Sqlite3.prepare(db, "SELECT round FROM cleanup_probe")
      assert {:ok, [[3]]} = Exqlite.Sqlite3.fetch_all(db, statement)
      assert :ok = Exqlite.Sqlite3.release(db, statement)
    after
      DB.close!(connection)
    end
  end

  defp run_child(mode, database \\ nil) do
    paths = :code.get_path() |> Enum.flat_map(&["-pa", &1 |> List.to_string() |> Path.expand()])

    args =
      ["--erl", "+S 2:2 +SDcpu 1 +SDio 1"] ++
        paths ++
        [
          "-r",
          "test/support/disposable_database_child.exs",
          "-e",
          "KyuubikiWeb.TestSupport.DisposableDatabaseChild.run(#{inspect(mode)}, #{inspect(database)})"
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
      {output, status} = await_exit(port, "", System.monotonic_time(:millisecond) + 30_000)
      output = String.replace(output, "\r\n", "\n")

      case Regex.run(~r/^CLEANUP_ROOT (.+)$/m, output) do
        [_, root] -> on_exit(fn -> KyuubikiWeb.TestSupport.DisposableDatabase.cleanup(root) end)
        nil -> :ok
      end

      {output, status}
    after
      if Port.info(port), do: Port.close(port)
    end
  end

  defp await_exit(port, output, deadline) do
    remaining = max(deadline - System.monotonic_time(:millisecond), 0)

    receive do
      {^port, {:data, bytes}} -> await_exit(port, output <> bytes, deadline)
      {^port, {:exit_status, status}} -> {output, status}
    after
      remaining -> flunk("disposable database child timed out: #{output}")
    end
  end

  defp rounds(output),
    do: Regex.scan(~r/^CLEANUP_ROUND (\d+)$/m, output, capture: :all_but_first) |> List.flatten()

  defp assert_cleaned(output) do
    assert [_, root] = Regex.run(~r/^CLEANUP_ROOT (.+)$/m, output)
    assert Path.dirname(root) == Path.expand(System.tmp_dir!())
    assert String.starts_with?(Path.basename(root), "kyuubiki-web-tests-")
    refute File.exists?(root), "test database survived process exit: #{root}"
  end
end
