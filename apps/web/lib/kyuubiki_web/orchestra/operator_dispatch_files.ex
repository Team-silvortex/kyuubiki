defmodule KyuubikiWeb.Orchestra.OperatorDispatchFiles do
  @moduledoc false

  alias KyuubikiWeb.Storage.DurableJson

  @schema "kyuubiki.operator-task-dispatch-record/v1"
  @max_records 512
  @max_file_bytes 4096
  @fields ~w(schema_version attempt_id request_id task_id task_digest operator_id program_id endpoint_fingerprint state created_at_ms updated_at_ms)
  @states ~w(dispatch_boundary_unconfirmed outcome_unknown observed_executed observed_failed observed_blocked not_dispatched)

  def limits, do: %{max_records: @max_records, max_file_bytes: @max_file_bytes}

  def load(root) do
    case File.lstat(root) do
      {:error, :enoent} -> {:ok, %{}}
      {:ok, %{type: :directory}} -> load_existing(root)
      _ -> {:error, :operator_task_dispatch_journal_unavailable}
    end
  end

  defp load_existing(root) do
    with :ok <- ensure_directory(root),
         {:ok, names} <- File.ls(root),
         true <- length(names) <= @max_records * 2 do
      Enum.reduce_while(names, {:ok, %{}}, fn name, {:ok, records} ->
        # A partial replacement is not rolled back: it might hide a newer dispatch.
        with true <- Regex.match?(~r/\A[0-9a-f]{32}\.json\z/, name),
             {:ok, record} <- read(Path.join(root, name)),
             true <- name == record["attempt_id"] <> ".json",
             true <- map_size(records) < @max_records do
          {:cont, {:ok, Map.put(records, record["attempt_id"], record)}}
        else
          _ -> {:halt, {:error, :operator_task_dispatch_journal_invalid}}
        end
      end)
    else
      _ -> {:error, :operator_task_dispatch_journal_unavailable}
    end
  end

  def write(root, record, expected \\ nil) do
    with true <- valid?(record),
         :ok <- ensure_directory(root),
         {:ok, bytes} <- Jason.encode(envelope(record)),
         true <- byte_size(bytes) <= @max_file_bytes do
      path = Path.join(root, record["attempt_id"] <> ".json")
      next = path <> ".next"

      with :ok <- expected_generation(path, expected),
           :ok <- File.write(next, bytes, [:exclusive, :sync]),
           :ok <- File.chmod(next, 0o600),
           :ok <- File.rename(next, path) do
        :ok
      else
        _ -> {:error, :operator_task_dispatch_journal_unavailable}
      end
    else
      _ -> {:error, :operator_task_dispatch_journal_invalid}
    end
  end

  def delete(root, record) do
    path = Path.join(root, record["attempt_id"] <> ".json")

    with :ok <- expected_generation(path, record), :ok <- File.rm(path) do
      :ok
    else
      _ -> {:error, :operator_task_dispatch_journal_unavailable}
    end
  end

  def valid?(record) when is_map(record) do
    Enum.sort(Map.keys(record)) == Enum.sort(@fields) and
      record["schema_version"] == @schema and
      hex?(record["attempt_id"], 32) and hex?(record["endpoint_fingerprint"], 64) and
      hex?(record["task_digest"], 64) and record["state"] in @states and
      Enum.all?(~w(request_id task_id operator_id program_id), &text?(record[&1])) and
      is_integer(record["created_at_ms"]) and record["created_at_ms"] >= 0 and
      is_integer(record["updated_at_ms"]) and
      record["updated_at_ms"] >= record["created_at_ms"] and fits_file_budget?(record)
  end

  def valid?(_record), do: false

  def schema, do: @schema

  def fingerprint(endpoint) do
    [endpoint[:id], endpoint[:host], endpoint[:port], endpoint[:agent_session_id]]
    |> DurableJson.encode!()
    |> digest()
  end

  defp read(path) do
    with {:ok, %{type: :regular, size: size}} <- File.lstat(path),
         true <- size <= @max_file_bytes,
         {:ok, bytes} <- bounded_read(path),
         {:ok, envelope} <- Jason.decode(bytes),
         %{"schema_version" => "kyuubiki.persistence-envelope/v1", "payload" => record} <-
           envelope,
         "sha256" <- envelope["digest_algorithm"],
         true <- valid?(record),
         true <- envelope["payload_sha256"] == digest(DurableJson.encode!(record)) do
      {:ok, record}
    else
      _ -> {:error, :operator_task_dispatch_journal_invalid}
    end
  end

  defp bounded_read(path) do
    with {:ok, file} <- File.open(path, [:read, :binary]) do
      try do
        case IO.binread(file, @max_file_bytes + 1) do
          bytes when is_binary(bytes) and byte_size(bytes) <= @max_file_bytes -> {:ok, bytes}
          _ -> {:error, :operator_task_dispatch_journal_invalid}
        end
      after
        File.close(file)
      end
    end
  end

  defp envelope(record) do
    %{
      "schema_version" => "kyuubiki.persistence-envelope/v1",
      "digest_algorithm" => "sha256",
      "payload_sha256" => digest(DurableJson.encode!(record)),
      "payload" => record
    }
  end

  defp fits_file_budget?(record) do
    case Jason.encode(envelope(record)) do
      {:ok, bytes} -> byte_size(bytes) <= @max_file_bytes
      _ -> false
    end
  end

  defp ensure_directory(root) do
    with :ok <- File.mkdir_p(root),
         {:ok, %{type: :directory}} <- File.lstat(root),
         :ok <- File.chmod(root, 0o700) do
      :ok
    else
      _ -> {:error, :operator_task_dispatch_journal_unavailable}
    end
  end

  defp expected_generation(path, nil) do
    case File.lstat(path) do
      {:error, :enoent} -> :ok
      _ -> {:error, :operator_task_dispatch_journal_invalid}
    end
  end

  defp expected_generation(path, expected) do
    case read(path) do
      {:ok, ^expected} -> :ok
      _ -> {:error, :operator_task_dispatch_journal_invalid}
    end
  end

  defp hex?(value, length) when is_binary(value),
    do: byte_size(value) == length and Regex.match?(~r/\A[0-9a-f]+\z/, value)

  defp hex?(_value, _length), do: false

  defp text?(value) when is_binary(value),
    do: byte_size(value) in 1..1024 and String.valid?(value)

  defp text?(_value), do: false
  defp digest(bytes), do: :crypto.hash(:sha256, bytes) |> Base.encode16(case: :lower)
end
