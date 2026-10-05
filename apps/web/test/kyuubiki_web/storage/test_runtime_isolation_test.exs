defmodule KyuubikiWeb.TestRuntimeIsolationTest do
  use ExUnit.Case, async: false

  @runtime Path.expand("../../../config/runtime.exs", __DIR__)
  @development_database Path.expand("../../../../../tmp/data/kyuubiki_dev.sqlite3", __DIR__)

  alias KyuubikiWeb.TestSupport.DisposableDatabase

  setup do
    keys = ~w(KYUUBIKI_STORAGE_BACKEND SQLITE_DATABASE_PATH)
    original = Map.new(keys, &{&1, System.get_env(&1)})
    Enum.each(keys, &System.delete_env/1)

    on_exit(fn ->
      Enum.each(original, fn
        {key, nil} -> System.delete_env(key)
        {key, value} -> System.put_env(key, value)
      end)
    end)

    :ok
  end

  test "default test runtime owns a fresh temporary SQLite directory" do
    config = read_runtime(:test)
    root = config[:test_database_root]
    assert is_binary(root)
    assert Path.dirname(root) == Path.expand(System.tmp_dir!())
    assert Path.basename(root) =~ "kyuubiki-web-tests-"
    assert config[KyuubikiWeb.SqliteRepo][:database] == Path.join(root, "kyuubiki.sqlite3")
    refute config[KyuubikiWeb.SqliteRepo][:database] == @development_database
  end

  test "independent test starts never reuse an owned database path" do
    first = read_runtime(:test)
    second = read_runtime(:test)
    refute first[:test_database_root] == second[:test_database_root]
    refute first[KyuubikiWeb.SqliteRepo][:database] == second[KyuubikiWeb.SqliteRepo][:database]
  end

  test "explicit SQLite paths remain caller-owned in every environment" do
    explicit = Path.join(System.tmp_dir!(), "caller-owned-kyuubiki.sqlite3")
    System.put_env("SQLITE_DATABASE_PATH", explicit)

    for env <- [:test, :dev, :prod] do
      config = read_runtime(env)
      assert config[KyuubikiWeb.SqliteRepo][:database] == explicit
      assert config[:test_database_root] == nil
    end
  end

  test "non-test defaults do not change the development database or claim cleanup ownership" do
    for env <- [:dev, :prod] do
      config = read_runtime(env)
      assert config[KyuubikiWeb.SqliteRepo][:database] == @development_database
      assert config[:test_database_root] == nil
    end
  end

  test "non-SQLite backends never allocate owned SQLite storage" do
    for backend <- ["memory", "json", "postgres"] do
      System.put_env("KYUUBIKI_STORAGE_BACKEND", backend)
      config = read_runtime(:test)
      assert config[:test_database_root] == nil
      assert config[KyuubikiWeb.SqliteRepo][:database] == @development_database
    end
  end

  test "cleanup removes only this run's database and journal files" do
    root = own_directory!()

    for suffix <- ["", "-wal", "-shm", "-journal"] do
      File.write!(Path.join(root, "kyuubiki.sqlite3" <> suffix), "disposable")
    end

    assert :ok = DisposableDatabase.cleanup(root)
    refute File.exists?(root)
    assert :ok = DisposableDatabase.cleanup(root)
  end

  test "cleanup refuses caller-owned directories and never recursively removes unexpected files" do
    root = own_directory!()
    File.write!(Path.join(root, "keep-me.txt"), "not a managed database file")
    assert {:error, reason} = DisposableDatabase.cleanup(root)
    assert reason in [:enotempty, :eexist]
    assert File.read!(Path.join(root, "keep-me.txt")) == "not a managed database file"

    assert {:error, :not_owned_test_database} =
             DisposableDatabase.cleanup(Path.join(root, "nested"))
  end

  defp own_directory! do
    root = read_runtime(:test)[:test_database_root]
    File.mkdir!(root)
    on_exit(fn -> File.rm_rf!(root) end)
    root
  end

  defp read_runtime(env), do: Config.Reader.read!(@runtime, env: env)[:kyuubiki_web]
end
