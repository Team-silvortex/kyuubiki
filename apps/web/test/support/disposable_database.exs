defmodule KyuubikiWeb.TestSupport.DisposableDatabase do
  @moduledoc false

  @files ~w(kyuubiki.sqlite3 kyuubiki.sqlite3-wal kyuubiki.sqlite3-shm kyuubiki.sqlite3-journal)

  def cleanup(root) when is_binary(root) do
    if Path.dirname(Path.expand(root)) == Path.expand(System.tmp_dir!()) and
         String.starts_with?(Path.basename(root), "kyuubiki-web-tests-") do
      with :ok <- remove_database_files(root), do: remove_if_present(root, &File.rmdir/1)
    else
      {:error, :not_owned_test_database}
    end
  end

  defp remove_database_files(root) do
    Enum.reduce_while(@files, :ok, fn name, :ok ->
      case remove_if_present(Path.join(root, name), &File.rm/1) do
        :ok -> {:cont, :ok}
        error -> {:halt, error}
      end
    end)
  end

  defp remove_if_present(path, operation) do
    case operation.(path) do
      {:error, :enoent} -> :ok
      result -> result
    end
  end
end

if root = Application.get_env(:kyuubiki_web, :test_database_root) do
  ExUnit.after_suite(fn _result ->
    # Close SQLite/WAL writers before removing only this run's owned files.
    case Application.stop(:kyuubiki_web) do
      result when result in [:ok, {:error, {:not_started, :kyuubiki_web}}] ->
        case KyuubikiWeb.TestSupport.DisposableDatabase.cleanup(root) do
          :ok ->
            :ok

          {:error, reason} ->
            IO.warn("disposable test database cleanup failed: #{inspect(reason)}")
        end

      {:error, reason} ->
        IO.warn("disposable test database retained: application stop failed: #{inspect(reason)}")
    end
  end)
end
