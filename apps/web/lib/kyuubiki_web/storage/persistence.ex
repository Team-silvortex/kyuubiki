defmodule KyuubikiWeb.Persistence do
  @moduledoc false

  @default_dir Path.expand("../../../../tmp/data", __DIR__)
  @envelope_schema "kyuubiki.persistence-envelope/v1"
  @recovery_schema "kyuubiki.persistence-recovery-receipt/v1"

  def data_dir do
    System.get_env("KYUUBIKI_DATA_DIR", @default_dir)
  end

  def jobs_path do
    Path.join(data_dir(), "jobs.json")
  end

  def results_path do
    Path.join(data_dir(), "results.json")
  end

  def security_events_path do
    Path.join(data_dir(), "security_events.json")
  end

  def ensure_dir! do
    File.mkdir_p!(data_dir())
  end

  def write_json!(path, payload) do
    File.mkdir_p!(Path.dirname(path))
    ensure_current_generation_is_rotatable!(path)

    envelope = %{
      "schema_version" => @envelope_schema,
      "digest_algorithm" => "sha256",
      "payload_sha256" => payload_digest(payload),
      "payload" => payload
    }

    commit_atomic!(path, Jason.encode!(envelope))
  end

  def read_json(path, default) do
    case read_verified_generation(path) do
      {:ok, payload, _kind} -> payload
      {:error, :enoent} -> recover_missing_primary(path, default)
      {:error, reason} -> recover_or_quarantine(path, default, reason)
    end
  end

  def clear! do
    File.rm_rf!(data_dir())
    :ok
  end

  defp commit_atomic!(path, bytes) do
    next = sidecar(path, ".next")
    previous = sidecar(path, ".previous")
    File.rm(next)
    File.write!(next, bytes, [:sync])
    File.rm(previous)

    if File.exists?(path) do
      File.rename!(path, previous)
    end

    case File.rename(next, path) do
      :ok ->
        :ok

      {:error, reason} ->
        if not File.exists?(path) and File.exists?(previous) do
          File.rename(previous, path)
        end

        raise File.Error, reason: reason, action: "commit persistence envelope", path: path
    end
  end

  defp ensure_current_generation_is_rotatable!(path) do
    case read_verified_generation(path) do
      {:ok, _payload, _kind} ->
        :ok

      {:error, :enoent} ->
        :ok

      {:error, reason} ->
        raise "refusing to overwrite invalid persistence generation #{Path.basename(path)}: #{inspect(reason)}"
    end
  end

  defp read_verified_generation(path) do
    with {:ok, contents} <- File.read(path),
         {:ok, decoded} <- Jason.decode(contents) do
      decode_generation(decoded)
    else
      {:error, :enoent} -> {:error, :enoent}
      {:error, reason} -> {:error, reason}
    end
  end

  defp decode_generation(%{"schema_version" => @envelope_schema} = envelope) do
    with "sha256" <- envelope["digest_algorithm"],
         digest when is_binary(digest) <- envelope["payload_sha256"],
         true <- valid_digest?(digest),
         payload <- envelope["payload"],
         true <- payload_digest(payload) == digest do
      {:ok, payload, :verified_envelope}
    else
      _ -> {:error, :persistence_digest_mismatch}
    end
  end

  defp decode_generation(%{"schema_version" => schema} = decoded)
       when is_binary(schema) and schema != @envelope_schema do
    if String.starts_with?(schema, "kyuubiki.persistence-envelope/") do
      {:error, {:unsupported_persistence_envelope, schema}}
    else
      {:ok, decoded, :legacy_json}
    end
  end

  defp decode_generation(decoded), do: {:ok, decoded, :legacy_json}

  defp recover_or_quarantine(path, default, primary_reason) do
    previous = sidecar(path, ".previous")

    case read_verified_generation(previous) do
      {:ok, payload, generation_kind} ->
        quarantine_primary(path)
        File.cp!(previous, path)

        write_recovery_receipt(path, %{
          "status" => "recovered_previous_generation",
          "primary_error" => inspect(primary_reason),
          "previous_generation_kind" => to_string(generation_kind),
          "previous_generation_used" => true
        })

        payload

      {:error, previous_reason} ->
        quarantined = quarantine_primary(path)

        write_recovery_receipt(path, %{
          "status" => "quarantined_and_defaulted",
          "primary_error" => inspect(primary_reason),
          "previous_error" => inspect(previous_reason),
          "previous_generation_used" => false,
          "corrupt_copy_retained" => quarantined
        })

        default
    end
  end

  defp recover_missing_primary(path, default) do
    if File.exists?(sidecar(path, ".previous")) do
      recover_or_quarantine(path, default, :primary_generation_missing)
    else
      default
    end
  end

  defp quarantine_primary(path) do
    corrupt = sidecar(path, ".corrupt")
    File.rm(corrupt)

    if File.exists?(path) do
      File.rename!(path, corrupt)
      true
    else
      false
    end
  end

  defp write_recovery_receipt(path, fields) do
    receipt =
      Map.merge(fields, %{
        "schema_version" => @recovery_schema,
        "storage_file" => Path.basename(path),
        "detected_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601()
      })

    receipt_path = sidecar(path, ".recovery.json")
    File.write!(sidecar(receipt_path, ".next"), Jason.encode!(receipt), [:sync])
    File.rm(receipt_path)
    File.rename!(sidecar(receipt_path, ".next"), receipt_path)
  end

  defp payload_digest(payload) do
    payload
    |> canonical_json_value()
    |> Jason.encode!()
    |> then(&:crypto.hash(:sha256, &1))
    |> Base.encode16(case: :lower)
  end

  defp canonical_json_value(value) when is_map(value) do
    value
    |> Enum.map(fn {key, item} -> {to_string(key), canonical_json_value(item)} end)
    |> Enum.sort_by(&elem(&1, 0))
    |> Jason.OrderedObject.new()
  end

  defp canonical_json_value(value) when is_list(value),
    do: Enum.map(value, &canonical_json_value/1)

  defp canonical_json_value(value), do: value

  defp valid_digest?(digest), do: Regex.match?(~r/\A[0-9a-f]{64}\z/, digest)
  defp sidecar(path, suffix), do: "#{path}#{suffix}"
end
