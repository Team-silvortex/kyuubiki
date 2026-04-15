defmodule KyuubikiSdk.ControlPlaneClient do
  @moduledoc "HTTP client for kyuubiki.control-plane/http-v1."

  alias KyuubikiSdk.Auth
  alias KyuubikiSdk.Error

  defstruct [:base_url, :auth]

  def new(base_url, opts \\ []) do
    auth =
      case {Keyword.get(opts, :auth), Keyword.get(opts, :token)} do
        {auth, _token} when not is_nil(auth) -> auth
        {nil, token} when is_binary(token) -> Auth.access_token(token)
        _ -> nil
      end

    %__MODULE__{
      base_url: String.trim_trailing(base_url, "/"),
      auth: auth
    }
  end

  def health(client), do: request(client, :get, "/api/health")
  def protocol(client), do: request(client, :get, "/api/v1/protocol")
  def agents(client), do: request(client, :get, "/api/v1/protocol/agents")
  def list_jobs(client), do: request(client, :get, "/api/v1/jobs")
  def fetch_job(client, job_id), do: request(client, :get, "/api/v1/jobs/#{job_id}")
  def update_job(client, job_id, payload), do: request(client, :patch, "/api/v1/jobs/#{job_id}", payload)
  def cancel_job(client, job_id), do: request(client, :post, "/api/v1/jobs/#{job_id}/cancel")
  def delete_job(client, job_id), do: request(client, :delete, "/api/v1/jobs/#{job_id}")
  def create_axial_bar_job(client, payload), do: request(client, :post, "/api/v1/fem/axial-bar/jobs", payload)
  def create_truss_2d_job(client, payload), do: request(client, :post, "/api/v1/fem/truss-2d/jobs", payload)
  def create_truss_3d_job(client, payload), do: request(client, :post, "/api/v1/fem/truss-3d/jobs", payload)
  def create_plane_triangle_2d_job(client, payload), do: request(client, :post, "/api/v1/fem/plane-triangle-2d/jobs", payload)
  def list_results(client), do: request(client, :get, "/api/v1/results")
  def fetch_result(client, job_id), do: request(client, :get, "/api/v1/results/#{job_id}")

  def fetch_result_chunk(client, job_id, kind, opts \\ []) do
    query =
      opts
      |> Enum.filter(fn {_key, value} -> not is_nil(value) end)
      |> Enum.map(fn {key, value} -> {key |> to_string() |> String.to_charlist(), to_charlist(to_string(value))} end)

    path = "/api/v1/results/#{job_id}/chunks/#{kind}"
    request(client, :get, path <> if(query == [], do: "", else: "?" <> URI.encode_query(query)))
  end

  def update_result(client, job_id, result), do: request(client, :patch, "/api/v1/results/#{job_id}", %{"result" => result})
  def delete_result(client, job_id), do: request(client, :delete, "/api/v1/results/#{job_id}")
  def export_database(client), do: request(client, :get, "/api/v1/export/database")

  defp request(client, method, path, payload \\ nil) do
    :inets.start()
    url = String.to_charlist(client.base_url <> path)
    headers =
      [{"content-type", "application/json"}] ++
        if client.auth, do: [{client.auth.header_name, client.auth.header_value}], else: []

    body = if payload, do: Jason.encode!(payload), else: ""

    request =
      case method do
        :get -> {url, headers}
        _ -> {url, headers, ~c"application/json", body}
      end

    options = [body_format: :binary]

    case :httpc.request(method, request, [], options) do
      {:ok, {{_, status, _}, _headers, response_body}} when status in 200..299 ->
        {:ok, Jason.decode!(response_body)}

      {:ok, {{_, status, _}, _headers, response_body}} ->
        {:error, Error.http(status, response_body)}

      {:error, reason} ->
        {:error, Error.transport(inspect(reason))}
    end
  end
end
