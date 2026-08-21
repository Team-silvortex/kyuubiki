defmodule KyuubikiWeb.Orchestra.LeaseStore do
  @moduledoc """
  Compact, database-backed ownership lease used to fence active Orchestra instances.

  Lease ownership is independent from workflow envelopes so heartbeats remain one small
  write per Orchestra rather than one write per active workflow.
  """

  alias KyuubikiWeb.Storage

  @identity_key {__MODULE__, :instance_id}

  @type token :: %{
          lease_name: String.t(),
          owner_instance_id: String.t(),
          fencing_token: pos_integer(),
          expires_at_ms: integer()
        }

  @spec acquire(String.t(), String.t(), pos_integer()) ::
          {:ok, token()} | {:error, {:lease_held, token()}} | {:error, term()}
  def acquire(lease_name, owner_instance_id, ttl_ms),
    do: backend().acquire(lease_name, owner_instance_id, ttl_ms)

  @spec renew(token(), pos_integer()) :: {:ok, token()} | {:error, term()}
  def renew(token, ttl_ms), do: backend().renew(token, ttl_ms)

  @spec with_lease(token(), (-> result)) :: result | {:error, term()} when result: term()
  def with_lease(token, callback), do: backend().with_lease(token, callback)

  @spec release(token()) :: :ok | {:error, term()}
  def release(token), do: backend().release(token)

  @spec current(String.t()) :: {:ok, token()} | :error | {:error, term()}
  def current(lease_name), do: backend().current(lease_name)

  @doc "Returns a stable identity for the lifetime of this BEAM instance."
  @spec instance_id() :: String.t()
  def instance_id do
    case configured_instance_id() do
      nil -> generated_instance_id()
      value -> value
    end
  end

  defp configured_instance_id do
    :kyuubiki_web
    |> Application.get_env(KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, [])
    |> Keyword.get(:instance_id)
    |> case do
      value when is_binary(value) and value != "" -> value
      _ -> nil
    end
  end

  defp generated_instance_id do
    case :persistent_term.get(@identity_key, :missing) do
      :missing ->
        identity = "orch-instance:" <> random_id()
        :persistent_term.put(@identity_key, identity)
        identity

      identity ->
        identity
    end
  end

  defp random_id, do: :crypto.strong_rand_bytes(16) |> Base.url_encode64(padding: false)

  defp backend do
    if Storage.sql?() do
      KyuubikiWeb.Orchestra.LeaseSqlBackend
    else
      KyuubikiWeb.Orchestra.LeaseMemoryBackend
    end
  end
end
