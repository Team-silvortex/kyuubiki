defmodule KyuubikiSdk.ControlPlaneClient do
  @moduledoc "HTTP client for kyuubiki.control-plane/http-v1."

  alias KyuubikiSdk.Auth
  alias KyuubikiSdk.Error

  @fem_job_paths %{
    "bar_1d" => "/api/v1/fem/axial-bar/jobs",
    "thermal_bar_1d" => "/api/v1/fem/thermal-bar-1d/jobs",
    "heat_bar_1d" => "/api/v1/fem/heat-bar-1d/jobs",
    "transient_heat_bar_1d" => "/api/v1/fem/transient-heat-bar-1d/jobs",
    "electrostatic_bar_1d" => "/api/v1/fem/electrostatic-bar-1d/jobs",
    "magnetostatic_bar_1d" => "/api/v1/fem/magnetostatic-bar-1d/jobs",
    "beam_1d" => "/api/v1/fem/beam-1d/jobs",
    "thermal_beam_1d" => "/api/v1/fem/thermal-beam-1d/jobs",
    "torsion_1d" => "/api/v1/fem/torsion-1d/jobs",
    "spring_1d" => "/api/v1/fem/spring-1d/jobs",
    "transient_spring_1d" => "/api/v1/fem/transient-spring-1d/jobs",
    "harmonic_spring_1d" => "/api/v1/fem/harmonic-spring-1d/jobs",
    "spring_2d" => "/api/v1/fem/spring-2d/jobs",
    "spring_3d" => "/api/v1/fem/spring-3d/jobs",
    "truss_2d" => "/api/v1/fem/truss-2d/jobs",
    "thermal_truss_2d" => "/api/v1/fem/thermal-truss-2d/jobs",
    "frame_2d" => "/api/v1/fem/frame-2d/jobs",
    "buckling_beam_1d" => "/api/v1/fem/buckling-beam-1d/jobs",
    "buckling_frame_2d" => "/api/v1/fem/buckling-frame-2d/jobs",
    "frame_2d_p_delta" => "/api/v1/fem/frame-2d-p-delta/jobs",
    "thermal_frame_2d" => "/api/v1/fem/thermal-frame-2d/jobs",
    "plane_triangle_2d" => "/api/v1/fem/plane-triangle-2d/jobs",
    "heat_plane_triangle_2d" => "/api/v1/fem/heat-plane-triangle-2d/jobs",
    "thermal_plane_triangle_2d" => "/api/v1/fem/thermal-plane-triangle-2d/jobs",
    "electrostatic_plane_triangle_2d" => "/api/v1/fem/electrostatic-plane-triangle-2d/jobs",
    "plane_quad_2d" => "/api/v1/fem/plane-quad-2d/jobs",
    "heat_plane_quad_2d" => "/api/v1/fem/heat-plane-quad-2d/jobs",
    "thermal_plane_quad_2d" => "/api/v1/fem/thermal-plane-quad-2d/jobs",
    "electrostatic_plane_quad_2d" => "/api/v1/fem/electrostatic-plane-quad-2d/jobs",
    "stokes_flow_triangle_2d" => "/api/v1/fem/stokes-flow-plane-triangle-2d/jobs",
    "stokes_flow_plane_triangle_2d" => "/api/v1/fem/stokes-flow-plane-triangle-2d/jobs",
    "stokes_flow_quad_2d" => "/api/v1/fem/stokes-flow-plane-quad-2d/jobs",
    "stokes_flow_plane_quad_2d" => "/api/v1/fem/stokes-flow-plane-quad-2d/jobs",
    "truss_3d" => "/api/v1/fem/truss-3d/jobs",
    "thermal_truss_3d" => "/api/v1/fem/thermal-truss-3d/jobs",
    "frame_3d" => "/api/v1/fem/frame-3d/jobs",
    "solid_tetra_3d" => "/api/v1/fem/solid-tetra-3d/jobs",
    "thermal_frame_3d" => "/api/v1/fem/thermal-frame-3d/jobs"
  }

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
  def list_workflow_catalog(client), do: request(client, :get, "/api/v1/workflows/catalog")

  def fetch_workflow_catalog_workflow(client, workflow_id),
    do: request(client, :get, "/api/v1/workflows/catalog/#{URI.encode(workflow_id)}")

  def list_workflow_operators(client, opts \\ []),
    do: request(client, :get, append_query("/api/v1/operators", opts))

  def fetch_workflow_operator(client, operator_id),
    do: request(client, :get, "/api/v1/operators/#{URI.encode(operator_id)}")

  def prepare_operator_task(client, task_ir),
    do: request(client, :post, "/api/v1/operator-tasks/prepare", %{"task" => task_ir})

  def execute_operator_task(client, task_ir),
    do: request(client, :post, "/api/v1/operator-tasks/execute", %{"task" => task_ir})

  def prepare_operator_task_batch(client, batch),
    do: request(client, :post, "/api/v1/operator-tasks/prepare-batch", %{"batch" => batch})

  def execute_operator_task_batch(client, batch),
    do: request(client, :post, "/api/v1/operator-tasks/execute-batch", %{"batch" => batch})

  def checkpoint_operator_task_batch(client, batch, opts \\ []) do
    payload =
      %{"batch" => batch}
      |> maybe_put_payload("preparation", Keyword.get(opts, :preparation))
      |> maybe_put_payload("execution", Keyword.get(opts, :execution))

    request(client, :post, "/api/v1/operator-tasks/checkpoint-batch", payload)
  end

  def verify_operator_task_batch_checkpoint(client, batch, checkpoint) do
    request(client, :post, "/api/v1/operator-tasks/verify-checkpoint-batch", %{
      "batch" => batch,
      "checkpoint" => checkpoint
    })
  end

  def plan_operator_task_batch_resume(client, batch, checkpoint) do
    request(client, :post, "/api/v1/operator-tasks/resume-plan-batch", %{
      "batch" => batch,
      "checkpoint" => checkpoint
    })
  end

  def list_jobs(client), do: request(client, :get, "/api/v1/jobs")
  def fetch_job(client, job_id), do: request(client, :get, "/api/v1/jobs/#{job_id}")

  def update_job(client, job_id, payload),
    do: request(client, :patch, "/api/v1/jobs/#{job_id}", payload)

  def cancel_job(client, job_id), do: request(client, :post, "/api/v1/jobs/#{job_id}/cancel")
  def delete_job(client, job_id), do: request(client, :delete, "/api/v1/jobs/#{job_id}")
  def create_axial_bar_job(client, payload), do: submit_fem_job(client, "bar_1d", payload)
  def create_truss_2d_job(client, payload), do: submit_fem_job(client, "truss_2d", payload)
  def create_truss_3d_job(client, payload), do: submit_fem_job(client, "truss_3d", payload)

  def create_plane_triangle_2d_job(client, payload),
    do: submit_fem_job(client, "plane_triangle_2d", payload)

  def submit_fem_job(client, solve_kind, payload) do
    case Map.fetch(@fem_job_paths, normalize_solve_kind(solve_kind)) do
      {:ok, path} -> request(client, :post, path, payload)
      :error -> {:error, Error.rpc("unsupported solve kind: #{solve_kind}")}
    end
  end

  def submit_workflow_catalog_job(client, workflow_id, input_artifacts \\ %{}),
    do:
      request(client, :post, "/api/v1/workflows/catalog/#{workflow_id}/jobs", %{
        "input_artifacts" => input_artifacts
      })

  def submit_workflow_graph_job(client, graph, input_artifacts \\ %{}),
    do:
      request(client, :post, "/api/v1/workflows/graph/jobs", %{
        "graph" => graph,
        "input_artifacts" => input_artifacts
      })

  def list_results(client), do: request(client, :get, "/api/v1/results")
  def fetch_result(client, job_id), do: request(client, :get, "/api/v1/results/#{job_id}")

  def fetch_result_chunk(client, job_id, kind, opts \\ []) do
    query =
      opts
      |> Enum.filter(fn {_key, value} -> not is_nil(value) end)
      |> Enum.map(fn {key, value} -> {to_string(key), to_string(value)} end)

    path = "/api/v1/results/#{job_id}/chunks/#{kind}"
    request(client, :get, path <> if(query == [], do: "", else: "?" <> URI.encode_query(query)))
  end

  def update_result(client, job_id, result),
    do: request(client, :patch, "/api/v1/results/#{job_id}", %{"result" => result})

  def delete_result(client, job_id), do: request(client, :delete, "/api/v1/results/#{job_id}")
  def export_database(client), do: request(client, :get, "/api/v1/export/database")

  def export_security_events(client, opts \\ []) do
    request(client, :get, append_query("/api/v1/export/security-events", opts))
  end

  def export_security_events_csv(client, opts \\ []) do
    request_text(client, :get, append_query("/api/v1/export/security-events.csv", opts))
  end

  defp request(client, method, path, payload \\ nil) do
    :inets.start()
    url = String.to_charlist(client.base_url <> path)

    headers =
      [{~c"content-type", ~c"application/json"}] ++
        if client.auth,
          do: [
            {String.to_charlist(client.auth.header_name),
             String.to_charlist(client.auth.header_value)}
          ],
          else: []

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

  defp request_text(client, method, path) do
    :inets.start()
    url = String.to_charlist(client.base_url <> path)

    headers =
      [{~c"content-type", ~c"application/json"}] ++
        if client.auth,
          do: [
            {String.to_charlist(client.auth.header_name),
             String.to_charlist(client.auth.header_value)}
          ],
          else: []

    options = [body_format: :binary]

    case :httpc.request(method, {url, headers}, [], options) do
      {:ok, {{_, status, _}, _headers, response_body}} when status in 200..299 ->
        {:ok, response_body}

      {:ok, {{_, status, _}, _headers, response_body}} ->
        {:error, Error.http(status, response_body)}

      {:error, reason} ->
        {:error, Error.transport(inspect(reason))}
    end
  end

  defp append_query(path, opts) do
    query =
      opts
      |> Enum.filter(fn {_key, value} -> not is_nil(value) and value != "" end)
      |> Enum.map(fn {key, value} -> {to_string(key), to_string(value)} end)

    path <> if(query == [], do: "", else: "?" <> URI.encode_query(query))
  end

  defp maybe_put_payload(payload, _key, nil), do: payload
  defp maybe_put_payload(payload, key, value), do: Map.put(payload, key, value)

  defp normalize_solve_kind(kind) when is_atom(kind),
    do: normalize_solve_kind(Atom.to_string(kind))

  defp normalize_solve_kind("axial_bar_1d"), do: "bar_1d"
  defp normalize_solve_kind(kind) when is_binary(kind), do: String.downcase(kind)
end
