defmodule KyuubikiWeb.Orchestra.WorkflowCancellationHandoff do
  @moduledoc false

  alias KyuubikiWeb.Orchestra.WorkflowCancellationDelivery, as: Delivery
  alias KyuubikiWeb.Orchestra.WorkflowCancellationStore, as: Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryOwnership, as: Ownership

  @registry KyuubikiWeb.Orchestra.WorkflowRunnerRegistry
  @max_jobs 128
  @batch_jobs 16
  @wait_ms 30_000

  def valid_job_id?(id), do: is_binary(id) and byte_size(id) in 1..1024

  def continuation_jobs(ids, visited) do
    {jobs, _seen, invalid?} =
      Enum.reduce_while(ids, {[], MapSet.new(), false}, fn id, {jobs, seen, invalid?} = acc ->
        cond do
          not valid_job_id?(id) ->
            {:cont, {jobs, seen, true}}

          MapSet.member?(visited, id) or MapSet.member?(seen, id) ->
            {:cont, acc}

          true ->
            seen = MapSet.put(seen, id)
            next = {[id | jobs], seen, invalid?}
            if MapSet.size(seen) > @max_jobs, do: {:halt, next}, else: {:cont, next}
        end
      end)

    # Keep one valid overflow sentinel and one invalid marker, never raw malformed input.
    Enum.reverse(jobs, if(invalid?, do: [nil], else: []))
  end

  def defer(state, holder, ids, authority, deadline) do
    cond do
      not current?(state, authority) -> state
      not is_pid(holder) or holder == self() or not is_list(ids) -> state
      not is_integer(deadline) or deadline <= now() -> error(state, :cancellation_handoff_expired)
      true -> enqueue(state, holder, ids, authority, deadline)
    end
  end

  defp enqueue(state, holder, ids, authority, deadline) do
    incoming = continuation_jobs(ids, MapSet.new())
    valid = Enum.filter(incoming, &valid_job_id?/1)

    # Timer delivery can lag behind admission of a still-valid new trigger.
    state =
      case state.cancellation_handoff do
        %{deadline: expiry, token: token} when valid != [] ->
          if expiry <= now(), do: expire(state, token), else: state

        _ ->
          state
      end

    pending = state.cancellation_handoff
    previous = if pending, do: pending.jobs, else: []
    deadline = min(deadline, if(pending, do: pending.deadline, else: now() + @wait_ms))
    jobs = Enum.uniq(previous ++ valid)

    diagnostic =
      cond do
        length(valid) > @max_jobs or length(jobs) > @max_jobs -> :cancellation_handoff_full
        length(valid) != length(incoming) -> :invalid_cancellation_handoff_jobs
        true -> state.cancellation_handoff_error
      end

    state = error(state, diagnostic)
    retained = Enum.take(jobs, @max_jobs)
    retained_ids = MapSet.new(retained)

    # Rejected overflow must not change the deadline or monitor of accepted work.
    if Enum.any?(valid, &MapSet.member?(retained_ids, &1)),
      do: arm(state, holder, retained, authority, deadline),
      else: state
  end

  def down(state, ref) do
    case state.cancellation_handoff do
      %{ref: ^ref} = pending when is_reference(ref) ->
        {:handled, arm(state, nil, pending.jobs, pending.authority, pending.deadline)}

      _ ->
        :unhandled
    end
  end

  def ready(state, token) do
    case state.cancellation_handoff do
      %{token: ^token, holder: nil} = pending ->
        cond do
          not current?(state, pending.authority) -> clear(state)
          now() >= pending.deadline -> expire(state, token)
          true -> launch_when_free(state, pending)
        end

      _ ->
        state
    end
  end

  def expire(state, token) do
    case state.cancellation_handoff do
      %{token: ^token} -> state |> clear() |> error(:cancellation_handoff_expired)
      _ -> state
    end
  end

  def clear(state) do
    if pending = Map.get(state, :cancellation_handoff) do
      if pending.ref, do: Process.demonitor(pending.ref, [:flush])
      Process.cancel_timer(pending.expiry_timer)
      if pending.ready_timer, do: Process.cancel_timer(pending.ready_timer)
    end

    Map.put(state, :cancellation_handoff, nil)
  end

  def summary(state) do
    pending = state.cancellation_handoff

    %{
      "max_pending_jobs" => @max_jobs,
      "max_wait_ms" => @wait_ms,
      "batch_jobs" => @batch_jobs,
      "pending_jobs" => if(pending, do: length(pending.jobs), else: 0),
      "waiting_for_worker" => not is_nil(pending && pending.holder),
      "remaining_wait_ms" => if(pending, do: max(pending.deadline - now(), 0), else: 0),
      "last_error" =>
        state.cancellation_handoff_error && Atom.to_string(state.cancellation_handoff_error)
    }
  end

  defp arm(state, holder, jobs, authority, deadline) do
    state = clear(state)
    token = make_ref()

    pending = %{
      holder: holder,
      jobs: jobs,
      authority: authority,
      deadline: deadline,
      token: token,
      ref: if(holder, do: Process.monitor(holder)),
      expiry_timer:
        Process.send_after(
          self(),
          {:cancellation_handoff_expired, token},
          max(deadline - now(), 0)
        ),
      # Registry removal and process-monitor delivery need not arrive in the same order.
      ready_timer:
        if(is_nil(holder),
          do: Process.send_after(self(), {:cancellation_handoff_ready, token}, 10)
        )
    }

    %{state | cancellation_handoff: pending}
  end

  defp launch_when_free(state, pending) do
    case lookup_slot() do
      {:ok, [{holder, _}]} ->
        arm(state, holder, pending.jobs, pending.authority, pending.deadline)

      {:ok, []} ->
        launch(state, pending)

      {:error, _} ->
        state |> clear() |> error(:workflow_runner_registry_unavailable)
    end
  end

  defp lookup_slot do
    {:ok, Registry.lookup(@registry, {Delivery, :sweep})}
  rescue
    # A prior availability check cannot protect this lookup from Registry shutdown.
    _ in ArgumentError -> {:error, :workflow_runner_registry_unavailable}
  catch
    :exit, _ -> {:error, :workflow_runner_registry_unavailable}
  end

  defp launch(state, pending) do
    {batch, remaining} = Enum.split(pending.jobs, @batch_jobs)
    state = clear(state)

    case Delivery.kick(batch, pending.authority, pending.deadline) do
      {:ok, pid} when remaining != [] ->
        arm(state, pid, remaining, pending.authority, pending.deadline)

      {:ok, _pid} ->
        state

      {:error, _} ->
        error(state, :cancellation_worker_unavailable)
    end
  end

  defp current?(state, authority),
    do: Ownership.owner?(state) and Store.authority(state) == authority

  defp error(state, reason), do: %{state | cancellation_handoff_error: reason}
  defp now, do: System.monotonic_time(:millisecond)
end
