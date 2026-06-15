defmodule KyuubikiSdk.Session do
  @moduledoc "Higher-level AI-friendly session over control-plane and solver-rpc clients."

  alias KyuubikiSdk.ControlPlaneClient
  alias KyuubikiSdk.Error
  alias KyuubikiSdk.SolverRpcClient

  defstruct [:control_plane, :solver_rpc]

  def new(opts \\ []) do
    control_plane =
      case Keyword.get(opts, :control_plane) do
        nil ->
          case Keyword.get(opts, :base_url) do
            nil -> nil
            base_url ->
              ControlPlaneClient.new(
                base_url,
                auth: Keyword.get(opts, :auth),
                token: Keyword.get(opts, :token)
              )
          end

        client ->
          client
      end

    solver_rpc =
      case Keyword.get(opts, :solver_rpc) do
        nil ->
          case {Keyword.get(opts, :rpc_host), Keyword.get(opts, :rpc_port)} do
            {host, port} when is_binary(host) and is_integer(port) ->
              SolverRpcClient.new(host, port, timeout: Keyword.get(opts, :rpc_timeout, 15_000))

            _ ->
              nil
          end

        client ->
          client
      end

    %__MODULE__{control_plane: control_plane, solver_rpc: solver_rpc}
  end

  def submit_job(session, solve_kind, payload) do
    with {:ok, client} <- fetch_control_plane(session) do
      dispatch_control_plane(client, solve_kind, payload)
    end
  end

  def submit_workflow_catalog_job(session, workflow_id, input_artifacts \\ %{}) do
    with {:ok, client} <- fetch_control_plane(session) do
      ControlPlaneClient.submit_workflow_catalog_job(client, workflow_id, input_artifacts)
    end
  end

  def submit_workflow_graph_job(session, graph, input_artifacts \\ %{}) do
    with {:ok, client} <- fetch_control_plane(session) do
      ControlPlaneClient.submit_workflow_graph_job(client, graph, input_artifacts)
    end
  end

  def submit_jobs(session, jobs) when is_list(jobs) do
    Enum.reduce_while(jobs, {:ok, []}, fn %{"solve_kind" => solve_kind, "payload" => payload}, {:ok, acc} ->
      case submit_job(session, solve_kind, payload) do
        {:ok, result} -> {:cont, {:ok, [result | acc]}}
        {:error, reason} -> {:halt, {:error, reason}}
      end
    end)
    |> case do
      {:ok, results} -> {:ok, Enum.reverse(results)}
      error -> error
    end
  end

  def solve_direct(session, solve_kind, payload) do
    with {:ok, client} <- fetch_solver_rpc(session),
         {:ok, outcome} <- dispatch_solver_rpc(client, solve_kind, payload) do
      {:ok, outcome.result}
    end
  end

  def wait_for_job(session, job_id, opts \\ []) do
    with {:ok, client} <- fetch_control_plane(session) do
      poll_interval = Keyword.get(opts, :poll_interval, 1_000)
      timeout = Keyword.get(opts, :timeout, 300_000)
      terminal_statuses = MapSet.new(Keyword.get(opts, :terminal_statuses, ["completed", "failed", "cancelled"]))
      started_at = System.monotonic_time(:millisecond)
      do_wait_for_job(client, job_id, poll_interval, timeout, terminal_statuses, started_at, [], nil, nil)
    end
  end

  def submit_and_wait(session, solve_kind, payload, opts \\ []) do
    with {:ok, submitted} <- submit_job(session, solve_kind, payload),
         %{"job" => %{"job_id" => job_id}} <- submitted,
         {:ok, waited} <- wait_for_job(session, job_id, opts) do
      {:ok, Map.put(waited, :submitted, submitted)}
    else
      {:error, reason} -> {:error, reason}
      _ -> {:error, Error.rpc("submit response did not include job_id")}
    end
  end

  def submit_workflow_catalog_and_wait(session, workflow_id, input_artifacts \\ %{}, opts \\ []) do
    with {:ok, submitted} <- submit_workflow_catalog_job(session, workflow_id, input_artifacts),
         %{"job" => %{"job_id" => job_id}} <- submitted,
         {:ok, waited} <- wait_for_job(session, job_id, opts) do
      {:ok, Map.put(waited, :submitted, submitted)}
    else
      {:error, reason} -> {:error, reason}
      _ -> {:error, Error.rpc("submit response did not include job_id")}
    end
  end

  def submit_workflow_graph_and_wait(session, graph, input_artifacts \\ %{}, opts \\ []) do
    with {:ok, submitted} <- submit_workflow_graph_job(session, graph, input_artifacts),
         %{"job" => %{"job_id" => job_id}} <- submitted,
         {:ok, waited} <- wait_for_job(session, job_id, opts) do
      {:ok, Map.put(waited, :submitted, submitted)}
    else
      {:error, reason} -> {:error, reason}
      _ -> {:error, Error.rpc("submit response did not include job_id")}
    end
  end

  defp fetch_control_plane(%__MODULE__{control_plane: nil}),
    do: {:error, Error.transport("control plane client is not configured")}

  defp fetch_control_plane(%__MODULE__{control_plane: client}), do: {:ok, client}

  defp fetch_solver_rpc(%__MODULE__{solver_rpc: nil}),
    do: {:error, Error.transport("solver rpc client is not configured")}

  defp fetch_solver_rpc(%__MODULE__{solver_rpc: client}), do: {:ok, client}

  defp dispatch_control_plane(client, solve_kind, payload) do
    ControlPlaneClient.submit_fem_job(client, solve_kind, payload)
  end

  defp dispatch_solver_rpc(client, solve_kind, payload) do
    SolverRpcClient.solve_study(client, solve_kind, payload)
  end

  defp do_wait_for_job(client, job_id, poll_interval, timeout, terminal_statuses, started_at, history, last_status, last_progress) do
    if System.monotonic_time(:millisecond) - started_at > timeout do
      {:error, Error.timeout("timed out waiting for job #{job_id}")}
    else
      case ControlPlaneClient.fetch_job(client, job_id) do
        {:ok, payload} ->
          job = Map.get(payload, "job", %{})
          status = Map.get(job, "status")
          progress = Map.get(job, "progress")

          next_history =
            if status != last_status or progress != last_progress do
              history ++ [payload]
            else
              history
            end

          if MapSet.member?(terminal_statuses, status) do
            {:ok, %{terminal: payload, history: next_history}}
          else
            Process.sleep(poll_interval)
            do_wait_for_job(client, job_id, poll_interval, timeout, terminal_statuses, started_at, next_history, status, progress)
          end

        error ->
          error
      end
    end
  end
end
