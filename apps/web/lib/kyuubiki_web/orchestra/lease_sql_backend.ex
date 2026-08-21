defmodule KyuubikiWeb.Orchestra.LeaseSqlBackend do
  @moduledoc false

  alias Ecto.Adapters.SQL
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator
  alias KyuubikiWeb.Storage

  def acquire(lease_name, owner_instance_id, ttl_ms)
      when is_binary(lease_name) and lease_name != "" and is_binary(owner_instance_id) and
             owner_instance_id != "" and is_integer(ttl_ms) and ttl_ms > 0 do
    case query(acquire_sql(), [lease_name, owner_instance_id, ttl_ms]) do
      {:ok, %{rows: [row]}} -> {:ok, decode(row)}
      {:ok, %{rows: []}} -> held_lease(lease_name)
      {:error, _reason} -> {:error, :orchestra_lease_store_unavailable}
    end
  end

  def acquire(_lease_name, _owner_instance_id, _ttl_ms), do: {:error, :invalid_lease_request}

  def renew(
        %{lease_name: lease_name, owner_instance_id: owner, fencing_token: fencing} = token,
        ttl_ms
      )
      when is_binary(lease_name) and lease_name != "" and is_binary(owner) and owner != "" and
             is_integer(fencing) and fencing > 0 and is_integer(ttl_ms) and ttl_ms > 0 do
    params = renew_params(token, ttl_ms)

    case query(renew_sql(), params) do
      {:ok, %{rows: [row]}} -> {:ok, decode(row)}
      {:ok, %{rows: []}} -> {:error, :orchestra_lease_lost}
      {:error, _reason} -> {:error, :orchestra_lease_store_unavailable}
    end
  end

  def renew(_token, _ttl_ms), do: {:error, :invalid_lease_request}

  def with_lease(
        %{lease_name: lease_name, owner_instance_id: owner, fencing_token: fencing} = token,
        callback
      )
      when is_binary(lease_name) and lease_name != "" and is_binary(owner) and owner != "" and
             is_integer(fencing) and fencing > 0 and is_function(callback, 0) do
    transaction = fn ->
      params = [token[:lease_name], token[:owner_instance_id], token[:fencing_token]]

      case query(owned_sql(), params) do
        {:ok, %{rows: [_row]}} -> callback.()
        {:ok, %{rows: []}} -> rollback(:orchestra_lease_lost)
        {:error, _reason} -> rollback(:orchestra_lease_store_unavailable)
      end
    end

    case safe_transaction(transaction) do
      {:ok, result} -> result
      {:error, reason} -> {:error, reason}
    end
  end

  def with_lease(_token, _callback), do: {:error, :invalid_lease_request}

  def release(%{lease_name: lease_name, owner_instance_id: owner, fencing_token: fencing} = token)
      when is_binary(lease_name) and lease_name != "" and is_binary(owner) and owner != "" and
             is_integer(fencing) and fencing > 0 do
    params = [token[:lease_name], token[:owner_instance_id], token[:fencing_token]]

    case query(release_sql(), params) do
      {:ok, %{rows: [_row]}} -> :ok
      {:ok, %{rows: []}} -> {:error, :orchestra_lease_lost}
      {:error, _reason} -> {:error, :orchestra_lease_store_unavailable}
    end
  end

  def release(_token), do: {:error, :invalid_lease_request}

  def current(lease_name) when is_binary(lease_name) do
    case query(current_sql(), [lease_name]) do
      {:ok, %{rows: [row]}} -> {:ok, decode(row)}
      {:ok, %{rows: []}} -> :error
      {:error, _reason} -> {:error, :orchestra_lease_store_unavailable}
    end
  end

  defp held_lease(lease_name) do
    case current(lease_name) do
      {:ok, lease} -> {:error, {:lease_held, lease}}
      _ -> {:error, :orchestra_lease_store_unavailable}
    end
  end

  defp safe_transaction(callback) do
    apply(repo(), :transaction, [callback, transaction_options()])
  rescue
    _error -> {:error, :orchestra_lease_store_unavailable}
  catch
    :exit, _reason -> {:error, :orchestra_lease_store_unavailable}
  end

  defp query(sql, params) do
    SQL.query(repo(), sql, params, timeout: query_timeout_ms())
  rescue
    _error -> {:error, :lease_query_failed}
  catch
    :exit, _reason -> {:error, :lease_query_failed}
  end

  defp repo, do: Storage.repo_module!()
  defp rollback(reason), do: apply(repo(), :rollback, [reason])

  defp transaction_options do
    options = [timeout: query_timeout_ms()]
    if Storage.sqlite?(), do: Keyword.put(options, :mode, :immediate), else: options
  end

  defp query_timeout_ms do
    value =
      :kyuubiki_web
      |> Application.get_env(WorkflowRecoveryCoordinator, [])
      |> Keyword.get(:lease_query_timeout_ms, 2_000)

    if is_integer(value) and value > 0, do: value, else: 2_000
  end

  defp renew_params(token, ttl_ms) do
    if Storage.postgres?() do
      [token[:lease_name], token[:owner_instance_id], token[:fencing_token], ttl_ms]
    else
      [ttl_ms, token[:lease_name], token[:owner_instance_id], token[:fencing_token]]
    end
  end

  defp decode([lease_name, owner_instance_id, fencing_token, expires_at_ms]) do
    %{
      lease_name: lease_name,
      owner_instance_id: owner_instance_id,
      fencing_token: fencing_token,
      expires_at_ms: expires_at_ms
    }
  end

  defp acquire_sql do
    if Storage.postgres?(), do: postgres_acquire_sql(), else: sqlite_acquire_sql()
  end

  defp renew_sql do
    if Storage.postgres?(), do: postgres_renew_sql(), else: sqlite_renew_sql()
  end

  defp owned_sql do
    if Storage.postgres?(), do: postgres_owned_sql(), else: sqlite_owned_sql()
  end

  defp release_sql do
    if Storage.postgres?(), do: postgres_release_sql(), else: sqlite_release_sql()
  end

  defp current_sql do
    if Storage.postgres?(),
      do:
        "SELECT lease_name, owner_instance_id, fencing_token, expires_at_ms FROM kyuubiki_orchestra_leases WHERE lease_name = $1",
      else:
        "SELECT lease_name, owner_instance_id, fencing_token, expires_at_ms FROM kyuubiki_orchestra_leases WHERE lease_name = ?"
  end

  defp postgres_acquire_sql do
    """
    INSERT INTO kyuubiki_orchestra_leases AS current_lease
      (lease_name, owner_instance_id, fencing_token, expires_at_ms, inserted_at, updated_at)
    VALUES ($1, $2, 1, #{postgres_now_ms()} + $3, NOW(), NOW())
    ON CONFLICT (lease_name) DO UPDATE SET
      owner_instance_id = EXCLUDED.owner_instance_id,
      fencing_token = CASE
        WHEN current_lease.owner_instance_id = EXCLUDED.owner_instance_id
          AND current_lease.expires_at_ms > #{postgres_now_ms()}
          THEN current_lease.fencing_token
        ELSE current_lease.fencing_token + 1
      END,
      expires_at_ms = EXCLUDED.expires_at_ms,
      updated_at = NOW()
    WHERE current_lease.owner_instance_id = EXCLUDED.owner_instance_id
       OR current_lease.expires_at_ms <= #{postgres_now_ms()}
    RETURNING lease_name, owner_instance_id, fencing_token, expires_at_ms
    """
  end

  defp sqlite_acquire_sql do
    """
    INSERT INTO kyuubiki_orchestra_leases
      (lease_name, owner_instance_id, fencing_token, expires_at_ms, inserted_at, updated_at)
    VALUES (?, ?, 1, #{sqlite_now_ms()} + ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
    ON CONFLICT (lease_name) DO UPDATE SET
      owner_instance_id = excluded.owner_instance_id,
      fencing_token = CASE
        WHEN kyuubiki_orchestra_leases.owner_instance_id = excluded.owner_instance_id
          AND kyuubiki_orchestra_leases.expires_at_ms > #{sqlite_now_ms()}
          THEN kyuubiki_orchestra_leases.fencing_token
        ELSE kyuubiki_orchestra_leases.fencing_token + 1
      END,
      expires_at_ms = excluded.expires_at_ms,
      updated_at = CURRENT_TIMESTAMP
    WHERE kyuubiki_orchestra_leases.owner_instance_id = excluded.owner_instance_id
       OR kyuubiki_orchestra_leases.expires_at_ms <= #{sqlite_now_ms()}
    RETURNING lease_name, owner_instance_id, fencing_token, expires_at_ms
    """
  end

  defp postgres_renew_sql do
    """
    UPDATE kyuubiki_orchestra_leases
    SET expires_at_ms = #{postgres_now_ms()} + $4, updated_at = NOW()
    WHERE lease_name = $1 AND owner_instance_id = $2 AND fencing_token = $3
      AND expires_at_ms > #{postgres_now_ms()}
    RETURNING lease_name, owner_instance_id, fencing_token, expires_at_ms
    """
  end

  defp sqlite_renew_sql do
    """
    UPDATE kyuubiki_orchestra_leases
    SET expires_at_ms = #{sqlite_now_ms()} + ?, updated_at = CURRENT_TIMESTAMP
    WHERE lease_name = ? AND owner_instance_id = ? AND fencing_token = ?
      AND expires_at_ms > #{sqlite_now_ms()}
    RETURNING lease_name, owner_instance_id, fencing_token, expires_at_ms
    """
  end

  defp postgres_owned_sql do
    """
    SELECT lease_name FROM kyuubiki_orchestra_leases
    WHERE lease_name = $1 AND owner_instance_id = $2 AND fencing_token = $3
      AND expires_at_ms > #{postgres_now_ms()}
    FOR UPDATE
    """
  end

  defp sqlite_owned_sql do
    """
    SELECT lease_name FROM kyuubiki_orchestra_leases
    WHERE lease_name = ? AND owner_instance_id = ? AND fencing_token = ?
      AND expires_at_ms > #{sqlite_now_ms()}
    """
  end

  defp postgres_release_sql do
    """
    UPDATE kyuubiki_orchestra_leases
    SET expires_at_ms = #{postgres_now_ms()}, updated_at = NOW()
    WHERE lease_name = $1 AND owner_instance_id = $2 AND fencing_token = $3
    RETURNING lease_name
    """
  end

  defp sqlite_release_sql do
    """
    UPDATE kyuubiki_orchestra_leases
    SET expires_at_ms = #{sqlite_now_ms()}, updated_at = CURRENT_TIMESTAMP
    WHERE lease_name = ? AND owner_instance_id = ? AND fencing_token = ?
    RETURNING lease_name
    """
  end

  defp postgres_now_ms,
    do: "FLOOR(EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT"

  defp sqlite_now_ms,
    do: "CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)"
end
