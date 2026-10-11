defmodule KyuubikiWeb.Storage.ReadSnapshot do
  @moduledoc false

  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.FailureBoundary
  alias KyuubikiWeb.Storage.MemorySnapshot

  @context {__MODULE__, :sql_context}

  # SQL uses a read transaction; memory stores validate captured generations.
  # Neither mechanism is a full database or external-artifact backup primitive.
  def run(callback) when is_function(callback, 0) do
    FailureBoundary.run(fn ->
      if Storage.sql?(), do: sql_snapshot(callback), else: MemorySnapshot.run(callback)
    end)
  end

  defp sql_snapshot(callback) do
    repo = Storage.repo_module!()
    identity = {repo, apply(repo, :get_dynamic_repo, [])}

    case Process.get(@context) do
      ^identity ->
        if apply(repo, :in_transaction?, []),
          do: callback.(),
          else: {:error, :analysis_store_unavailable}

      nil ->
        start_snapshot(repo, identity, callback)

      _other_repository ->
        {:error, :analysis_store_unavailable}
    end
  end

  defp start_snapshot(repo, identity, callback) do
    # An ambient transaction may already have read at weaker isolation. Refuse
    # it rather than silently advertise a snapshot we cannot establish.
    if apply(repo, :in_transaction?, []) do
      {:error, :analysis_store_unavailable}
    else
      options = if Storage.sqlite?(), do: [mode: :deferred], else: []

      case apply(repo, :transaction, [fn -> read(repo, identity, callback) end, options]) do
        {:ok, reply} -> reply
        {:error, reason} -> {:error, reason}
      end
    end
  end

  defp read(repo, identity, callback) do
    if Storage.postgres?() do
      SQL.query!(
        apply(repo, :get_dynamic_repo, []),
        "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY",
        []
      )
    end

    # Only our established snapshot may be reused, and only on the same repo.
    Process.put(@context, identity)

    try do
      case callback.() do
        # A nested boundary can contain a query exception while PostgreSQL has
        # already aborted the transaction. Roll back with the original error.
        {:error, reason} -> apply(repo, :rollback, [reason])
        reply -> reply
      end
    after
      Process.delete(@context)
    end
  end
end
