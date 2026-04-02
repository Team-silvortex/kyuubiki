defmodule KyuubikiWeb.Storage do
  @moduledoc false

  def backend do
    Application.get_env(:kyuubiki_web, :storage_backend, :sqlite)
  end

  def postgres?, do: backend() == :postgres
  def sqlite?, do: backend() == :sqlite
  def memory?, do: backend() == :memory
  def sql?, do: postgres?() or sqlite?()

  def repo_module do
    case backend() do
      :postgres -> KyuubikiWeb.PostgresRepo
      :sqlite -> KyuubikiWeb.SqliteRepo
      _ -> nil
    end
  end

  def repo_module! do
    case repo_module() do
      nil -> raise "SQL storage backend is not configured"
      repo -> repo
    end
  end
end
