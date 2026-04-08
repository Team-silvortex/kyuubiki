defmodule KyuubikiWeb.Security do
  @moduledoc """
  Small runtime security helpers for control-plane HTTP access.

  Kyuubiki keeps local workstation mode friction low by default, so token
  enforcement is opt-in. Once a token is configured, mutating API routes and
  cluster-management routes require it. Read routes can also be protected by
  enabling `protect_reads?`.
  """

  @type scope :: :read | :write | :cluster

  def authorize(conn, scope) when scope in [:read, :write, :cluster] do
    if token_required?(scope) do
      configured = configured_token()

      case extract_token(conn) do
        {:ok, token} when token == configured ->
          :ok

        _ ->
          {:error, 401,
           %{
             "error" => "unauthorized",
             "message" => "missing or invalid API token"
           }}
      end
    else
      :ok
    end
  end

  def descriptor do
    %{
      "api_token_configured" => token_configured?(),
      "protect_reads" => protect_reads?(),
      "mutating_routes_protected" => token_configured?(),
      "cluster_routes_protected" => token_configured?()
    }
  end

  def token_configured? do
    configured_token() not in [nil, ""]
  end

  def protect_reads? do
    config()[:protect_reads?] == true
  end

  defp token_required?(:read), do: token_configured?() and protect_reads?()
  defp token_required?(:write), do: token_configured?()
  defp token_required?(:cluster), do: token_configured?()

  defp configured_token do
    config()[:api_token]
  end

  defp config do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
  end

  defp extract_token(conn) do
    with {:header, [value | _]} <- {:header, Plug.Conn.get_req_header(conn, "authorization")},
         "Bearer " <> token <- value do
      {:ok, token}
    else
      _ ->
        case Plug.Conn.get_req_header(conn, "x-kyuubiki-token") do
          [token | _] when token not in [nil, ""] -> {:ok, token}
          _ -> :error
        end
    end
  end
end
