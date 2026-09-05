defmodule KyuubikiWeb.Playground.AgentClient do
  @moduledoc """
  TCP client for the Rust FEM agent RPC process.
  """

  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutionSummary
  alias KyuubikiWeb.Orchestra.OperatorTaskReadiness
  alias KyuubikiWeb.Orchestra.DistributedRecovery
  alias KyuubikiWeb.Playground.AgentExecutionGate
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.Playground.AgentRpcTransport
  alias KyuubikiWeb.Playground.AgentRegistry

  @rpc_version 1
  @operator_control_opts [:job_id, :queue_timeout_ms, :request_timeout_ms, :orchestration]

  @spec solve_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_bar_1d", params, on_progress)
  end

  @spec solve_acoustic_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_acoustic_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_acoustic_bar_1d", params, on_progress)
  end

  @spec solve_thermal_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_bar_1d", params, on_progress)
  end

  @spec solve_heat_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_heat_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_heat_bar_1d", params, on_progress)
  end

  @spec solve_transient_heat_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_transient_heat_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_transient_heat_bar_1d", params, on_progress)
  end

  @spec solve_electrostatic_bar_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_electrostatic_bar_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_electrostatic_bar_1d", params, on_progress)
  end

  @spec solve_electrostatic_plane_triangle_2d(map(), (map() -> any())) ::
          {:ok, map()} | {:error, term()}
  def solve_electrostatic_plane_triangle_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_electrostatic_plane_triangle_2d", params, on_progress)
  end

  @spec solve_electrostatic_plane_quad_2d(map(), (map() -> any())) ::
          {:ok, map()} | {:error, term()}
  def solve_electrostatic_plane_quad_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_electrostatic_plane_quad_2d", params, on_progress)
  end

  @spec solve_electric_conduction_plane_quad_2d(map(), (map() -> any())) ::
          {:ok, map()} | {:error, term()}
  def solve_electric_conduction_plane_quad_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_electric_conduction_plane_quad_2d", params, on_progress)
  end

  @spec solve_heat_plane_triangle_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_heat_plane_triangle_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_heat_plane_triangle_2d", params, on_progress)
  end

  @spec solve_heat_plane_quad_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_heat_plane_quad_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_heat_plane_quad_2d", params, on_progress)
  end

  @spec solve_thermal_truss_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_truss_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_truss_2d", params, on_progress)
  end

  @spec solve_thermal_truss_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_truss_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_truss_3d", params, on_progress)
  end

  @spec solve_beam_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_beam_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_beam_1d", params, on_progress)
  end

  @spec solve_thermal_beam_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_beam_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_beam_1d", params, on_progress)
  end

  @spec solve_thermal_frame_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_frame_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_frame_2d", params, on_progress)
  end

  @spec solve_thermal_frame_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_frame_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_frame_3d", params, on_progress)
  end

  @spec solve_torsion_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_torsion_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_torsion_1d", params, on_progress)
  end

  @spec solve_spring_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_spring_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_spring_1d", params, on_progress)
  end

  @spec solve_transient_spring_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_transient_spring_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_transient_spring_1d", params, on_progress)
  end

  @spec solve_harmonic_spring_1d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_harmonic_spring_1d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_harmonic_spring_1d", params, on_progress)
  end

  @spec solve_spring_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_spring_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_spring_2d", params, on_progress)
  end

  @spec solve_spring_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_spring_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_spring_3d", params, on_progress)
  end

  @spec solve_truss_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_truss_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_truss_2d", params, on_progress)
  end

  @spec solve_truss_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_truss_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_truss_3d", params, on_progress)
  end

  @spec solve_plane_triangle_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_plane_triangle_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_plane_triangle_2d", params, on_progress)
  end

  @spec solve_thermal_plane_triangle_2d(map(), (map() -> any())) ::
          {:ok, map()} | {:error, term()}
  def solve_thermal_plane_triangle_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_plane_triangle_2d", params, on_progress)
  end

  @spec solve_plane_quad_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_plane_quad_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_plane_quad_2d", params, on_progress)
  end

  @spec solve_thermal_plane_quad_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_thermal_plane_quad_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_thermal_plane_quad_2d", params, on_progress)
  end

  @spec solve_frame_2d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_frame_2d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_frame_2d", params, on_progress)
  end

  @spec solve_frame_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_frame_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_frame_3d", params, on_progress)
  end

  @spec solve_solid_tetra_3d(map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def solve_solid_tetra_3d(params, on_progress \\ fn _progress -> :ok end) do
    request("solve_solid_tetra_3d", params, on_progress)
  end

  @spec cancel_job(String.t()) :: {:ok, map()} | {:error, term()}
  def cancel_job(job_id) when is_binary(job_id) do
    request("cancel_job", %{job_id: job_id})
  end

  @spec release_operator_package_job(String.t(), AgentPool.endpoint() | nil) ::
          {:ok, map()} | {:error, term()}
  def release_operator_package_job(job_id, endpoint \\ nil) when is_binary(job_id) do
    request_to_target("release_operator_package_job", %{job_id: job_id}, endpoint)
  end

  @spec run_operator_task_ir(map(), keyword() | (map() -> any()), (map() -> any())) ::
          {:ok, map()} | {:error, term()}
  def run_operator_task_ir(
        task_ir,
        opts_or_progress \\ [],
        on_progress \\ fn _progress -> :ok end
      )
      when is_map(task_ir) do
    {opts, progress_handler} = normalize_operator_task_rpc_opts(opts_or_progress, on_progress)

    with :ok <- OperatorTaskExecutionSummary.validate_digest(task_ir),
         {:ok, _summary} <- OperatorTaskExecutionSummary.build(task_ir),
         {:ok, result} <-
           request(
             OperatorTaskIR.agent_rpc_method(),
             OperatorTaskIR.agent_rpc_params(task_ir, opts),
             progress_handler,
             operator_task_routing_opts(task_ir, opts)
           ) do
      {:ok, OperatorTaskReadiness.normalize_agent_result(result)}
    end
  end

  @spec ping(AgentPool.endpoint() | nil) :: {:ok, map()} | {:error, term()}
  def ping(endpoint \\ nil) do
    request_to_target("ping", %{}, endpoint)
  end

  @spec describe_agent(AgentPool.endpoint() | nil) :: {:ok, map()} | {:error, term()}
  def describe_agent(endpoint \\ nil) do
    request_to_target("describe_agent", %{}, endpoint)
  end

  @spec request(String.t(), map(), (map() -> any())) :: {:ok, map()} | {:error, term()}
  def request(method, params, on_progress \\ fn _progress -> :ok end)
      when is_binary(method) and is_map(params) and is_function(on_progress, 1) do
    with {:ok, result, _endpoint} <- request_with_agent(method, params, on_progress) do
      {:ok, result}
    end
  end

  @spec request(String.t(), map(), (map() -> any()), keyword()) :: {:ok, map()} | {:error, term()}
  def request(method, params, on_progress, opts)
      when is_binary(method) and is_map(params) and is_function(on_progress, 1) and is_list(opts) do
    with {:ok, result, _endpoint} <- request_with_agent(method, params, on_progress, opts) do
      {:ok, result}
    end
  end

  defp normalize_operator_task_rpc_opts(on_progress, _default_progress)
       when is_function(on_progress, 1),
       do: {[], on_progress}

  defp normalize_operator_task_rpc_opts(opts, on_progress)
       when is_list(opts) and is_function(on_progress, 1),
       do: {opts, on_progress}

  defp operator_task_routing_opts(task_ir, opts) do
    task_ir
    |> OperatorTaskIR.agent_routing_opts()
    |> Keyword.merge(Keyword.take(opts, @operator_control_opts))
    |> maybe_override_retry_policy(opts)
    |> maybe_require_operator_package_runtime(Keyword.get(opts, :mode))
  end

  defp maybe_override_retry_policy(routing_opts, opts) do
    retry_safety = Keyword.get(opts, :retry_safety)
    replay_checkpoint = Keyword.get(opts, :replay_checkpoint)

    cond do
      retry_safety in [:idempotent, "idempotent"] ->
        Keyword.put(routing_opts, :retry_safety, retry_safety)

      retry_safety in [:checkpointed, "checkpointed"] and
          verified_replay_checkpoint?(replay_checkpoint) ->
        routing_opts
        |> Keyword.put(:retry_safety, retry_safety)
        |> Keyword.put(:replay_checkpoint, replay_checkpoint)

      true ->
        routing_opts
    end
  end

  defp verified_replay_checkpoint?(%{
         "operator_task_batch_checkpoint_verification_contract" =>
           "kyuubiki.operator_task_batch_checkpoint_verification/v1",
         "status" => "verified",
         "checkpoint_digest" => digest
       })
       when is_binary(digest),
       do: Regex.match?(~r/\A[0-9a-f]{64}\z/, digest)

  defp verified_replay_checkpoint?(_checkpoint), do: false

  defp maybe_require_operator_package_runtime(routing_opts, mode)
       when mode in [:execute, "execute"] do
    Keyword.put(routing_opts, :requires_operator_package_runtime, true)
  end

  defp maybe_require_operator_package_runtime(routing_opts, _mode), do: routing_opts

  @spec request_with_agent(String.t(), map(), (map() -> any()), keyword()) ::
          {:ok, map(), AgentPool.endpoint()} | {:error, term()}
  def request_with_agent(method, params, on_progress \\ fn _progress -> :ok end, opts \\ [])
      when is_binary(method) and is_map(params) and is_function(on_progress, 1) and is_list(opts) do
    request_id = request_id()
    opts = put_execution_lease(opts, request_id, method)
    request = build_request(request_id, method, params) |> put_rpc_job_id(method, opts)

    with :ok <- AgentRpcTransport.validate_request(request) do
      endpoints = AgentPool.checkout_endpoints(method, opts)

      case endpoints do
        [] -> {:error, no_matching_agent_error(method, opts)}
        _ -> attempt_request(endpoints, request_id, request, on_progress, opts, [])
      end
    end
  end

  defp build_request(request_id, method, params) do
    %{
      "rpc_version" => @rpc_version,
      "id" => request_id,
      "method" => method,
      "params" => params
    }
  end

  defp request_to_target(method, params, nil) do
    request(method, params)
  end

  defp request_to_target(method, params, endpoint) when is_map(endpoint) do
    request_id = request_id()
    request = build_request(request_id, method, params)
    normalized = normalize_endpoint(endpoint)

    case AgentRpcTransport.request(normalized, request_id, request, fn _ -> :ok end, []) do
      {:ok, result} ->
        :ok = AgentPool.report_success(normalized)
        {:ok, result}

      {:error, {:rpc_error, _code, _message} = reason} ->
        :ok = AgentPool.report_success(normalized)
        {:error, reason}

      {:error, reason} ->
        unless AgentRpcTransport.local_failure?(reason),
          do: AgentPool.report_failure(normalized, DistributedRecovery.health_reason(reason))

        {:error, reason}
    end
  end

  defp attempt_request([], _request_id, _request, _on_progress, _opts, failures) do
    {:error, {:all_agents_failed, Enum.reverse(failures)}}
  end

  defp attempt_request(endpoints, request_id, request, on_progress, opts, failures) do
    with :ok <- emit_queue_progress(on_progress, opts, endpoints) do
      case AgentExecutionGate.acquire(endpoints, request_id, queue_timeout_ms(opts)) do
        {:ok, endpoint, queue_metadata} ->
          remaining = Enum.reject(endpoints, &(&1.id == endpoint.id))

          endpoint_result =
            try do
              with :ok <- emit_dispatch_progress(on_progress, opts, endpoint, queue_metadata),
                   :ok <- authorize_dispatch(opts) do
                with_claimed_endpoint(endpoint, opts, fn ->
                  AgentRpcTransport.request(endpoint, request_id, request, on_progress, opts)
                end)
              end
            after
              _ = AgentExecutionGate.release(request_id)
            end

          handle_endpoint_result(
            endpoint_result,
            endpoint,
            remaining,
            request_id,
            request,
            on_progress,
            opts,
            failures
          )

        {:error, reason} ->
          {:error, reason}
      end
    end
  end

  defp handle_endpoint_result(
         {:ok, result},
         endpoint,
         _remaining,
         _request_id,
         _request,
         _on_progress,
         _opts,
         _failures
       ) do
    :ok = AgentPool.report_success(endpoint)
    {:ok, result, endpoint}
  end

  defp handle_endpoint_result(
         {:error, {:rpc_error, _code, _message} = reason},
         endpoint,
         _remaining,
         _request_id,
         _request,
         _on_progress,
         _opts,
         _failures
       ) do
    :ok = AgentPool.report_success(endpoint)
    {:error, reason}
  end

  defp handle_endpoint_result(
         {:error, reason},
         endpoint,
         remaining,
         request_id,
         request,
         on_progress,
         opts,
         failures
       ) do
    if AgentRpcTransport.local_failure?(reason) do
      {:error, reason}
    else
      handle_recoverable_endpoint_failure(
        reason,
        endpoint,
        remaining,
        request_id,
        request,
        on_progress,
        opts,
        failures
      )
    end
  end

  defp handle_recoverable_endpoint_failure(
         reason,
         endpoint,
         remaining,
         request_id,
         request,
         on_progress,
         opts,
         failures
       ) do
    receipt =
      DistributedRecovery.failure_receipt(
        endpoint,
        request["method"],
        reason,
        opts,
        length(remaining),
        length(failures) + 1
      )

    if DistributedRecovery.agent_health_failure?(receipt) do
      :ok = AgentPool.report_failure(endpoint, DistributedRecovery.health_reason(reason))
    end

    with :ok <- emit_recovery_progress(on_progress, opts, receipt) do
      cond do
        DistributedRecovery.retryable?(receipt) and remaining != [] ->
          attempt_request(remaining, request_id, request, on_progress, opts, [receipt | failures])

        DistributedRecovery.retryable?(receipt) ->
          {:error, {:all_agents_failed, Enum.reverse([receipt | failures])}}

        true ->
          {:error, {:agent_retry_blocked, receipt}}
      end
    end
  end

  defp no_matching_agent_error(method, opts) do
    {:no_matching_agent,
     %{}
     |> Map.put(:method, method)
     |> Map.put(
       :required_capabilities,
       opts |> Keyword.get(:required_capabilities, []) |> Enum.filter(&is_binary/1)
     )
     |> Map.put(
       :placement_tags,
       opts |> Keyword.get(:placement_tags, []) |> Enum.filter(&is_binary/1)
     )
     |> maybe_put_required_package_runtime(opts)}
  end

  defp maybe_put_required_package_runtime(error, opts) do
    if Keyword.get(opts, :requires_operator_package_runtime, false),
      do: Map.put(error, :required_operator_package_runtime, true),
      else: error
  end

  defp with_claimed_endpoint(endpoint, opts, fun)
       when is_map(endpoint) and is_list(opts) and is_function(fun, 0) do
    lease = Keyword.get(opts, :execution_lease, %{})

    case claim_execution(endpoint, opts) do
      :ok ->
        try do
          fun.()
        after
          release_execution(endpoint, lease)
        end

      {:error, _reason} = error ->
        error
    end
  end

  defp claim_execution(endpoint, opts) do
    lease = Keyword.get(opts, :execution_lease, %{})

    cond do
      not is_binary(Map.get(endpoint, :control_mode)) ->
        :ok

      not is_binary(Map.get(lease, :lease_id)) ->
        :ok

      true ->
        lease
        |> Map.new()
        |> maybe_put_agent_session_id(endpoint)
        |> then(&AgentRegistry.claim_execution(endpoint.id, &1))
        |> case do
          {:ok, _lease} -> :ok
          {:error, _reason} = error -> error
        end
    end
  end

  defp maybe_put_agent_session_id(lease, %{agent_session_id: session_id})
       when is_binary(session_id),
       do: Map.put(lease, "agent_session_id", session_id)

  defp maybe_put_agent_session_id(lease, _endpoint), do: lease

  defp release_execution(endpoint, lease) when is_map(endpoint) and is_map(lease) do
    if is_binary(Map.get(endpoint, :control_mode)) and is_binary(Map.get(lease, :lease_id)) do
      AgentRegistry.release_execution(endpoint.id, Map.get(lease, :lease_id))
    else
      :ok
    end
  end

  defp put_execution_lease(opts, request_id, method) do
    orchestration = Keyword.get(opts, :orchestration, %{})

    lease =
      %{
        "lease_id" => "lease:" <> request_id,
        "method" => method,
        "job_id" => Keyword.get(opts, :job_id)
      }
      |> maybe_put_lease_value(
        "control_mode",
        Map.get(orchestration, :control_mode) || Map.get(orchestration, "control_mode")
      )
      |> maybe_put_lease_value(
        "orch_id",
        Map.get(orchestration, :orch_id) || Map.get(orchestration, "orch_id")
      )
      |> maybe_put_lease_value(
        "orch_session_id",
        Map.get(orchestration, :orch_session_id) || Map.get(orchestration, "orch_session_id")
      )
      |> maybe_put_lease_value(
        "cluster_id",
        Map.get(orchestration, :cluster_id) || Map.get(orchestration, "cluster_id")
      )

    Keyword.put(opts, :execution_lease, lease)
  end

  defp maybe_put_lease_value(lease, _key, nil), do: lease

  defp maybe_put_lease_value(lease, key, value) when is_binary(key) and is_binary(value),
    do: Map.put(lease, key, value)

  defp maybe_put_lease_value(lease, _key, _value), do: lease

  defp put_rpc_job_id(request, method, opts)
       when is_map(request) and is_binary(method) and is_list(opts) do
    job_id = Keyword.get(opts, :job_id)
    job_bound = String.starts_with?(method, "solve_") or method == "run_operator_task_ir"

    if job_bound and is_binary(job_id) and job_id != "" do
      Map.update!(request, "params", &Map.put(&1, "job_id", job_id))
    else
      request
    end
  end

  defp normalize_endpoint(%{id: _id, host: _host, port: _port} = endpoint), do: endpoint

  defp normalize_endpoint(%{"host" => host, "port" => port} = endpoint) do
    %{
      id: Map.get(endpoint, "id", "#{host}:#{port}"),
      host: host,
      port: port
    }
  end

  @spec worker_id(AgentPool.endpoint()) :: String.t()
  def worker_id(endpoint), do: "rust-agent-rpc@#{endpoint.id}"

  defp request_id do
    :crypto.strong_rand_bytes(8) |> Base.encode16(case: :lower)
  end

  defp queue_timeout_ms(opts) do
    case Keyword.get(opts, :queue_timeout_ms) do
      value when is_integer(value) and value > 0 -> value
      _ -> configured_queue_timeout_ms()
    end
  end

  defp configured_queue_timeout_ms do
    Application.get_env(:kyuubiki_web, __MODULE__, [])
    |> Keyword.get(:queue_timeout_ms, 120_000)
  end

  defp emit_queue_progress(on_progress, opts, endpoints) do
    if Keyword.get(opts, :job_id) do
      snapshot = AgentExecutionGate.snapshot()

      AgentRpcTransport.emit_progress(on_progress, %{
        "stage" => "queued",
        "progress" => 0.0,
        "message" =>
          "waiting for agent capacity; queue_depth=#{snapshot.queued_request_count}; " <>
            "candidate_agents=#{length(endpoints)}; queue_timeout_ms=#{queue_timeout_ms(opts)}"
      })
    else
      :ok
    end
  end

  defp emit_dispatch_progress(on_progress, opts, endpoint, queue_metadata) do
    if Keyword.get(opts, :job_id) do
      AgentRpcTransport.emit_progress(on_progress, %{
        "stage" => "preprocessing",
        "progress" => 0.01,
        "scheduler" => %{
          "policy" => queue_metadata.selection_policy,
          "agent_id" => queue_metadata.selected_agent_id,
          "active_slots_before" => queue_metadata.active_slots_before,
          "active_slots_after" => queue_metadata.active_slots_after,
          "capacity_slots" => queue_metadata.capacity_slots,
          "utilization_before" => queue_metadata.utilization_before,
          "utilization_after" => queue_metadata.utilization_after
        },
        "message" =>
          "agent capacity acquired; agent_id=#{endpoint.id}; " <>
            "queue_wait_ms=#{queue_metadata.waited_ms}; " <>
            "scheduler=#{queue_metadata.selection_policy}; " <>
            "utilization=#{queue_metadata.utilization_after}; " <>
            "execution_timeout_ms=#{request_timeout_value(opts)}"
      })
    else
      :ok
    end
  end

  defp emit_recovery_progress(on_progress, opts, receipt) do
    if Keyword.get(opts, :job_id) do
      AgentRpcTransport.emit_progress(on_progress, %{
        "stage" => "recovering",
        "progress" => 0.01,
        "message" =>
          "agent request failed; reason_code=#{receipt.reason_code}; " <>
            "next_action=#{receipt.next_action}",
        "recovery" => receipt
      })
    else
      :ok
    end
  end

  defp request_timeout_value(opts) do
    AgentRpcTransport.request_timeout_ms(opts)
  end

  defp authorize_dispatch(opts) do
    case Keyword.get(opts, :before_dispatch) do
      callback when is_function(callback, 0) -> callback.()
      _ -> :ok
    end
  end
end
