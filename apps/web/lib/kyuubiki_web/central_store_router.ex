defmodule KyuubikiWeb.CentralStoreRouter do
  @moduledoc false

  use Plug.Router

  alias KyuubikiWeb.CentralStore
  import KyuubikiWeb.RouterSupport

  plug(:match)
  plug(:dispatch)

  get "/catalog" do
    with_auth(conn, :read, fn conn ->
      conn = fetch_query_params(conn)
      respond_json(conn, 200, CentralStore.catalog(conn.query_params))
    end)
  end

  get "/session-policy" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, CentralStore.session_policy())
    end)
  end

  get "/publish-policy" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, CentralStore.publish_policy())
    end)
  end

  get "/publish-readiness" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, CentralStore.publish_readiness())
    end)
  end

  get "/database-policy" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, CentralStore.database_policy())
    end)
  end

  get "/provenance-policy" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, CentralStore.provenance_policy())
    end)
  end

  get "/database-status" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, CentralStore.database_status())
    end)
  end

  get "/catalog/:kind/:entry_id" do
    with_auth(conn, :read, fn conn ->
      case CentralStore.fetch(kind, entry_id) do
        {:ok, payload} ->
          respond_json(conn, 200, payload)

        {:error, {:store_entry_not_found, _kind, _id}} ->
          not_found(conn, kind, entry_id)

        {:error, {:central_store_entry_not_found, _kind, _id}} ->
          not_found(conn, kind, entry_id)

        {:error, reason} ->
          unprocessable(conn, reason)
      end
    end)
  end

  match _ do
    respond_json(conn, 404, %{"error" => "not_found"})
  end

  defp not_found(conn, kind, entry_id) do
    respond_json(conn, 404, %{
      "error" => "central_store_entry_not_found",
      "kind" => kind,
      "id" => entry_id
    })
  end
end
