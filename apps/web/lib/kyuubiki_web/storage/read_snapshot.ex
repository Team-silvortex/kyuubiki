defmodule KyuubikiWeb.Storage.ReadSnapshot do
  @moduledoc false

  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.FailureBoundary

  # Only the SQL stores share a transaction boundary. Independent memory/JSON
  # stores keep their existing behavior; this is not a global backup primitive.
  def run(callback) when is_function(callback, 0) do
    FailureBoundary.run(fn ->
      if Storage.sql?(), do: sql_snapshot(callback), else: callback.()
    end)
  end

  defp sql_snapshot(callback) do
    repo = Storage.repo_module!()

    # An ambient transaction may already have read at weaker isolation. Refuse
    # it rather than silently advertise a snapshot we cannot establish.
    if apply(repo, :in_transaction?, []) do
      {:error, :analysis_store_unavailable}
    else
      options = if Storage.sqlite?(), do: [mode: :deferred], else: []

      case apply(repo, :transaction, [fn -> read(repo, callback) end, options]) do
        {:ok, reply} -> reply
        {:error, reason} -> {:error, reason}
      end
    end
  end

  defp read(repo, callback) do
    if Storage.postgres?() do
      SQL.query!(
        apply(repo, :get_dynamic_repo, []),
        "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY",
        []
      )
    end

    callback.()
  end
end
