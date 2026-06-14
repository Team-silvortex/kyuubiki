defmodule KyuubikiWeb.RouterSupport do
  @moduledoc false

  alias KyuubikiWeb.Security

  def respond_json(conn, status, payload) do
    conn
    |> Plug.Conn.put_resp_content_type("application/json")
    |> Plug.Conn.send_resp(status, Jason.encode!(payload))
  end

  def respond_text(conn, status, content_type, payload) do
    conn
    |> Plug.Conn.put_resp_content_type(content_type)
    |> Plug.Conn.send_resp(status, payload)
  end

  def unprocessable(conn, reason), do: respond_json(conn, 422, %{"error" => inspect(reason)})

  def request_base_url(conn) do
    trust_forwarded? = Application.get_env(:kyuubiki_web, :trust_forwarded_headers, false)

    scheme =
      if trust_forwarded? do
        case Plug.Conn.get_req_header(conn, "x-forwarded-proto") do
          [value | _] -> value
          _ -> Atom.to_string(conn.scheme)
        end
      else
        Atom.to_string(conn.scheme)
      end

    host =
      if trust_forwarded? do
        case Plug.Conn.get_req_header(conn, "x-forwarded-host") do
          [value | _] -> value
          _ -> conn.host
        end
      else
        conn.host
      end

    port =
      if trust_forwarded? do
        case Plug.Conn.get_req_header(conn, "x-forwarded-port") do
          [value | _] ->
            case Integer.parse(value) do
              {parsed, ""} -> parsed
              _ -> conn.port
            end

          _ ->
            case scheme do
              "https" -> 443
              "http" -> 80
              _ -> conn.port
            end
        end
      else
        conn.port
      end

    if (scheme == "http" and port == 80) or (scheme == "https" and port == 443) do
      "#{scheme}://#{host}"
    else
      "#{scheme}://#{host}:#{port}"
    end
  end

  def with_cluster_fingerprint(conn, attrs) when is_map(attrs) do
    case Security.cluster_fingerprint(conn) do
      {:ok, fingerprint} -> Map.put(attrs, "fingerprint", fingerprint)
      :error -> attrs
    end
  end

  def validate_registered_fingerprint(conn, agent_id) do
    case Enum.find(KyuubikiWeb.Playground.AgentRegistry.agents(), &(&1.id == agent_id)) do
      %{fingerprint: registered} when is_binary(registered) and registered != "" ->
        case Security.cluster_fingerprint(conn) do
          {:ok, ^registered} ->
            :ok

          _ ->
            {:error, 401,
             %{
               "error" => "invalid_cluster_identity",
               "message" => "cluster fingerprint does not match the registered agent identity"
             }}
        end

      _ ->
        :ok
    end
  end

  def with_auth(conn, scope, fun) do
    case Security.authorize(conn, scope) do
      :ok -> fun.(conn)
      {:error, status, payload} -> respond_json(conn, status, payload)
    end
  end
end
