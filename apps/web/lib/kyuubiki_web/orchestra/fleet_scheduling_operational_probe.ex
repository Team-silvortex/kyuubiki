defmodule KyuubikiWeb.Orchestra.FleetSchedulingOperationalProbe do
  @moduledoc """
  Exercises Orchestra fleet scheduling against externally managed Rust Agents.

  The native qualification runner owns package installation, process lifecycle,
  fault injection, and cleanup. This probe owns only Orchestra routing policy.
  """

  alias KyuubikiWeb.Playground.{AgentClient, AgentExecutionGate, AgentPool}

  @schema_version "kyuubiki.fleet-scheduling-operational-probe/v1"
  @high_id "fleet-high-capacity"
  @low_id "fleet-low-capacity"
  @high_capacity 3
  @low_capacity 1
  @solver_params %{
    length: 1.0,
    area: 2.0,
    youngs_modulus: 1_000.0,
    elements: 64,
    tip_force: 20.0
  }

  @spec run_baseline_from_env!() :: map()
  def run_baseline_from_env! do
    start_isolated_runtime!()

    with_runtime(fn high, low ->
      descriptors = [describe!(high), describe!(low)]
      capacity = exercise_capacity_distribution!([high, low])
      solver_runs = [solve_on!(high, "fleet-baseline-high"), solve_on!(low, "fleet-baseline-low")]

      write_report!(%{
        "schema_version" => @schema_version,
        "status" => "pass",
        "phase" => "baseline",
        "agent_descriptors" => descriptors,
        "capacity_distribution" => capacity,
        "solver_runs" => solver_runs
      })
    end)
  end

  @spec run_failover_recovery_from_env!() :: map()
  def run_failover_recovery_from_env! do
    start_isolated_runtime!()

    with_runtime(fn high, _low ->
      failover = solve_with_fleet!("fleet-failover")
      health = fleet_health(high)
      cooldown = solve_with_fleet!("fleet-cooldown")

      unless failover["selected_agent_id"] == @low_id and
               Enum.map(failover["scheduler_events"], & &1["agent_id"]) == [@high_id, @low_id] and
               is_map(failover["recovery"]) and cooldown["selected_agent_id"] == @low_id and
               health["cooling_down_count"] == 1 and health["failed_agent_cooling_down"] do
        raise "fleet fallback or cooldown contract was not observed"
      end

      signal_ready_for_restart!()
      wait_for_restart_release!()

      {:ok, ping} = AgentClient.ping(high)
      AgentPool.reload()
      resumed = solve_with_fleet!("fleet-resumed")
      recovered_health = fleet_health(high)

      unless ping["pong"] == true and resumed["selected_agent_id"] == @high_id and
               recovered_health["cooling_down_count"] == 0 do
        raise "restarted high-capacity Agent did not resume scheduling"
      end

      write_report!(%{
        "schema_version" => @schema_version,
        "status" => "pass",
        "phase" => "failover_recovery",
        "failover" => failover,
        "cooldown" => Map.put(cooldown, "health", health),
        "recovery" => %{
          "ping_verified" => ping["pong"],
          "resumed_run" => resumed,
          "health" => recovered_health
        }
      })
    end)
  end

  defp with_runtime(fun) do
    high = endpoint(@high_id, fetch_port!("KYUUBIKI_QUAL_HIGH_PORT"), @high_capacity)
    low = endpoint(@low_id, fetch_port!("KYUUBIKI_QUAL_LOW_PORT"), @low_capacity)
    original_pool = Application.get_env(:kyuubiki_web, AgentPool, [])
    original_client = Application.get_env(:kyuubiki_web, AgentClient, [])

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [high, low],
      failure_cooldown_ms: 30_000,
      failure_cooldown_max_ms: 30_000
    )

    Application.put_env(:kyuubiki_web, AgentClient,
      connect_timeout_ms: 300,
      recv_timeout_ms: 5_000,
      request_timeout_ms: 15_000,
      queue_timeout_ms: 5_000
    )

    try do
      AgentPool.reload()
      fun.(high, low)
    after
      Application.put_env(:kyuubiki_web, AgentPool, original_pool)
      Application.put_env(:kyuubiki_web, AgentClient, original_client)
      AgentPool.reload()
      clear_events()
    end
  end

  defp start_isolated_runtime! do
    ensure_started!(AgentPool)
    ensure_started!(AgentExecutionGate)
  end

  defp ensure_started!(module) do
    case Process.whereis(module) do
      nil ->
        case module.start_link([]) do
          {:ok, _pid} -> :ok
          {:error, {:already_started, _pid}} -> :ok
          {:error, reason} -> raise "failed to start #{inspect(module)}: #{inspect(reason)}"
        end

      _pid ->
        :ok
    end
  end

  defp exercise_capacity_distribution!(endpoints) do
    leases =
      Enum.map(1..(@high_capacity + @low_capacity), fn index ->
        lease_id = "fleet-capacity-#{index}"
        {:ok, endpoint, scheduling} = AgentExecutionGate.acquire(endpoints, lease_id, 1_000)
        %{lease_id: lease_id, endpoint: endpoint, scheduling: scheduling}
      end)

    try do
      sequence = Enum.map(leases, & &1.endpoint.id)
      counts = Enum.frequencies(sequence)
      snapshot = AgentExecutionGate.snapshot(endpoints)

      unless sequence == [@high_id, @low_id, @high_id, @high_id] and
               counts == %{@high_id => @high_capacity, @low_id => @low_capacity} and
               snapshot.selection_policy == "least_utilized_capacity_v1" do
        raise "capacity-normalized scheduling distribution drifted"
      end

      %{
        "policy" => snapshot.selection_policy,
        "declared_capacity" => snapshot.capacity_by_endpoint,
        "lease_sequence" => sequence,
        "selected_counts" => counts,
        "decisions" =>
          Enum.map(leases, fn lease ->
            Map.take(lease.scheduling, [
              :selection_policy,
              :selected_agent_id,
              :active_slots_before,
              :active_slots_after,
              :capacity_slots,
              :utilization_before,
              :utilization_after
            ])
          end),
        "snapshot" => %{
          "active_lease_count" => snapshot.active_lease_count,
          "capacity_slots" => snapshot.capacity_slots,
          "active_by_endpoint" => snapshot.active_by_endpoint,
          "utilization_by_endpoint" => snapshot.utilization_by_endpoint
        }
      }
    after
      Enum.each(Enum.reverse(leases), fn lease ->
        :ok = AgentExecutionGate.release(lease.lease_id)
      end)
    end
  end

  defp describe!(endpoint) do
    {:ok, descriptor} = AgentClient.describe_agent(endpoint)

    unless descriptor["program"] == "kyuubiki-rust-agent" do
      raise "fleet endpoint is not a Rust Agent"
    end

    %{
      "agent_id" => endpoint.id,
      "program" => descriptor["program"],
      "role" => descriptor["role"],
      "rpc_version" => get_in(descriptor, ["protocol", "rpc_version"])
    }
  end

  defp solve_on!(endpoint, job_id) do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [endpoint],
      failure_cooldown_ms: 30_000,
      failure_cooldown_max_ms: 30_000
    )

    AgentPool.reload()
    solve!(job_id)
  end

  defp solve_with_fleet!(job_id) do
    high = endpoint(@high_id, fetch_port!("KYUUBIKI_QUAL_HIGH_PORT"), @high_capacity)
    low = endpoint(@low_id, fetch_port!("KYUUBIKI_QUAL_LOW_PORT"), @low_capacity)

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [high, low],
      failure_cooldown_ms: 30_000,
      failure_cooldown_max_ms: 30_000
    )

    solve!(job_id)
  end

  defp solve!(job_id) do
    clear_events()

    {:ok, result, endpoint} =
      AgentClient.request_with_agent("solve_bar_1d", @solver_params, event_handler(),
        job_id: job_id,
        request_timeout_ms: 15_000
      )

    events = events()
    assert_result!(result)

    %{
      "job_id" => job_id,
      "selected_agent_id" => endpoint.id,
      "max_stress" => result["max_stress"],
      "tip_displacement" => result["tip_displacement"],
      "scheduler_events" =>
        events
        |> Enum.filter(&is_map(&1["scheduler"]))
        |> Enum.map(& &1["scheduler"]),
      "recovery" =>
        events
        |> Enum.find_value(fn event -> event["recovery"] end)
        |> sanitize_recovery()
    }
  end

  defp assert_result!(result) do
    stress = result["max_stress"]
    displacement = result["tip_displacement"]

    unless is_number(stress) and is_number(displacement) and abs(stress - 10.0) <= 1.0e-9 and
             abs(displacement - 0.01) <= 1.0e-12 do
      raise "fleet solver result failed the closed-form reference"
    end
  end

  defp fleet_health(high) do
    failed = Enum.find(AgentPool.endpoints(), &(&1.id == high.id)) || %{}

    %{
      "cooling_down_count" => AgentPool.deployment_info().cooling_down_count,
      "failed_agent_cooling_down" => Map.get(failed, :cooldown_remaining_ms, 0) > 0,
      "failed_agent_failure_count" => Map.get(failed, :consecutive_failures, 0)
    }
  end

  defp sanitize_recovery(nil), do: nil

  defp sanitize_recovery(recovery) do
    Map.take(recovery, [
      :failure_stage,
      :reason_code,
      :process_loss,
      :retry_safety,
      :retryable,
      :remaining_agent_count,
      :safe_to_continue_other_tasks,
      :next_action,
      "failure_stage",
      "reason_code",
      "process_loss",
      "retry_safety",
      "retryable",
      "remaining_agent_count",
      "safe_to_continue_other_tasks",
      "next_action"
    ])
  end

  defp signal_ready_for_restart! do
    path = fetch_env!("KYUUBIKI_QUAL_READY_PATH")
    File.write!(path, "ready\n")
  end

  defp wait_for_restart_release! do
    path = fetch_env!("KYUUBIKI_QUAL_RELEASE_PATH")
    deadline = System.monotonic_time(:millisecond) + 60_000
    wait_for_file!(path, deadline)
  end

  defp wait_for_file!(path, deadline) do
    cond do
      File.regular?(path) -> :ok
      System.monotonic_time(:millisecond) >= deadline -> raise "Agent restart handshake timed out"
      true -> Process.sleep(50) && wait_for_file!(path, deadline)
    end
  end

  defp event_handler do
    fn event ->
      Process.put(:fleet_scheduling_events, [event | Process.get(:fleet_scheduling_events, [])])
    end
  end

  defp events, do: Process.get(:fleet_scheduling_events, []) |> Enum.reverse()
  defp clear_events, do: Process.delete(:fleet_scheduling_events)

  defp write_report!(report) do
    output = fetch_env!("KYUUBIKI_QUAL_REPORT_PATH")
    File.write!(output, Jason.encode_to_iodata!(report, pretty: true))
    report
  end

  defp endpoint(id, port, capacity),
    do: %{id: id, host: "127.0.0.1", port: port, capacity: capacity}

  defp fetch_port!(name) do
    case Integer.parse(fetch_env!(name)) do
      {port, ""} when port in 1..65_535 -> port
      _ -> raise "#{name} must be a valid TCP port"
    end
  end

  defp fetch_env!(name), do: System.fetch_env!(name)
end
