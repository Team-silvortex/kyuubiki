defmodule KyuubikiWeb.Library.ProjectRequestSql do
  @moduledoc false
  alias KyuubikiWeb.Library.ProjectRequest
  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.{ProjectRecord, ProjectRequestRecord}

  # Caller owns the transaction, including the project and the receipt insert.
  def create(attrs, write) do
    case ProjectRequest.identify(attrs) do
      nil -> write.()
      identity -> replay_or_create(identity, write)
    end
  end

  defp replay_or_create(identity, write) do
    repo = Storage.repo_module!()

    if Storage.postgres?() do
      # Projects have no parent row to lock. Lock only this request identity;
      # hash collisions serialize unrelated requests but never merge them.
      <<lock_id::signed-64, _::binary>> = :crypto.hash(:sha256, identity.request_key)

      Ecto.Adapters.SQL.query!(
        apply(repo, :get_dynamic_repo, []),
        "SELECT pg_advisory_xact_lock($1)",
        [lock_id]
      )
    end

    case apply(repo, :get, [ProjectRequestRecord, identity.request_key]) do
      nil ->
        with {:ok, project} <- write.() do
          record =
            Ecto.Changeset.change(
              %ProjectRequestRecord{},
              ProjectRequest.receipt(identity, project)
            )

          apply(repo, :insert!, [record])

          {:ok, project}
        end

      receipt ->
        ProjectRequest.replay(
          receipt,
          identity,
          apply(repo, :get, [ProjectRecord, receipt.project_id]) != nil
        )
    end
  end
end
