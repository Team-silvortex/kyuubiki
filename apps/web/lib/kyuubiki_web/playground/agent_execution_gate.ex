defmodule KyuubikiWeb.Playground.AgentExecutionGate do
  @moduledoc """
  Applies one capacity contract to static, manifest, and registry-discovered agents.

  Requests wait in Orchestra instead of opening unbounded solver connections. A caller
  owns its lease until it explicitly releases it or the caller process exits.
  """

  use GenServer

  @default_queue_timeout_ms 120_000

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, %{}, name: __MODULE__)
  end

  def acquire(endpoints, lease_id, timeout_ms \\ @default_queue_timeout_ms)
      when is_list(endpoints) and is_binary(lease_id) and is_integer(timeout_ms) and
             timeout_ms > 0 do
    call_timeout = timeout_ms + 2_000
    GenServer.call(__MODULE__, {:acquire, self(), endpoints, lease_id, timeout_ms}, call_timeout)
  end

  def release(lease_id) when is_binary(lease_id) do
    GenServer.call(__MODULE__, {:release, lease_id})
  end

  def snapshot(endpoints \\ []) when is_list(endpoints) do
    GenServer.call(__MODULE__, {:snapshot, endpoints})
  end

  @impl true
  def init(_opts) do
    {:ok, %{leases: %{}, waiters: [], seen_endpoints: %{}}}
  end

  @impl true
  def handle_call({:acquire, pid, endpoints, lease_id, timeout_ms}, from, state) do
    endpoints = normalize_endpoints(endpoints)
    state = remember_endpoints(state, endpoints)

    cond do
      endpoints == [] ->
        {:reply, {:error, :no_agent_candidates}, state}

      Map.has_key?(state.leases, lease_id) ->
        lease = Map.fetch!(state.leases, lease_id)
        {:reply, {:ok, lease.endpoint, queue_metadata(0, 0)}, state}

      true ->
        case available_endpoint(endpoints, state.leases) do
          nil -> queue_waiter(state, from, pid, endpoints, lease_id, timeout_ms)
          endpoint -> grant_immediately(state, from, pid, endpoint, lease_id)
        end
    end
  end

  def handle_call({:release, lease_id}, _from, state) do
    state = release_lease(state, lease_id) |> dispatch_waiters()
    {:reply, :ok, state}
  end

  def handle_call({:snapshot, endpoints}, _from, state) do
    state = refresh_snapshot_endpoints(state, normalize_endpoints(endpoints))
    capacities = Map.new(state.seen_endpoints, fn {id, endpoint} -> {id, capacity(endpoint)} end)

    {:reply,
     %{
       active_lease_count: map_size(state.leases),
       queued_request_count: length(state.waiters),
       known_endpoint_count: map_size(state.seen_endpoints),
       capacity_slots: capacities |> Map.values() |> Enum.sum(),
       active_by_endpoint: active_counts(state.leases),
       capacity_by_endpoint: capacities
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

  defp grant_immediately(state, _from, pid, endpoint, lease_id) do
    monitor_ref = Process.monitor(pid)
    lease = lease(endpoint, lease_id, pid, monitor_ref)
    next_state = put_in(state, [:leases, lease_id], lease)
    {:reply, {:ok, endpoint, queue_metadata(0, 0)}, next_state}
  end

  defp queue_waiter(state, from, pid, endpoints, lease_id, timeout_ms) do
    monitor_ref = Process.monitor(pid)
    timer_ref = Process.send_after(self(), {:queue_timeout, lease_id}, timeout_ms)
    position = length(state.waiters) + 1

    waiter = %{
      from: from,
      pid: pid,
      endpoints: endpoints,
      lease_id: lease_id,
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
        case available_endpoint(waiter.endpoints, acc.leases) do
          nil ->
            {acc, pending ++ [waiter]}

          endpoint ->
            Process.cancel_timer(waiter.timer_ref)
            waited_ms = System.monotonic_time(:millisecond) - waiter.enqueued_at_ms
            lease = lease(endpoint, waiter.lease_id, waiter.pid, waiter.monitor_ref)
            GenServer.reply(waiter.from, {:ok, endpoint, queue_metadata(waited_ms, 0)})
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

  defp available_endpoint(endpoints, leases) do
    counts = active_counts(leases)
    Enum.find(endpoints, &(Map.get(counts, &1.id, 0) < capacity(&1)))
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

  defp lease(endpoint, lease_id, pid, monitor_ref) do
    %{endpoint: endpoint, lease_id: lease_id, pid: pid, monitor_ref: monitor_ref}
  end

  defp queue_metadata(waited_ms, queue_position) do
    %{waited_ms: max(waited_ms, 0), queue_position: queue_position}
  end

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
