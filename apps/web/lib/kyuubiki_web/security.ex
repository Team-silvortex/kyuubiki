defmodule KyuubikiWeb.Security do
  @moduledoc """
  Small runtime security helpers for control-plane HTTP access.

  Kyuubiki keeps local workstation mode friction low by default, while still
  avoiding accidental exposure beyond the host machine. When no token is
  configured, mutating and cluster-management routes stay available only to
  loopback callers. Once a token is configured, protected routes require it.
  Read routes can also be protected by enabling `protect_reads?`.
  """

  @type scope :: :read | :write | :cluster
  @cluster_nonce_table :kyuubiki_cluster_nonce_cache

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
      authorize_without_token(conn, scope)
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
      "cluster_nonce_required" => true,
      "cluster_timestamp_window_ms" => cluster_timestamp_window_ms(),
      "cluster_identity_headers_required" => cluster_token_required?(),
      "protect_reads" => protect_reads?(),
      "mutating_routes_protected" => true,
      "cluster_routes_protected" => true,
      "loopback_only_without_token" => true
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

  def reset_cluster_nonce_cache do
    case :ets.whereis(@cluster_nonce_table) do
      :undefined -> :ok
      table -> :ets.delete_all_objects(table)
    end
  end

  defp token_required?(:read), do: token_configured?() and protect_reads?()
  defp token_required?(:write), do: token_configured?()
  defp token_required?(:cluster), do: cluster_token_required?()

  defp authorize_without_token(conn, :read) do
    if loopback_request?(conn) do
      :ok
    else
      {:error, 401,
       %{
         "error" => "unauthorized",
         "message" => "missing API token for non-local request"
       }}
    end
  end

  defp authorize_without_token(conn, scope) when scope in [:write, :cluster] do
    if loopback_request?(conn) do
      authorize_post_token(conn, scope)
    else
      {:error, 401,
       %{
         "error" => "unauthorized",
         "message" => "missing API token for non-local request"
       }}
    end
  end

  defp authorize_post_token(conn, :cluster) do
    with :ok <- validate_cluster_timestamp(conn),
         :ok <- validate_cluster_nonce(conn) do
      :ok
    end
  end

  defp authorize_post_token(_conn, _scope), do: :ok

  defp cluster_token_required? do
    configured_token(:cluster) not in [nil, ""]
  end

  defp configured_token(:cluster) do
    direct_cluster_token() || configured_token(:write)
  end

  defp configured_token(:read) do
    configured_token(:write)
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
        {:error, 401,
         %{
           "error" => "stale_cluster_request",
           "message" => "missing or stale cluster timestamp"
         }}

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

  defp validate_cluster_nonce(conn) do
    with {:ok, nonce} <- required_header(conn, "x-kyuubiki-cluster-nonce"),
         :ok <- validate_cluster_nonce_format(nonce),
         :ok <- remember_cluster_nonce(conn, nonce) do
      :ok
    else
      _ ->
        {:error, 401,
         %{
           "error" => "replayed_cluster_request",
           "message" => "missing, invalid, or replayed cluster nonce"
         }}
    end
  end

  defp timestamp_fresh?(timestamp_ms) do
    abs(System.system_time(:millisecond) - timestamp_ms) <= cluster_timestamp_window_ms()
  end

  defp validate_cluster_nonce_format(nonce) when byte_size(nonce) > 0 and byte_size(nonce) <= 160 do
    if String.match?(nonce, ~r/^[A-Za-z0-9._:-]+$/u) do
      :ok
    else
      :error
    end
  end

  defp validate_cluster_nonce_format(_nonce), do: :error

  defp remember_cluster_nonce(conn, nonce) do
    prune_expired_cluster_nonces()

    expires_at = System.system_time(:millisecond) + cluster_nonce_ttl_ms()
    cache_key = {cluster_nonce_scope(conn), nonce}

    if :ets.insert_new(cluster_nonce_table(), {cache_key, expires_at}) do
      :ok
    else
      :error
    end
  end

  defp cluster_nonce_scope(conn) do
    case Plug.Conn.get_req_header(conn, "x-kyuubiki-agent-id") do
      [agent_id | _] when agent_id not in [nil, ""] ->
        {:agent, agent_id}

      _ ->
        {:ip, conn.remote_ip}
    end
  end

  defp cluster_nonce_ttl_ms do
    cluster_timestamp_window_ms()
  end

  defp cluster_nonce_table do
    case :ets.whereis(@cluster_nonce_table) do
      :undefined ->
        :ets.new(@cluster_nonce_table, [
          :named_table,
          :public,
          :set,
          read_concurrency: true,
          write_concurrency: true
        ])

      table ->
        table
    end
  end

  defp prune_expired_cluster_nonces do
    now = System.system_time(:millisecond)

    :ets.select_delete(cluster_nonce_table(), [
      {
        {:"$1", :"$2"},
        [{:<, :"$2", now}],
        [true]
      }
    ])

    :ok
  end

  defp loopback_request?(conn) do
    case conn.remote_ip do
      {127, _, _, _} -> true
      {0, 0, 0, 0, 0, 0, 0, 1} -> true
      _ -> false
    end
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
    if cluster_agent_allowlist_enabled?() and
         not MapSet.member?(cluster_allowed_agent_ids(), agent_id) do
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
