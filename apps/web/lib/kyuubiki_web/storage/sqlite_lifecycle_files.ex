defmodule KyuubikiWeb.Storage.SqliteLifecycleFiles do
  @moduledoc false
  @max_bytes 16 * 1024 * 1024 * 1024

  def max_bytes, do: @max_bytes

  def regular!(path) do
    case File.lstat(path) do
      {:ok, %{type: :regular, size: size}} when size > 0 and size <= @max_bytes -> :ok
      _ -> raise "database input must be a nonempty regular file within the 16 GiB limit"
    end
  end

  def standalone!(path) do
    regular!(path)

    Enum.each(["-wal", "-journal"], fn suffix ->
      case File.lstat(path <> suffix) do
        {:error, :enoent} -> :ok
        {:ok, %{type: :regular, size: 0}} -> :ok
        _ -> raise "database has live journal/WAL state; use backup first, not a main-file copy"
      end
    end)
  end

  def safe_sidecars!(path) do
    Enum.each(["-wal", "-shm", "-journal"], fn suffix ->
      case File.lstat(path <> suffix) do
        {:error, :enoent} -> :ok
        {:ok, %{type: :regular, size: size}} when size <= @max_bytes -> :ok
        _ -> raise "database sidecar is not a bounded regular file"
      end
    end)
  end

  def digest!(path) do
    regular!(path)

    {context, _size} =
      path
      |> File.stream!(1024 * 1024)
      |> Enum.reduce({:crypto.hash_init(:sha256), 0}, fn bytes, {context, size} ->
        size = size + byte_size(bytes)
        if size > @max_bytes, do: raise("database exceeds 16 GiB safety limit")
        {:crypto.hash_update(context, bytes), size}
      end)

    :crypto.hash_final(context) |> Base.encode16(case: :lower)
  end

  def approve!(path, expected) do
    unless is_binary(expected) and Regex.match?(~r/\A[0-9a-f]{64}\z/, expected),
      do: raise("expected SHA-256 must be 64 lowercase hexadecimal characters")

    standalone!(path)
    if digest!(path) != expected, do: raise("database digest differs from the approved snapshot")
  end

  def copy!(source, target) do
    File.open!(target, [:write, :binary, :exclusive], fn device ->
      source
      |> File.stream!(1024 * 1024)
      |> Enum.reduce(0, fn bytes, size ->
        size = size + byte_size(bytes)
        if size > @max_bytes, do: raise("database exceeds copy safety limit")
        :ok = IO.binwrite(device, bytes)
        size
      end)
    end)
  end

  def publish!(output, fun) do
    output = Path.expand(output)
    absent!(output)

    stage =
      Path.join(
        Path.dirname(output),
        ".kyuubiki-data-" <> Base.encode16(:crypto.strong_rand_bytes(16), case: :lower)
      )

    File.mkdir!(stage)

    outcome =
      try do
        private!(stage, 0o700)
        candidate = Path.join(stage, "candidate.sqlite3")
        receipt = fun.(candidate)
        standalone!(candidate)
        private!(candidate, 0o600)
        sync!(candidate)
        digest = digest!(candidate)
        # Atomic, no-overwrite publication. Unsupported filesystems fail closed.
        absent!(output)
        File.ln!(candidate, output)

        result =
          Map.merge(receipt, %{
            "schema_version" => "kyuubiki.sqlite-lifecycle-receipt/v1",
            "output_sha256" => digest,
            "output_bytes" => File.stat!(candidate).size,
            "directory_synced" => sync_directory(Path.dirname(output)),
            "source_retained" => true,
            "activation" => "explicit_service_switch",
            "created_at" => DateTime.utc_now() |> DateTime.to_iso8601()
          })

        {:ok, result}
      catch
        kind, reason -> {:error, kind, reason, __STACKTRACE__}
      end

    case {outcome, File.rm_rf(stage)} do
      {{:ok, receipt}, {:ok, _}} ->
        receipt

      {{:ok, receipt}, {:error, _reason, _}} ->
        Map.put(receipt, "cleanup_pending", Path.basename(stage))

      {{:error, kind, reason, stack}, {:ok, _}} ->
        :erlang.raise(kind, reason, stack)

      {{:error, _kind, reason, _stack}, {:error, cleanup, _}} ->
        raise "database operation failed (#{inspect(reason)}); staging cleanup pending at #{Path.basename(stage)}: #{inspect(cleanup)}"
    end
  end

  defp absent!(path) do
    Enum.each([path, path <> "-wal", path <> "-shm", path <> "-journal"], fn name ->
      unless File.lstat(name) == {:error, :enoent},
        do: raise("database output or sidecar already exists; refusing overwrite")
    end)
  end

  defp private!(path, mode) do
    if match?({:unix, _}, :os.type()), do: File.chmod!(path, mode)
  end

  defp sync!(path) do
    {:ok, device} = :file.open(String.to_charlist(path), [:read, :write, :raw, :binary])

    try do
      :ok = :file.sync(device)
    after
      :file.close(device)
    end
  end

  defp sync_directory(path) do
    case :file.open(String.to_charlist(path), [:read, :raw, :directory]) do
      {:ok, device} ->
        result = :file.sync(device) == :ok
        :file.close(device)
        result

      _ ->
        false
    end
  end
end
