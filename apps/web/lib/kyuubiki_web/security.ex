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
      configured = configured_token(scope)

      case extract_token(conn) do
        {:ok, token} when token == configured ->
          authorize_post_token(conn, scope)

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
      "cluster_token_configured" => cluster_token_configured?(),
      "cluster_agent_allowlist_enabled" => cluster_agent_allowlist_enabled?(),
      "cluster_agent_allowlist_count" => MapSet.size(cluster_allowed_agent_ids()),
      "cluster_cluster_allowlist_enabled" => cluster_cluster_allowlist_enabled?(),
      "cluster_cluster_allowlist_count" => MapSet.size(cluster_allowed_cluster_ids()),
      "cluster_fingerprint_required" => cluster_fingerprint_required?(),
      "cluster_timestamp_window_ms" => cluster_timestamp_window_ms(),
      "cluster_identity_headers_required" => cluster_token_required?(),
      "protect_reads" => protect_reads?(),
      "mutating_routes_protected" => token_configured?(),
      "cluster_routes_protected" => cluster_token_required?()
    }
  end

  def validate_cluster_registration_identity(conn, attrs) when is_map(attrs) do
    if cluster_token_required?() do
      with {:ok, header_agent_id} <- required_header(conn, "x-kyuubiki-agent-id"),
           {:ok, body_agent_id} <- fetch_string_field(attrs, "id"),
           true <- header_agent_id == body_agent_id,
           :ok <- validate_cluster_id_match(conn, attrs),
           :ok <- validate_cluster_fingerprint_presence(conn),
           :ok <- validate_cluster_registration_allowlist(conn, body_agent_id, attrs) do
        :ok
      else
        _ ->
          {:error, 401,
           %{
             "error" => "invalid_cluster_identity",
             "message" => "cluster identity headers do not match the registration payload"
           }}
      end
    else
      :ok
    end
  end

  def validate_cluster_agent_identity(conn, agent_id, attrs \\ %{})
      when is_binary(agent_id) and is_map(attrs) do
    if cluster_token_required?() do
      with {:ok, header_agent_id} <- required_header(conn, "x-kyuubiki-agent-id"),
           true <- header_agent_id == agent_id,
           :ok <- validate_cluster_id_match(conn, attrs),
           :ok <- validate_cluster_fingerprint_presence(conn),
           :ok <- validate_cluster_agent_allowlist(conn, agent_id, attrs) do
        :ok
      else
        _ ->
          {:error, 401,
           %{
             "error" => "invalid_cluster_identity",
             "message" => "cluster identity headers do not match the target agent"
           }}
      end
    else
      :ok
    end
  end

  def token_configured? do
    configured_token(:write) not in [nil, ""]
  end

  def cluster_token_configured? do
    direct_cluster_token() not in [nil, ""]
  end

  def protect_reads? do
    config()[:protect_reads?] == true
  end

  defp token_required?(:read), do: token_configured?() and protect_reads?()
  defp token_required?(:write), do: token_configured?()
  defp token_required?(:cluster), do: cluster_token_required?()

  defp authorize_post_token(conn, :cluster), do: validate_cluster_timestamp(conn)
  defp authorize_post_token(_conn, _scope), do: :ok

  defp cluster_token_required? do
    configured_token(:cluster) not in [nil, ""]
  end

  defp configured_token(:cluster) do
    direct_cluster_token() || configured_token(:write)
  end

  defp configured_token(:write) do
    config()[:api_token]
  end

  defp direct_cluster_token do
    config()[:cluster_api_token]
  end

  defp cluster_allowed_agent_ids do
    config()[:cluster_allowed_agent_ids] || MapSet.new()
  end

  defp cluster_allowed_cluster_ids do
    config()[:cluster_allowed_cluster_ids] || MapSet.new()
  end

  defp cluster_agent_allowlist_enabled? do
    MapSet.size(cluster_allowed_agent_ids()) > 0
  end

  defp cluster_cluster_allowlist_enabled? do
    MapSet.size(cluster_allowed_cluster_ids()) > 0
  end

  defp cluster_fingerprint_required? do
    config()[:cluster_require_fingerprint?] == true
  end

  defp config do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
  end

  defp cluster_timestamp_window_ms do
    config()[:cluster_timestamp_window_ms] || 30_000
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

  defp validate_cluster_timestamp(conn) do
    case Plug.Conn.get_req_header(conn, "x-kyuubiki-cluster-ts") do
      [] ->
        :ok

      [value | _] ->
        with {timestamp_ms, ""} <- Integer.parse(value),
             true <- timestamp_fresh?(timestamp_ms) do
          :ok
        else
          _ ->
            {:error, 401,
             %{
               "error" => "stale_cluster_request",
               "message" => "missing or stale cluster timestamp"
             }}
        end
    end
  end

  defp timestamp_fresh?(timestamp_ms) do
    abs(System.system_time(:millisecond) - timestamp_ms) <= cluster_timestamp_window_ms()
  end

  defp validate_cluster_id_match(conn, attrs) do
    case Plug.Conn.get_req_header(conn, "x-kyuubiki-cluster-id") do
      [] ->
        :ok

      [header_cluster_id | _] ->
        case Map.get(attrs, "cluster_id") || Map.get(attrs, :cluster_id) do
          nil -> :ok
          ^header_cluster_id -> :ok
          _ -> :error
        end
    end
  end

  defp validate_cluster_registration_allowlist(conn, agent_id, attrs) do
    with :ok <- validate_agent_id_allowlist(agent_id),
         :ok <- validate_cluster_id_allowlist(resolve_cluster_id(conn, attrs)) do
      :ok
    end
  end

  defp validate_cluster_agent_allowlist(conn, agent_id, attrs) do
    with :ok <- validate_agent_id_allowlist(agent_id),
         :ok <- validate_cluster_id_allowlist(resolve_cluster_id(conn, attrs)) do
      :ok
    end
  end

  defp validate_cluster_fingerprint_presence(conn) do
    if cluster_fingerprint_required?() do
      case cluster_fingerprint(conn) do
        {:ok, _} -> :ok
        :error -> :error
      end
    else
      :ok
    end
  end

  defp validate_agent_id_allowlist(agent_id) do
    if cluster_agent_allowlist_enabled?() and not MapSet.member?(cluster_allowed_agent_ids(), agent_id) do
      :error
    else
      :ok
    end
  end

  defp validate_cluster_id_allowlist(nil) do
    if cluster_cluster_allowlist_enabled?(), do: :error, else: :ok
  end

  defp validate_cluster_id_allowlist(cluster_id) do
    if cluster_cluster_allowlist_enabled?() and
         not MapSet.member?(cluster_allowed_cluster_ids(), cluster_id) do
      :error
    else
      :ok
    end
  end

  defp resolve_cluster_id(conn, attrs) do
    case Plug.Conn.get_req_header(conn, "x-kyuubiki-cluster-id") do
      [value | _] when value not in [nil, ""] ->
        value

      _ ->
        case Map.get(attrs, "cluster_id") || Map.get(attrs, :cluster_id) do
          value when is_binary(value) and value != "" -> value
          _ -> nil
        end
    end
  end

  def cluster_fingerprint(conn) do
    case Plug.Conn.get_req_header(conn, "x-kyuubiki-agent-fingerprint") do
      [value | _] when value not in [nil, ""] -> {:ok, value}
      _ -> :error
    end
  end

  defp required_header(conn, name) do
    case Plug.Conn.get_req_header(conn, name) do
      [value | _] when value not in [nil, ""] -> {:ok, value}
      _ -> :error
    end
  end

  defp fetch_string_field(attrs, key) do
    case Map.get(attrs, key) || Map.get(attrs, String.to_atom(key)) do
      value when is_binary(value) and value != "" -> {:ok, value}
      _ -> :error
    end
  end
end
