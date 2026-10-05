defmodule KyuubikiWeb.Playground.AgentExecutionGate do
  @moduledoc """
  Applies one capacity contract to static, manifest, and registry-discovered agents.

  Requests wait in Orchestra instead of opening unbounded solver connections. A caller
  owns its lease until it explicitly releases it or the caller process exits.
  Job ownership is captured with that lease. Dispatch authorization marks the
  transport boundary, not proof of Agent acceptance or a terminal computation.
  """

  use GenServer

  @default_queue_timeout_ms 120_000
  @max_lease_id_bytes 128
  @selection_policy "least_utilized_capacity_v1"

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, %{}, name: __MODULE__)
  end

  def acquire(endpoints, lease_id, timeout_ms \\ @default_queue_timeout_ms, job_id \\ nil)

  def acquire(endpoints, lease_id, timeout_ms, job_id)
      when is_list(endpoints) and is_binary(lease_id) and is_integer(timeout_ms) and
             timeout_ms > 0 and byte_size(lease_id) > 0 and
             byte_size(lease_id) <= @max_lease_id_bytes do
    if is_nil(job_id) or valid_job_id?(job_id) do
      GenServer.call(
        __MODULE__,
        {:acquire, self(), endpoints, lease_id, timeout_ms, job_id},
        timeout_ms + 2_000
      )
    else
      {:error, :invalid_execution_job_id}
    end
  end

  def acquire(_endpoints, _lease_id, _timeout_ms, _job_id), do: {:error, :invalid_execution_lease}

  def authorize_dispatch(lease_id, starting? \\ true) do
    GenServer.call(__MODULE__, {:authorize_dispatch, self(), lease_id, starting?})
  end

  def cancel_job_targets(job_id) do
    if valid_job_id?(job_id),
      do: GenServer.call(__MODULE__, {:cancel_job_targets, job_id}),
      else: {:error, :invalid_execution_job_id}
  end

  def valid_job_id?(job_id) when is_binary(job_id) do
    byte_size(job_id) in 1..256 and String.valid?(job_id) and
      not Regex.match?(~r/\p{Cc}/u, job_id)
  end

  def valid_job_id?(_job_id), do: false

  def release(lease_id)
      when is_binary(lease_id) and byte_size(lease_id) > 0 and
             byte_size(lease_id) <= @max_lease_id_bytes do
    GenServer.call(__MODULE__, {:release, self(), lease_id})
  end

  def release(_lease_id), do: {:error, :invalid_execution_lease}

  def snapshot(endpoints \\ []) when is_list(endpoints) do
    GenServer.call(__MODULE__, {:snapshot, endpoints})
  end

  @impl true
  def init(_opts) do
    {:ok, %{leases: %{}, waiters: [], seen_endpoints: %{}}}
  end

  @impl true
  def handle_call({:acquire, pid, endpoints, lease_id, timeout_ms, job_id}, from, state) do
    endpoints = normalize_endpoints(endpoints)
    state = remember_endpoints(state, endpoints)

    cond do
      endpoints == [] ->
        {:reply, {:error, :no_agent_candidates}, state}

      lease_id_in_use?(state, lease_id) ->
        {:reply, {:error, {:duplicate_execution_lease, lease_id}}, state}

      true ->
        case select_endpoint(endpoints, state.leases) do
          nil -> queue_waiter(state, from, pid, endpoints, lease_id, timeout_ms, job_id)
          selection -> grant_immediately(state, from, pid, selection, lease_id, job_id)
        end
    end
  end

  def handle_call({:authorize_dispatch, pid, lease_id, starting?}, _from, state) do
    case Map.get(state.leases, lease_id) do
      %{pid: ^pid, cancel_requested: true} ->
        {:reply, cancelled(), state}

      %{pid: ^pid} ->
        next =
          if starting?, do: put_in(state, [:leases, lease_id, :dispatched], true), else: state

        {:reply, :ok, next}

      _ ->
        {:reply, {:error, {:execution_lease_not_owned, lease_id}}, state}
    end
  end

  def handle_call({:cancel_job_targets, job_id}, _from, state) do
    {removed, remaining} = Enum.split_with(state.waiters, &(&1.job_id == job_id))

    Enum.each(removed, fn waiter ->
      Process.cancel_timer(waiter.timer_ref)
      Process.demonitor(waiter.monitor_ref, [:flush])
      GenServer.reply(waiter.from, cancelled())
    end)

    matching = state.leases |> Map.values() |> Enum.filter(&(&1.job_id == job_id))

    targets = %{
      endpoints:
        matching
        |> Enum.filter(& &1.dispatched)
        |> Enum.map(& &1.endpoint)
        |> Enum.uniq_by(& &1.id)
        |> Enum.sort_by(& &1.id),
      queued_cancelled_count: length(removed),
      reserved_cancelled_count: Enum.count(matching, &(not &1.dispatched))
    }

    leases =
      Map.new(state.leases, fn {id, lease} ->
        {id, if(lease.job_id == job_id, do: %{lease | cancel_requested: true}, else: lease)}
      end)

    {:reply, {:ok, targets}, %{state | leases: leases, waiters: renumber_waiters(remaining)}}
  end

  def handle_call({:release, pid, lease_id}, _from, state) do
    case Map.get(state.leases, lease_id) do
      nil ->
        {:reply, :ok, state}

      %{pid: ^pid} ->
        state = release_lease(state, lease_id) |> dispatch_waiters()
        {:reply, :ok, state}

      _lease ->
        {:reply, {:error, {:execution_lease_not_owned, lease_id}}, state}
    end
  end

  def handle_call({:snapshot, endpoints}, _from, state) do
    state = refresh_snapshot_endpoints(state, normalize_endpoints(endpoints))
    capacities = Map.new(state.seen_endpoints, fn {id, endpoint} -> {id, capacity(endpoint)} end)
    active = active_counts(state.leases)

    utilizations =
      Map.new(capacities, fn {id, slots} ->
        {id, utilization(Map.get(active, id, 0), slots)}
      end)

    {:reply,
     %{
       selection_policy: @selection_policy,
       active_lease_count: map_size(state.leases),
       queued_request_count: length(state.waiters),
       known_endpoint_count: map_size(state.seen_endpoints),
       capacity_slots: capacities |> Map.values() |> Enum.sum(),
       saturated_endpoint_count:
         Enum.count(capacities, fn {id, slots} -> Map.get(active, id, 0) >= slots end),
       active_by_endpoint: active,
       capacity_by_endpoint: capacities,
       utilization_by_endpoint: utilizations
     }, state}
  end

  @impl true
  def handle_info({:queue_timeout, lease_id}, state) do
    {waiter, remaining} = pop_waiter(state.waiters, lease_id)

    case waiter do
      nil ->
        {:noreply, state}

      waiter ->
        Process.demonitor(waiter.monitor_ref, [:flush])

        GenServer.reply(
          waiter.from,
          {:error,
           {:agent_queue_timeout,
            %{
              timeout_ms: waiter.timeout_ms,
              queue_position: waiter.queue_position,
              candidate_agent_ids: Enum.map(waiter.endpoints, & &1.id)
            }}}
        )

        {:noreply, %{state | waiters: remaining}}
    end
  end

  def handle_info({:DOWN, monitor_ref, :process, _pid, _reason}, state) do
    state =
      case lease_id_for_monitor(state.leases, monitor_ref) do
        nil -> remove_waiter_by_monitor(state, monitor_ref)
        lease_id -> release_lease(state, lease_id)
      end

    {:noreply, dispatch_waiters(state)}
  end

  defp grant_immediately(state, _from, pid, {endpoint, scheduling}, lease_id, job_id) do
    monitor_ref = Process.monitor(pid)
    lease = lease(endpoint, lease_id, pid, monitor_ref, job_id)
    next_state = put_in(state, [:leases, lease_id], lease)
    {:reply, {:ok, endpoint, queue_metadata(0, 0, scheduling)}, next_state}
  end

  defp queue_waiter(state, from, pid, endpoints, lease_id, timeout_ms, job_id) do
    monitor_ref = Process.monitor(pid)
    timer_ref = Process.send_after(self(), {:queue_timeout, lease_id}, timeout_ms)
    position = length(state.waiters) + 1

    waiter = %{
      from: from,
      pid: pid,
      endpoints: endpoints,
      lease_id: lease_id,
      job_id: job_id,
      monitor_ref: monitor_ref,
      timer_ref: timer_ref,
      enqueued_at_ms: System.monotonic_time(:millisecond),
      timeout_ms: timeout_ms,
      queue_position: position
    }

    {:noreply, %{state | waiters: state.waiters ++ [waiter]}}
  end

  defp dispatch_waiters(%{waiters: []} = state), do: state

  defp dispatch_waiters(state) do
    {state, pending} =
      Enum.reduce(state.waiters, {%{state | waiters: []}, []}, fn waiter, {acc, pending} ->
        case select_endpoint(waiter.endpoints, acc.leases) do
          nil ->
            {acc, pending ++ [waiter]}

          {endpoint, scheduling} ->
            Process.cancel_timer(waiter.timer_ref)
            waited_ms = System.monotonic_time(:millisecond) - waiter.enqueued_at_ms

            lease =
              lease(endpoint, waiter.lease_id, waiter.pid, waiter.monitor_ref, waiter.job_id)

            GenServer.reply(
              waiter.from,
              {:ok, endpoint, queue_metadata(waited_ms, waiter.queue_position, scheduling)}
            )

            {put_in(acc, [:leases, waiter.lease_id], lease), pending}
        end
      end)

    %{state | waiters: renumber_waiters(pending)}
  end

  defp release_lease(state, lease_id) do
    case Map.pop(state.leases, lease_id) do
      {nil, _leases} ->
        state

      {lease, leases} ->
        Process.demonitor(lease.monitor_ref, [:flush])
        %{state | leases: leases}
    end
  end

  defp select_endpoint(endpoints, leases) do
    counts = active_counts(leases)

    endpoints
    |> Enum.with_index()
    |> Enum.filter(fn {endpoint, _index} ->
      Map.get(counts, endpoint.id, 0) < capacity(endpoint)
    end)
    |> case do
      [] ->
        nil

      candidates ->
        {endpoint, _index} =
          Enum.min_by(candidates, fn {candidate, index} ->
            active = Map.get(counts, candidate.id, 0)
            {routing_priority(candidate), utilization(active, capacity(candidate)), index}
          end)

        endpoint = Map.delete(endpoint, :_scheduler_priority)
        active = Map.get(counts, endpoint.id, 0)
        slots = capacity(endpoint)

        {endpoint,
         %{
           selection_policy: @selection_policy,
           selected_agent_id: endpoint.id,
           active_slots_before: active,
           active_slots_after: active + 1,
           capacity_slots: slots,
           utilization_before: utilization(active, slots),
           utilization_after: utilization(active + 1, slots)
         }}
    end
  end

  defp lease_id_in_use?(state, lease_id) do
    Map.has_key?(state.leases, lease_id) or
      Enum.any?(state.waiters, &(&1.lease_id == lease_id))
  end

  defp active_counts(leases) do
    Enum.reduce(leases, %{}, fn {_lease_id, lease}, acc ->
      Map.update(acc, lease.endpoint.id, 1, &(&1 + 1))
    end)
  end

  defp capacity(endpoint) do
    case Map.get(endpoint, :capacity) do
      value when is_integer(value) and value > 0 -> value
      _ -> 1
    end
  end

  defp lease(endpoint, lease_id, pid, monitor_ref, job_id) do
    %{
      endpoint: endpoint,
      lease_id: lease_id,
      pid: pid,
      monitor_ref: monitor_ref,
      job_id: job_id,
      cancel_requested: false,
      dispatched: false
    }
  end

  defp cancelled,
    do: {:error, {:rpc_error, "cancelled", "job cancelled before result publication"}}

  defp queue_metadata(waited_ms, queue_position, scheduling) do
    Map.merge(
      %{waited_ms: max(waited_ms, 0), queue_position: queue_position},
      scheduling
    )
  end

  defp utilization(_active, 0), do: 0.0
  defp utilization(active, slots), do: Float.round(active / slots, 6)

  defp routing_priority(%{_scheduler_priority: {availability, constraint, score, method}})
       when is_integer(availability) and is_integer(constraint) and is_integer(score) and
              is_integer(method),
       do: {availability, constraint, score, method}

  defp routing_priority(_endpoint), do: {0, 0, 0, 0}

  defp normalize_endpoints(endpoints) do
    endpoints
    |> Enum.filter(&(is_map(&1) and is_binary(Map.get(&1, :id))))
    |> Enum.uniq_by(& &1.id)
  end

  defp remember_endpoints(state, endpoints) do
    seen = Enum.reduce(endpoints, state.seen_endpoints, &Map.put(&2, &1.id, &1))
    %{state | seen_endpoints: seen}
  end

  defp refresh_snapshot_endpoints(state, []), do: state

  defp refresh_snapshot_endpoints(state, configured) do
    active = Enum.map(state.leases, fn {_lease_id, lease} -> lease.endpoint end)
    queued = Enum.flat_map(state.waiters, & &1.endpoints)
    endpoints = Enum.uniq_by(configured ++ active ++ queued, & &1.id)
    %{state | seen_endpoints: Map.new(endpoints, &{&1.id, &1})}
  end

  defp pop_waiter(waiters, lease_id) do
    case Enum.split_while(waiters, &(&1.lease_id != lease_id)) do
      {before, [waiter | after_waiter]} -> {waiter, before ++ after_waiter}
      {_before, []} -> {nil, waiters}
    end
  end

  defp remove_waiter_by_monitor(state, monitor_ref) do
    {removed, remaining} = Enum.split_with(state.waiters, &(&1.monitor_ref == monitor_ref))
    Enum.each(removed, &Process.cancel_timer(&1.timer_ref))
    %{state | waiters: renumber_waiters(remaining)}
  end

  defp lease_id_for_monitor(leases, monitor_ref) do
    Enum.find_value(leases, fn {lease_id, lease} ->
      if lease.monitor_ref == monitor_ref, do: lease_id
    end)
  end

  defp renumber_waiters(waiters) do
    waiters
    |> Enum.with_index(1)
    |> Enum.map(fn {waiter, position} -> %{waiter | queue_position: position} end)
  end
end
