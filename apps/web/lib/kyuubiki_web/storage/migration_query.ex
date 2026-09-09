defmodule KyuubikiWeb.Storage.MigrationQuery do
  @moduledoc false
  alias Exqlite.Sqlite3

  def rows!(connection, sql, params \\ [])

  def rows!({:sqlite, db}, sql, params) do
    {:ok, statement} = Sqlite3.prepare(db, sql)

    try do
      :ok = Sqlite3.bind(statement, params)
      {:ok, rows} = Sqlite3.fetch_all(db, statement)
      rows
    after
      Sqlite3.release(db, statement)
    end
  end

  def rows!(repo, sql, params),
    do: Ecto.Adapters.SQL.query!(repo.get_dynamic_repo(), sql, params).rows || []

  def transaction!({:sqlite, db} = connection, :sqlite, fun) do
    :ok = Sqlite3.execute(db, "BEGIN IMMEDIATE")

    try do
      result = fun.()
      rows!(connection, "COMMIT")
      result
    catch
      kind, reason ->
        Sqlite3.execute(db, "ROLLBACK")
        :erlang.raise(kind, reason, __STACKTRACE__)
    end
  end

  def transaction!(repo, backend, fun) do
    options = if backend == :sqlite, do: [mode: :immediate], else: []

    case repo.transaction(
           fn ->
             lock!(repo, backend)
             fun.()
           end,
           options
         ) do
      {:ok, result} -> result
      {:error, reason} -> raise "database migration rolled back: #{inspect(reason)}"
    end
  end

  defp lock!(repo, :postgres) do
    # Transaction-scoped, database-local serialization; no permanently retained lock.
    unless rows!(repo, "SELECT pg_try_advisory_xact_lock(1264145749, 1)") == [[true]],
      do: raise("another database migration owns the transaction lock; retry after it completes")
  end

  defp lock!(_repo, :sqlite), do: :ok
end
