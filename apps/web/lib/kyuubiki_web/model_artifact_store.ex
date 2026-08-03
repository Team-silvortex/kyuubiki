defmodule KyuubikiWeb.ModelArtifactStore do
  @moduledoc """
  Content-addressed storage for large FEM model payloads.

  Uploads are streamed to disk before JSON decoding so the ordinary control-plane
  parser can keep a conservative request limit.
  """

  import Plug.Conn

  alias KyuubikiWeb.Persistence

  @media_type "application/vnd.kyuubiki.model+json"
  @default_max_bytes 536_870_912
  @read_length 1_048_576
  @read_timeout 120_000

  def media_type, do: @media_type

  def init(options), do: options

  def call(conn, _options) do
    if model_artifact_request?(conn) do
      %{conn | body_params: %{}}
    else
      conn
    end
  end

  def descriptor do
    %{
      "schema_version" => "kyuubiki.model-artifact-transport/v1",
      "upload_endpoint" => "/api/v1/model-artifacts",
      "media_type" => @media_type,
      "max_artifact_bytes" => max_bytes(),
      "digest_algorithm" => "sha256",
      "storage_mode" => "orchestra_content_addressed",
      "storage_namespace" => "KYUUBIKI_DATA_DIR/model-artifacts/sha256"
    }
  end

  def put_conn(conn) do
    with :ok <- validate_content_type(conn),
         {:ok, declared_bytes} <- declared_content_length(conn),
         :ok <- validate_declared_size(declared_bytes),
         {:ok, io, temporary_path} <- open_temporary_file() do
      stream_to_store(conn, io, temporary_path, declared_bytes)
    else
      {:error, status, payload} -> {:error, conn, status, payload}
    end
  end

  def metadata(artifact_id) when is_binary(artifact_id) do
    with :ok <- validate_artifact_id(artifact_id),
         {:ok, stat} <- File.stat(artifact_path(artifact_id)) do
      {:ok, artifact_descriptor(artifact_id, stat.size)}
    else
      _ -> :error
    end
  end

  def resolve_model_params(%{"model_artifact_ref" => reference} = params)
      when is_map(reference) do
    with {:ok, artifact_id} <- artifact_id_from_reference(reference),
         {:ok, model} <- read_verified_model(artifact_id) do
      context = Map.drop(params, ["model_artifact_ref"])
      {:ok, Map.merge(model, context)}
    end
  end

  def resolve_model_params(params) when is_map(params), do: {:ok, params}

  defp stream_to_store(conn, io, temporary_path, declared_bytes) do
    result = stream_body(conn, io, :crypto.hash_init(:sha256), 0, declared_bytes)
    File.close(io)

    case result do
      {:ok, conn, digest, size_bytes} ->
        artifact_id = Base.encode16(digest, case: :lower)
        destination = artifact_path(artifact_id)
        File.mkdir_p!(Path.dirname(destination))
        persist_content_addressed(temporary_path, destination)
        {:ok, conn, artifact_descriptor(artifact_id, size_bytes)}

      {:error, conn, status, payload} ->
        File.rm(temporary_path)
        {:error, conn, status, payload}
    end
  end

  defp stream_body(conn, io, hash, received, declared_bytes) do
    case read_body(conn,
           length: @read_length,
           read_length: @read_length,
           read_timeout: @read_timeout
         ) do
      {:more, chunk, conn} ->
        with {:ok, received, hash} <- write_chunk(io, chunk, received, hash, declared_bytes) do
          stream_body(conn, io, hash, received, declared_bytes)
        else
          {:error, status, payload} -> {:error, conn, status, payload}
        end

      {:ok, chunk, conn} ->
        with {:ok, received, hash} <- write_chunk(io, chunk, received, hash, declared_bytes),
             :ok <- validate_received_size(received, declared_bytes) do
          {:ok, conn, :crypto.hash_final(hash), received}
        else
          {:error, status, payload} -> {:error, conn, status, payload}
        end

      {:error, reason} ->
        {:error, conn, 400, error_payload("artifact_body_read_failed", inspect(reason))}
    end
  end

  defp write_chunk(io, chunk, received, hash, declared_bytes) do
    next_received = received + byte_size(chunk)

    cond do
      next_received > declared_bytes ->
        {:error, 400, error_payload("artifact_content_length_mismatch")}

      next_received > max_bytes() ->
        {:error, 413, oversize_payload(next_received)}

      true ->
        case IO.binwrite(io, chunk) do
          :ok ->
            {:ok, next_received, :crypto.hash_update(hash, chunk)}

          {:error, reason} ->
            {:error, 500, error_payload("artifact_write_failed", inspect(reason))}
        end
    end
  end

  defp validate_received_size(received, received), do: :ok

  defp validate_received_size(_received, _declared),
    do: {:error, 400, error_payload("artifact_content_length_mismatch")}

  defp validate_content_type(conn) do
    if model_artifact_request?(conn),
      do: :ok,
      else: {:error, 415, error_payload("unsupported_artifact_media_type")}
  end

  defp model_artifact_request?(conn) do
    conn
    |> get_req_header("content-type")
    |> Enum.any?(fn value ->
      value |> String.split(";", parts: 2) |> hd() |> String.trim() == @media_type
    end)
  end

  defp declared_content_length(conn) do
    case get_req_header(conn, "content-length") do
      [value] ->
        case Integer.parse(value) do
          {length, ""} when length >= 0 -> {:ok, length}
          _ -> {:error, 400, error_payload("invalid_artifact_content_length")}
        end

      _ ->
        {:error, 411, error_payload("artifact_content_length_required")}
    end
  end

  defp validate_declared_size(0),
    do: {:error, 400, error_payload("empty_model_artifact")}

  defp validate_declared_size(length) when length > 0 do
    if length <= max_bytes(), do: :ok, else: {:error, 413, oversize_payload(length)}
  end

  defp open_temporary_file do
    directory = Path.join(Persistence.data_dir(), "model-artifacts/tmp")
    File.mkdir_p!(directory)
    path = Path.join(directory, Base.url_encode64(:crypto.strong_rand_bytes(18), padding: false))

    case File.open(path, [:write, :binary, :exclusive]) do
      {:ok, io} ->
        {:ok, io, path}

      {:error, reason} ->
        {:error, 500, error_payload("artifact_store_unavailable", inspect(reason))}
    end
  end

  defp persist_content_addressed(temporary_path, destination) do
    if File.exists?(destination) do
      File.rm!(temporary_path)
    else
      File.rename!(temporary_path, destination)
    end
  end

  defp read_verified_model(artifact_id) do
    with :ok <- validate_artifact_id(artifact_id),
         {:ok, bytes} <- File.read(artifact_path(artifact_id)),
         :ok <- verify_digest(bytes, artifact_id),
         {:ok, model} when is_map(model) <- Jason.decode(bytes) do
      {:ok, model}
    else
      {:error, :enoent} -> {:error, {:model_artifact_not_found, artifact_id}}
      {:error, reason} -> {:error, {:invalid_model_artifact, artifact_id, reason}}
      _ -> {:error, {:invalid_model_artifact, artifact_id}}
    end
  end

  defp artifact_id_from_reference(reference) do
    case Map.get(reference, "artifact_id") || Map.get(reference, :artifact_id) do
      artifact_id when is_binary(artifact_id) ->
        with :ok <- validate_artifact_id(artifact_id),
             :ok <- validate_reference_digest(reference, artifact_id) do
          {:ok, String.downcase(artifact_id)}
        end

      _ ->
        {:error, :missing_model_artifact_id}
    end
  end

  defp validate_reference_digest(reference, artifact_id) do
    digest = Map.get(reference, "sha256") || Map.get(reference, :sha256)

    if digest in [nil, artifact_id, String.downcase(artifact_id)],
      do: :ok,
      else: {:error, :model_artifact_digest_mismatch}
  end

  defp validate_artifact_id(artifact_id) do
    if byte_size(artifact_id) == 64 and
         String.match?(artifact_id, ~r/\A[0-9a-fA-F]{64}\z/) do
      :ok
    else
      {:error, :invalid_model_artifact_id}
    end
  end

  defp verify_digest(bytes, artifact_id) do
    digest = :crypto.hash(:sha256, bytes) |> Base.encode16(case: :lower)
    if digest == String.downcase(artifact_id), do: :ok, else: {:error, :digest_mismatch}
  end

  defp artifact_path(artifact_id) do
    digest = String.downcase(artifact_id)

    Persistence.data_dir()
    |> Path.join("model-artifacts/sha256")
    |> Path.join(String.slice(digest, 0, 2))
    |> Path.join("#{digest}.json")
  end

  defp artifact_descriptor(artifact_id, size_bytes) do
    %{
      "schema_version" => "kyuubiki.model-artifact-ref/v1",
      "artifact_id" => artifact_id,
      "sha256" => artifact_id,
      "size_bytes" => size_bytes,
      "media_type" => @media_type,
      "immutable" => true
    }
  end

  defp max_bytes do
    case System.get_env("KYUUBIKI_MODEL_ARTIFACT_MAX_BYTES") do
      value when is_binary(value) ->
        case Integer.parse(value) do
          {bytes, ""} when bytes > 0 -> bytes
          _ -> @default_max_bytes
        end

      _ ->
        @default_max_bytes
    end
  end

  defp oversize_payload(size_bytes) do
    %{
      "error" => "model_artifact_too_large",
      "payload_bytes" => size_bytes,
      "max_artifact_bytes" => max_bytes()
    }
  end

  defp error_payload(error, detail \\ nil) do
    %{"error" => error}
    |> then(fn payload -> if detail, do: Map.put(payload, "detail", detail), else: payload end)
  end
end
