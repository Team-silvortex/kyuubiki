defmodule KyuubikiWeb.Orchestra.WorkflowRecoveryOwnership do
  @moduledoc false

  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator

  def acquire(state) do
    case LeaseStore.acquire(state.lease_name, state.instance_id, state.lease_ttl_ms) do
      {:ok, lease} ->
        send(self(), :recover)

        state
        |> Map.merge(%{
          lease: lease,
          lease_status: :owner,
          lease_holder: nil,
          last_lease_error: nil
        })
        |> schedule()

      {:error, {:lease_held, holder}} ->
        state
        |> Map.merge(%{
          lease: nil,
          lease_status: :standby,
          lease_holder: holder,
          last_lease_error: nil
        })
        |> schedule()

      {:error, reason} ->
        state
        |> Map.merge(%{
          lease: nil,
          lease_status: :standby,
          lease_holder: nil,
          last_lease_error: reason
        })
        |> schedule()
    end
  end

  def schedule(state) do
    if is_reference(state.lease_timer_ref), do: Process.cancel_timer(state.lease_timer_ref)

    {message, delay} =
      case state.lease do
        %{fencing_token: fencing_token, expires_at_ms: expires_at_ms} ->
          {{:renew_lease, fencing_token, expires_at_ms}, state.lease_heartbeat_ms}

        nil ->
          {:acquire_lease, state.lease_retry_ms}
      end

    %{state | lease_timer_ref: Process.send_after(self(), message, delay)}
  end

  def guarded_write(state, callback) do
    if owner?(state),
      do: LeaseStore.with_lease(state.lease, callback),
      else: {:error, :orchestra_standby}
  end

  def after_write(state, {:error, reason})
      when reason in [:orchestra_lease_lost, :orchestra_lease_store_unavailable],
      do: lose(state, reason)

  def after_write(state, _result), do: state

  def lose(state, reason) do
    Enum.each(state.jobs, fn {_job_id, %{pid: pid, ref: ref}} ->
      Process.demonitor(ref, [:flush])
      if Process.alive?(pid), do: Process.exit(pid, :shutdown)
    end)

    state
    |> Map.merge(%{
      lease: nil,
      lease_status: :standby,
      lease_holder: nil,
      last_lease_error: reason,
      refs: %{},
      jobs: %{},
      progress: %{}
    })
    |> schedule()
  end

  def snapshot(state) do
    visible = state.lease || state.lease_holder

    %{
      "status" => Atom.to_string(state.lease_status),
      "lease_name" => state.lease_name,
      "owner_instance_id" => visible && visible.owner_instance_id,
      "fencing_token" => visible && visible.fencing_token,
      "expires_at_ms" => visible && visible.expires_at_ms,
      "last_error" => state.last_lease_error && format_reason(state.last_lease_error)
    }
  end

  def standby_summary(state) do
    %{
      "status" => "standby",
      "lease_name" => state.lease_name,
      "active_jobs" => 0,
      "recovered" => 0,
      "blocked" => 0,
      "skipped" => 0
    }
  end

  def owner?(%{lease_status: :owner, lease: lease}) when is_map(lease), do: true
  def owner?(_state), do: false

  def config, do: Application.get_env(:kyuubiki_web, WorkflowRecoveryCoordinator, [])

  def positive_interval(config, key, default) do
    case Keyword.get(config, key, default) do
      value when is_integer(value) and value > 0 -> value
      _ -> default
    end
  end

  def graceful_shutdown?(reason),
    do: reason in [:normal, :shutdown] or match?({:shutdown, _}, reason)

  defp format_reason(reason) when is_atom(reason), do: Atom.to_string(reason)
  defp format_reason(reason), do: inspect(reason)
end
