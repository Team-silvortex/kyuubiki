defmodule KyuubikiWeb.Orchestra.LeaseMemoryBackend do
  @moduledoc false

  use Agent

  def start_link(_opts), do: Agent.start_link(fn -> %{} end, name: __MODULE__)

  def acquire(lease_name, owner_instance_id, ttl_ms)
      when is_binary(lease_name) and lease_name != "" and is_binary(owner_instance_id) and
             owner_instance_id != "" and is_integer(ttl_ms) and ttl_ms > 0 do
    Agent.get_and_update(__MODULE__, fn leases ->
      now = now_ms()

      case Map.get(leases, lease_name) do
        nil ->
          lease = new_lease(lease_name, owner_instance_id, 1, now + ttl_ms)
          {{:ok, lease}, Map.put(leases, lease_name, lease)}

        %{owner_instance_id: ^owner_instance_id, expires_at_ms: expires_at_ms} = current
        when expires_at_ms > now ->
          lease = %{current | expires_at_ms: now + ttl_ms}
          {{:ok, lease}, Map.put(leases, lease_name, lease)}

        %{expires_at_ms: expires_at_ms} = current when expires_at_ms <= now ->
          lease =
            new_lease(
              lease_name,
              owner_instance_id,
              current.fencing_token + 1,
              now + ttl_ms
            )

          {{:ok, lease}, Map.put(leases, lease_name, lease)}

        current ->
          {{:error, {:lease_held, current}}, leases}
      end
    end)
  end

  def acquire(_lease_name, _owner_instance_id, _ttl_ms), do: {:error, :invalid_lease_request}

  def renew(
        %{
          lease_name: lease_name,
          owner_instance_id: owner_instance_id,
          fencing_token: fencing_token
        },
        ttl_ms
      )
      when is_binary(lease_name) and lease_name != "" and is_binary(owner_instance_id) and
             owner_instance_id != "" and is_integer(fencing_token) and fencing_token > 0 and
             is_integer(ttl_ms) and ttl_ms > 0 do
    Agent.get_and_update(__MODULE__, fn leases ->
      now = now_ms()

      case Map.get(leases, lease_name) do
        %{owner_instance_id: owner, fencing_token: fencing, expires_at_ms: expires} = current
        when owner == owner_instance_id and fencing == fencing_token and
               expires > now ->
          renewed = %{current | expires_at_ms: now + ttl_ms}
          {{:ok, renewed}, Map.put(leases, lease_name, renewed)}

        _ ->
          {{:error, :orchestra_lease_lost}, leases}
      end
    end)
  end

  def renew(_token, _ttl_ms), do: {:error, :invalid_lease_request}

  def with_lease(
        %{
          lease_name: lease_name,
          owner_instance_id: owner_instance_id,
          fencing_token: fencing_token
        },
        callback
      )
      when is_binary(lease_name) and lease_name != "" and is_binary(owner_instance_id) and
             owner_instance_id != "" and is_integer(fencing_token) and fencing_token > 0 and
             is_function(callback, 0) do
    result =
      Agent.get(
        __MODULE__,
        fn leases ->
          case Map.get(leases, lease_name) do
            %{owner_instance_id: owner, fencing_token: fencing, expires_at_ms: expires}
            when owner == owner_instance_id and fencing == fencing_token ->
              if expires > now_ms(), do: invoke(callback), else: {:error, :orchestra_lease_lost}

            _ ->
              {:error, :orchestra_lease_lost}
          end
        end,
        :infinity
      )

    restore_callback_result(result)
  end

  def with_lease(_token, _callback), do: {:error, :invalid_lease_request}

  def release(%{
        lease_name: lease_name,
        owner_instance_id: owner_instance_id,
        fencing_token: fencing_token
      })
      when is_binary(lease_name) and lease_name != "" and is_binary(owner_instance_id) and
             owner_instance_id != "" and is_integer(fencing_token) and fencing_token > 0 do
    Agent.get_and_update(__MODULE__, fn leases ->
      case Map.get(leases, lease_name) do
        %{owner_instance_id: owner, fencing_token: fencing}
        when owner == owner_instance_id and fencing == fencing_token ->
          expired = %{Map.fetch!(leases, lease_name) | expires_at_ms: now_ms()}
          {:ok, Map.put(leases, lease_name, expired)}

        _ ->
          {{:error, :orchestra_lease_lost}, leases}
      end
    end)
  end

  def release(_token), do: {:error, :invalid_lease_request}

  def current(lease_name) when is_binary(lease_name) do
    Agent.get(__MODULE__, fn leases ->
      case Map.get(leases, lease_name) do
        nil -> :error
        lease -> {:ok, lease}
      end
    end)
  end

  defp invoke(callback) do
    {:lease_callback_result, callback.()}
  catch
    kind, reason -> {:lease_callback_failure, kind, reason, __STACKTRACE__}
  end

  defp restore_callback_result({:lease_callback_result, result}), do: result

  defp restore_callback_result({:lease_callback_failure, kind, reason, stacktrace}),
    do: :erlang.raise(kind, reason, stacktrace)

  defp restore_callback_result(result), do: result

  defp new_lease(lease_name, owner_instance_id, fencing_token, expires_at_ms) do
    %{
      lease_name: lease_name,
      owner_instance_id: owner_instance_id,
      fencing_token: fencing_token,
      expires_at_ms: expires_at_ms
    }
  end

  defp now_ms, do: System.system_time(:millisecond)
end
