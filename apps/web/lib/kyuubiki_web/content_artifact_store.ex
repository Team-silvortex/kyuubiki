defmodule KyuubikiWeb.ContentArtifactStore do
  @moduledoc false

  import Plug.Conn

  alias KyuubikiWeb.Persistence

  @read_length 1_048_576
  @read_timeout 120_000
  @default_temp_retention_seconds 3_600
  @kinds %{
    model: %{
      media_type: "application/vnd.kyuubiki.model+json",
      namespace: "model-artifacts",
      max_env: "KYUUBIKI_MODEL_ARTIFACT_MAX_BYTES",
      default_max_bytes: 536_870_912
    },
    result: %{
      media_type: "application/vnd.kyuubiki.result+json",
      namespace: "result-artifacts",
      max_env: "KYUUBIKI_RESULT_ARTIFACT_MAX_BYTES",
      default_max_bytes: 2_147_483_648
    }
  }

  def init(options), do: options

  def call(conn, _options) do
    if artifact_request?(conn), do: %{conn | body_params: %{}}, else: conn
  end

  def media_type(kind), do: config(kind).media_type

  def descriptor(kind) do
    config = config(kind)

    %{
      "schema_version" => "kyuubiki.#{kind}-artifact-transport/v1",
      "upload_endpoint" => "/api/v1/#{config.namespace}",
      "media_type" => config.media_type,
      "max_artifact_bytes" => max_bytes(kind),
      "digest_algorithm" => "sha256",
      "storage_mode" => "orchestra_content_addressed",
      "storage_namespace" => "KYUUBIKI_DATA_DIR/#{config.namespace}/sha256",
      "temporary_retention_seconds" => temp_retention_seconds()
    }
  end

  def put_conn(conn, kind) do
    with :ok <- validate_content_type(conn, kind),
         {:ok, declared_bytes} <- declared_content_length(conn),
         :ok <- validate_declared_size(declared_bytes, kind),
         {:ok, io, temporary_path} <- open_temporary_file(kind) do
      stream_to_store(conn, io, temporary_path, declared_bytes, kind)
    else
      {:error, status, payload} -> {:error, conn, status, payload}
    end
  end

  def metadata(kind, artifact_id) when is_binary(artifact_id) do
    with :ok <- validate_artifact_id(artifact_id),
         {:ok, stat} <- File.stat(artifact_path(kind, artifact_id)) do
      {:ok, artifact_descriptor(kind, String.downcase(artifact_id), stat.size)}
    else
      _ -> :error
    end
  end

  def validate_reference(kind, reference) when is_map(reference) do
    with {:ok, artifact_id} <- artifact_id_from_reference(reference),
         {:ok, artifact} <- metadata(kind, artifact_id),
         :ok <- validate_reference_size(reference, artifact) do
      {:ok, artifact_id, artifact}
    end
  end

  def send_content(conn, kind, artifact_id) when is_binary(artifact_id) do
    with :ok <- validate_artifact_id(artifact_id),
         path <- artifact_path(kind, artifact_id),
         {:ok, stat} <- File.stat(path) do
      conn =
        conn
        |> put_resp_content_type(media_type(kind))
        |> put_resp_header("cache-control", "private, immutable")
        |> put_resp_header("x-kyuubiki-sha256", String.downcase(artifact_id))
        |> send_file(200, path, 0, stat.size)

      {:ok, conn}
    else
      _ -> :error
    end
  end

  def read_verified_json(kind, artifact_id) do
    with :ok <- validate_artifact_id(artifact_id),
         {:ok, bytes} <- File.read(artifact_path(kind, artifact_id)),
         :ok <- verify_digest(bytes, artifact_id),
         {:ok, value} <- Jason.decode(bytes) do
      {:ok, value}
    else
      {:error, :enoent} -> {:error, {:artifact_not_found, kind, artifact_id}}
      {:error, reason} -> {:error, {:invalid_artifact, kind, artifact_id, reason}}
      _ -> {:error, {:invalid_artifact, kind, artifact_id}}
    end
  end

  defp stream_to_store(conn, io, temporary_path, declared_bytes, kind) do
    result = stream_body(conn, io, :crypto.hash_init(:sha256), 0, declared_bytes, kind)
    File.close(io)

    case result do
      {:ok, conn, digest, size_bytes} ->
        artifact_id = Base.encode16(digest, case: :lower)
        destination = artifact_path(kind, artifact_id)
        File.mkdir_p!(Path.dirname(destination))
        persist_content_addressed(temporary_path, destination)
        {:ok, conn, artifact_descriptor(kind, artifact_id, size_bytes)}

      {:error, conn, status, payload} ->
        File.rm(temporary_path)
        {:error, conn, status, payload}
    end
  end

  defp stream_body(conn, io, hash, received, declared_bytes, kind) do
    case read_body(conn,
           length: @read_length,
           read_length: @read_length,
           read_timeout: @read_timeout
         ) do
      {:more, chunk, conn} ->
        with {:ok, received, hash} <-
               write_chunk(io, chunk, received, hash, declared_bytes, kind) do
          stream_body(conn, io, hash, received, declared_bytes, kind)
        else
          {:error, status, payload} -> {:error, conn, status, payload}
        end

      {:ok, chunk, conn} ->
        with {:ok, received, hash} <-
               write_chunk(io, chunk, received, hash, declared_bytes, kind),
             :ok <- validate_received_size(received, declared_bytes) do
          {:ok, conn, :crypto.hash_final(hash), received}
        else
          {:error, status, payload} -> {:error, conn, status, payload}
        end

      {:error, reason} ->
        {:error, conn, 400, error_payload(kind, "artifact_body_read_failed", inspect(reason))}
    end
  end

  defp write_chunk(io, chunk, received, hash, declared_bytes, kind) do
    next_received = received + byte_size(chunk)

    cond do
      next_received > declared_bytes ->
        {:error, 400, error_payload(kind, "artifact_content_length_mismatch")}

      next_received > max_bytes(kind) ->
        {:error, 413, oversize_payload(kind, next_received)}

      true ->
        case :file.write(io, chunk) do
          :ok ->
            {:ok, next_received, :crypto.hash_update(hash, chunk)}

          {:error, reason} ->
            {:error, 500, error_payload(kind, "artifact_write_failed", inspect(reason))}
        end
    end
  end

  defp validate_received_size(received, received), do: :ok

  defp validate_received_size(_received, _declared),
    do: {:error, 400, %{"error" => "artifact_content_length_mismatch"}}

  defp validate_content_type(conn, kind) do
    if request_media_type(conn) == media_type(kind),
      do: :ok,
      else: {:error, 415, error_payload(kind, "unsupported_artifact_media_type")}
  end

  defp artifact_request?(conn),
    do: request_media_type(conn) in Enum.map(Map.keys(@kinds), &media_type/1)

  defp request_media_type(conn) do
    conn
    |> get_req_header("content-type")
    |> List.first("")
    |> String.split(";", parts: 2)
    |> hd()
    |> String.trim()
  end

  defp declared_content_length(conn) do
    case get_req_header(conn, "content-length") do
      [value] ->
        case Integer.parse(value) do
          {length, ""} when length >= 0 -> {:ok, length}
          _ -> {:error, 400, %{"error" => "invalid_artifact_content_length"}}
        end

      _ ->
        {:error, 411, %{"error" => "artifact_content_length_required"}}
    end
  end

  defp validate_declared_size(0, kind),
    do: {:error, 400, error_payload(kind, "empty_#{kind}_artifact")}

  defp validate_declared_size(length, kind) do
    if length <= max_bytes(kind), do: :ok, else: {:error, 413, oversize_payload(kind, length)}
  end

  defp open_temporary_file(kind) do
    directory = artifact_root(kind, "tmp")
    File.mkdir_p!(directory)
    cleanup_stale_temporary_files(directory)
    path = Path.join(directory, Base.url_encode64(:crypto.strong_rand_bytes(18), padding: false))

    case File.open(path, [:write, :binary, :exclusive]) do
      {:ok, io} ->
        {:ok, io, path}

      {:error, reason} ->
        {:error, 500, error_payload(kind, "artifact_store_unavailable", inspect(reason))}
    end
  end

  defp persist_content_addressed(temporary_path, destination) do
    if File.exists?(destination),
      do: File.rm!(temporary_path),
      else: File.rename!(temporary_path, destination)
  end

  defp artifact_id_from_reference(reference) do
    case Map.get(reference, "artifact_id") || Map.get(reference, :artifact_id) do
      artifact_id when is_binary(artifact_id) ->
        with :ok <- validate_artifact_id(artifact_id),
             :ok <- validate_reference_digest(reference, artifact_id) do
          {:ok, String.downcase(artifact_id)}
        end

      _ ->
        {:error, :missing_artifact_id}
    end
  end

  defp validate_reference_digest(reference, artifact_id) do
    digest = Map.get(reference, "sha256") || Map.get(reference, :sha256)

    if digest in [nil, artifact_id, String.downcase(artifact_id)],
      do: :ok,
      else: {:error, :artifact_digest_mismatch}
  end

  defp validate_reference_size(reference, artifact) do
    expected_size = artifact["size_bytes"]

    case Map.get(reference, "size_bytes") || Map.get(reference, :size_bytes) do
      nil -> :ok
      ^expected_size -> :ok
      _ -> {:error, :artifact_size_mismatch}
    end
  end

  defp validate_artifact_id(artifact_id) do
    if byte_size(artifact_id) == 64 and String.match?(artifact_id, ~r/\A[0-9a-fA-F]{64}\z/),
      do: :ok,
      else: {:error, :invalid_artifact_id}
  end

  defp verify_digest(bytes, artifact_id) do
    digest = :crypto.hash(:sha256, bytes) |> Base.encode16(case: :lower)
    if digest == String.downcase(artifact_id), do: :ok, else: {:error, :digest_mismatch}
  end

  defp artifact_path(kind, artifact_id) do
    digest = String.downcase(artifact_id)

    artifact_root(kind, "sha256")
    |> Path.join(String.slice(digest, 0, 2))
    |> Path.join("#{digest}.json")
  end

  defp artifact_root(kind, leaf),
    do: Path.join([Persistence.data_dir(), config(kind).namespace, leaf])

  defp artifact_descriptor(kind, artifact_id, size_bytes) do
    %{
      "schema_version" => "kyuubiki.#{kind}-artifact-ref/v1",
      "artifact_id" => artifact_id,
      "sha256" => artifact_id,
      "size_bytes" => size_bytes,
      "media_type" => media_type(kind),
      "immutable" => true
    }
  end

  defp max_bytes(kind) do
    config = config(kind)

    case System.get_env(config.max_env) do
      value when is_binary(value) ->
        case Integer.parse(value) do
          {bytes, ""} when bytes > 0 -> bytes
          _ -> config.default_max_bytes
        end

      _ ->
        config.default_max_bytes
    end
  end

  defp cleanup_stale_temporary_files(directory) do
    cutoff = System.system_time(:second) - temp_retention_seconds()

    directory
    |> File.ls!()
    |> Enum.each(fn name ->
      path = Path.join(directory, name)

      case File.stat(path, time: :posix) do
        {:ok, %{type: :regular, mtime: mtime}} when mtime < cutoff -> File.rm(path)
        _ -> :ok
      end
    end)
  end

  defp temp_retention_seconds do
    configured =
      System.get_env("KYUUBIKI_ARTIFACT_TEMP_RETENTION_SECONDS") ||
        System.get_env("KYUUBIKI_MODEL_ARTIFACT_TEMP_RETENTION_SECONDS")

    case configured do
      value when is_binary(value) ->
        case Integer.parse(value) do
          {seconds, ""} when seconds > 0 -> seconds
          _ -> @default_temp_retention_seconds
        end

      _ ->
        @default_temp_retention_seconds
    end
  end

  defp oversize_payload(kind, size_bytes) do
    %{
      "error" => "#{kind}_artifact_too_large",
      "payload_bytes" => size_bytes,
      "max_artifact_bytes" => max_bytes(kind)
    }
  end

  defp error_payload(_kind, error, detail \\ nil) do
    %{"error" => error}
    |> then(fn payload -> if detail, do: Map.put(payload, "detail", detail), else: payload end)
  end

  defp config(kind), do: Map.fetch!(@kinds, kind)
end
