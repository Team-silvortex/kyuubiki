defmodule KyuubikiSdk.ControlPlaneClient do
  @moduledoc "HTTP client for kyuubiki.control-plane/http-v1."

  defstruct [:base_url, :token]

  def new(base_url, opts \\ []) do
    %__MODULE__{
      base_url: String.trim_trailing(base_url, "/"),
      token: Keyword.get(opts, :token)
    }
  end

  def health(client), do: request(client, :get, "/api/health")
  def protocol(client), do: request(client, :get, "/api/v1/protocol")
  def agents(client), do: request(client, :get, "/api/v1/protocol/agents")
  def fetch_job(client, job_id), do: request(client, :get, "/api/v1/jobs/#{job_id}")
  def cancel_job(client, job_id), do: request(client, :post, "/api/v1/jobs/#{job_id}/cancel")
  def create_axial_bar_job(client, payload), do: request(client, :post, "/api/v1/fem/axial-bar/jobs", payload)
  def create_truss_2d_job(client, payload), do: request(client, :post, "/api/v1/fem/truss-2d/jobs", payload)
  def create_truss_3d_job(client, payload), do: request(client, :post, "/api/v1/fem/truss-3d/jobs", payload)
  def create_plane_triangle_2d_job(client, payload), do: request(client, :post, "/api/v1/fem/plane-triangle-2d/jobs", payload)

  defp request(client, method, path, payload \\ nil) do
    :inets.start()
    url = String.to_charlist(client.base_url <> path)
    headers =
      [{"content-type", "application/json"}] ++
        if client.token, do: [{"x-kyuubiki-token", client.token}], else: []

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
        {:error, {status, response_body}}

      {:error, reason} ->
        {:error, reason}
    end
  end
end
