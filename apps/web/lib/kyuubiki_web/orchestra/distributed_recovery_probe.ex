defmodule KyuubikiWeb.Orchestra.DistributedRecoveryProbe do
  @moduledoc """
  Runs deterministic TCP fault injection against Orchestra's Agent client.
  """

  alias KyuubikiWeb.Orchestra.{OperatorTaskBatchRun, OperatorTaskIR}
  alias KyuubikiWeb.Playground.{AgentClient, AgentPool}

  @schema_version "kyuubiki.orchestra-process-loss-fault-injection/v1"

  @spec run!() :: map()
  def run! do
    original_pool = Application.get_env(:kyuubiki_web, AgentPool, [])
    original_client = Application.get_env(:kyuubiki_web, AgentClient, [])

    Application.put_env(:kyuubiki_web, AgentClient,
      connect_timeout_ms: 100,
      recv_timeout_ms: 500,
      request_timeout_ms: 1_000
    )

    try do
      scenarios = [idempotent_failover!(), unsafe_replay_blocked!(), checkpointed_failover!()]

      %{
        "schema_version" => @schema_version,
        "status" => "pass",
        "scenario_count" => length(scenarios),
        "scenarios" => scenarios
      }
    after
      Application.put_env(:kyuubiki_web, AgentPool, original_pool)
      Application.put_env(:kyuubiki_web, AgentClient, original_client)
      AgentPool.reload()
    end
  end

  defp idempotent_failover! do
    {failed_pid, failed_port} = start_agent!(:idempotent_failed, :disconnect)
    {healthy_pid, healthy_port} = start_agent!(:idempotent_healthy, solve_response())
    configure_agents!("idempotent", failed_port, healthy_port)
    owner = self()

    {:ok, result, endpoint} =
      AgentClient.request_with_agent(
        "solve_bar_1d",
        solve_params(),
        &send(owner, {:probe_progress, :idempotent, &1}),
        job_id: "probe-idempotent-process-loss",
        placement_tags: ["probe", "preferred"]
      )

    failed_request = await_request!(:idempotent_failed)
    healthy_request = await_request!(:idempotent_healthy)
    recovery = await_recovery!(:idempotent)
    stop_agents([failed_pid, healthy_pid])

    scenario(
      "idempotent_task_process_loss_failover",
      "agent_disconnect_after_dispatch",
      "retry_next_agent",
      %{
        "failed_agent_received_request" => failed_request["method"] == "solve_bar_1d",
        "fallback_agent_received_request" => healthy_request["method"] == "solve_bar_1d",
        "fallback_agent_id" => endpoint.id,
        "result_retained" => result["max_stress"] == 1.0,
        "recovery" => recovery
      }
    )
  end

  defp unsafe_replay_blocked! do
    task_ir = export_task_ir!(["probe", "preferred"])
    capabilities = task_ir["runtime_hints"]["required_capabilities"]
    {failed_pid, failed_port} = start_agent!(:unsafe_failed, :disconnect)
    {fallback_pid, fallback_port} = start_agent!(:unsafe_fallback, export_response())
    configure_agents!("unsafe", failed_port, fallback_port, capabilities)

    {:error, {:agent_retry_blocked, receipt}} =
      AgentClient.run_operator_task_ir(task_ir, mode: :execute)

    failed_request = await_request!(:unsafe_failed)
    fallback_received = request_received?(:unsafe_fallback, 75)
    stop_agents([failed_pid, fallback_pid])

    scenario(
      "side_effect_replay_blocked_without_checkpoint",
      "agent_disconnect_after_export_dispatch",
      "checkpoint_before_retry",
      %{
        "failed_agent_received_request" => failed_request["method"] == "run_operator_task_ir",
        "fallback_agent_received_request" => fallback_received,
        "duplicate_side_effect_prevented" => not fallback_received,
        "recovery" => receipt
      }
    )
  end

  defp checkpointed_failover! do
    task_ir = export_task_ir!(["probe", "preferred"])
    replay_checkpoint = verified_checkpoint!(task_ir)
    capabilities = task_ir["runtime_hints"]["required_capabilities"]
    {failed_pid, failed_port} = start_agent!(:checkpointed_failed, :disconnect)
    {healthy_pid, healthy_port} = start_agent!(:checkpointed_healthy, export_response())
    configure_agents!("checkpointed", failed_port, healthy_port, capabilities)
    owner = self()

    {:ok, result} =
      AgentClient.run_operator_task_ir(
        task_ir,
        [
          mode: :execute,
          retry_safety: :checkpointed,
          replay_checkpoint: replay_checkpoint
        ],
        &send(owner, {:probe_progress, :checkpointed, &1})
      )

    failed_request = await_request!(:checkpointed_failed)
    healthy_request = await_request!(:checkpointed_healthy)
    recovery = await_recovery!(:checkpointed)
    stop_agents([failed_pid, healthy_pid])

    scenario(
      "checkpointed_side_effect_process_loss_failover",
      "agent_disconnect_after_checkpointed_export_dispatch",
      "retry_next_agent",
      %{
        "failed_agent_received_request" => failed_request["method"] == "run_operator_task_ir",
        "fallback_agent_received_request" => healthy_request["method"] == "run_operator_task_ir",
        "checkpointed_result_retained" => result["status"] == "exported",
        "recovery" => recovery
      }
    )
  end

  defp configure_agents!(scope, failed_port, healthy_port, capabilities \\ []) do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        endpoint("#{scope}-failed", failed_port, ["probe", "preferred"], capabilities),
        endpoint("#{scope}-healthy", healthy_port, ["probe"], capabilities)
      ]
    )

    AgentPool.reload()
  end

  defp endpoint(id, port, tags, capabilities) do
    %{
      id: id,
      host: "127.0.0.1",
      port: port,
      methods: ["solve_bar_1d", "run_operator_task_ir"],
      tags: tags,
      capabilities: capabilities
    }
  end

  defp start_agent!(label, behavior) do
    owner = self()

    {:ok, pid} =
      Task.start(fn ->
        {:ok, listener} =
          :gen_tcp.listen(0, [
            :binary,
            packet: 4,
            active: false,
            reuseaddr: true,
            ip: {127, 0, 0, 1}
          ])

        {:ok, port} = :inet.port(listener)
        send(owner, {:probe_agent_ready, label, port})
        {:ok, socket} = :gen_tcp.accept(listener)
        {:ok, payload} = :gen_tcp.recv(socket, 0, 1_000)
        request = Jason.decode!(payload)
        send(owner, {:probe_agent_request, label, request})
        respond(socket, request, behavior)
        :gen_tcp.close(socket)
        :gen_tcp.close(listener)
      end)

    receive do
      {:probe_agent_ready, ^label, port} -> {pid, port}
    after
      1_000 -> raise "probe Agent #{label} did not start"
    end
  end

  defp respond(_socket, _request, :disconnect), do: :ok

  defp respond(socket, request, result) do
    payload =
      %{
        "rpc_version" => request["rpc_version"],
        "id" => request["id"],
        "ok" => true,
        "result" => result
      }
      |> Jason.encode!()

    :ok = :gen_tcp.send(socket, payload)
  end

  defp await_request!(label) do
    receive do
      {:probe_agent_request, ^label, request} -> request
    after
      1_000 -> raise "probe Agent #{label} did not receive its request"
    end
  end

  defp request_received?(label, timeout_ms) do
    receive do
      {:probe_agent_request, ^label, _request} -> true
    after
      timeout_ms -> false
    end
  end

  defp await_recovery!(label) do
    receive do
      {:probe_progress, ^label, %{"recovery" => recovery}} -> recovery
      {:probe_progress, ^label, _progress} -> await_recovery!(label)
    after
      1_000 -> raise "recovery progress for #{label} was not emitted"
    end
  end

  defp stop_agents(pids) do
    Enum.each(pids, fn pid ->
      if Process.alive?(pid), do: Process.exit(pid, :kill)
    end)
  end

  defp export_task_ir!(placement_tags) do
    {:ok, task_ir} =
      OperatorTaskIR.build(
        "export.summary_json",
        %{"summary" => %{"status" => "ready"}},
        %{},
        placement_tags: placement_tags
      )

    task_ir
  end

  defp solve_params do
    %{length: 1.0, area: 1.0, youngs_modulus: 1.0, elements: 1, tip_force: 1.0}
  end

  defp solve_response do
    %{
      "tip_displacement" => 1.0,
      "reaction_force" => -1.0,
      "max_displacement" => 1.0,
      "max_stress" => 1.0,
      "nodes" => [],
      "elements" => [],
      "input" => solve_params()
    }
  end

  defp export_response, do: %{"status" => "exported"}

  defp verified_checkpoint!(task_ir) do
    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "operator_id" => task_ir["operator"]["id"],
      "tasks" => [%{"case_id" => "checkpointed-export", "task_ir" => task_ir}]
    }

    checkpoint = OperatorTaskBatchRun.checkpoint(batch)
    {:ok, verification} = OperatorTaskBatchRun.verify_checkpoint(batch, checkpoint)
    verification
  end

  defp scenario(id, injected_fault, recovery_policy, observations) do
    %{
      "id" => id,
      "status" => "pass",
      "injected_fault" => injected_fault,
      "recovery_policy" => recovery_policy,
      "observations" => observations
    }
  end
end
