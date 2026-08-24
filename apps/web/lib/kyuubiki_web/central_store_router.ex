defmodule KyuubikiWeb.CentralStoreRouter do
  @moduledoc false

  use Plug.Router

  alias KyuubikiWeb.CentralStore
  alias KyuubikiWeb.OperatorPackageDistributionStore
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

  get "/publisher-policy" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, CentralStore.publisher_policy())
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

  get "/artifact-admission-policy" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, CentralStore.artifact_admission_policy())
    end)
  end

  get "/publish-pipeline" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, CentralStore.publish_pipeline())
    end)
  end

  get "/database-status" do
    with_auth(conn, :read, fn conn ->
      respond_json(conn, 200, CentralStore.database_status())
    end)
  end

  get "/operator-packages/:package_id/:package_version/:target/resolve" do
    with_auth(conn, :read, fn conn ->
      case OperatorPackageDistributionStore.resolve(package_id, package_version, target) do
        {:ok, payload} ->
          respond_json(conn, 200, payload)

        {:error, :operator_package_target_unavailable} ->
          respond_json(conn, 404, %{"error" => "operator_package_target_unavailable"})

        {:error, reason} ->
          operator_package_error(conn, reason)
      end
    end)
  end

  get "/operator-packages/:package_id/:package_version/:target/:artifact_kind" do
    with_auth(conn, :read, fn conn ->
      if artifact_kind in ["manifest", "entrypoint"] do
        case OperatorPackageDistributionStore.send_artifact(
               conn,
               package_id,
               package_version,
               target,
               artifact_kind
             ) do
          {:ok, conn} -> conn
          {:error, reason} -> operator_package_error(conn, reason)
        end
      else
        respond_json(conn, 404, %{"error" => "operator_package_artifact_not_found"})
      end
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

  defp operator_package_error(conn, reason) do
    status =
      if reason in [
           :operator_package_distribution_root_unconfigured,
           :operator_package_distribution_root_unavailable
         ],
         do: 503,
         else: 422

    respond_json(conn, status, %{
      "error" => "operator_package_resolution_failed",
      "reason" => Atom.to_string(reason)
    })
  end
end
