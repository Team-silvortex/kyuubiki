defmodule KyuubikiWeb.OperatorPackageDistributionStore do
  @moduledoc false

  import Plug.Conn

  @distribution_file "kyuubiki-operator-distribution.json"
  @distribution_schema "kyuubiki.operator-package-distribution/v1"
  @package_schema "kyuubiki.operator-package/v1"
  @sdk_api "kyuubiki.operator-sdk/v1"
  @digest_pattern ~r/\A[0-9a-f]{64}\z/
  @token_pattern ~r/\A(?:[A-Za-z0-9]|[A-Za-z0-9][A-Za-z0-9._-]{0,126}[A-Za-z0-9])\z/
  @version_pattern ~r/\A(?:[A-Za-z0-9]|[A-Za-z0-9][A-Za-z0-9._-]{0,126}[A-Za-z0-9])\z/
  @target_pattern ~r/\A(?:macos|linux|windows)-[a-z0-9_]+\z/

  def resolve(package_id, package_version, target) do
    with {:ok, root} <- configured_root(),
         :ok <- validate_identity(package_id, package_version, target),
         relative_distribution <- Path.join([package_id, package_version, @distribution_file]),
         {:ok, distribution_path} <- regular_path(root, relative_distribution),
         {:ok, distribution_bytes} <- File.read(distribution_path),
         {:ok, distribution} <- Jason.decode(distribution_bytes),
         :ok <- validate_distribution(distribution, package_id, package_version),
         {:ok, artifact} <- select_artifact(distribution, target),
         version_root <- Path.join(package_id, package_version),
         {:ok, manifest} <- verified_artifact(root, version_root, artifact, "manifest"),
         {:ok, entrypoint} <- verified_artifact(root, version_root, artifact, "entrypoint"),
         :ok <- validate_package_manifest(manifest.path, artifact, package_id, package_version) do
      base_path = "/api/v1/central/operator-packages/#{package_id}/#{package_version}/#{target}"

      {:ok,
       %{
         "schema_version" => "kyuubiki.operator-package-resolution/v1",
         "package_ref" => "orchestra://operator-package/#{package_id}",
         "package_id" => package_id,
         "package_version" => package_version,
         "sdk_api_version" => @sdk_api,
         "target" => target,
         "authority_mode" => "bound_orchestra",
         "cache_scope" => "task_required_disposable",
         "distribution_sha256" => sha256(distribution_bytes),
         "manifest" => artifact_descriptor(manifest, "#{base_path}/manifest"),
         "entrypoint" => artifact_descriptor(entrypoint, "#{base_path}/entrypoint")
       }}
    end
  rescue
    _ -> {:error, :invalid_operator_package_distribution}
  end

  def send_artifact(conn, package_id, package_version, target, kind)
      when kind in ["manifest", "entrypoint"] do
    with {:ok, resolution} <- resolve(package_id, package_version, target),
         descriptor <- Map.fetch!(resolution, kind),
         {:ok, root} <- configured_root(),
         {:ok, path} <- regular_path(root, descriptor["path"]) do
      content_type =
        if kind == "manifest",
          do: "application/vnd.kyuubiki.operator-package+json",
          else: "application/octet-stream"

      conn =
        conn
        |> put_resp_content_type(content_type)
        |> put_resp_header("cache-control", "private, immutable")
        |> put_resp_header("x-content-type-options", "nosniff")
        |> put_resp_header("x-kyuubiki-sha256", descriptor["sha256"])
        |> put_resp_header("etag", ~s("sha256:#{descriptor["sha256"]}"))
        |> send_file(200, path, 0, descriptor["size_bytes"])

      {:ok, conn}
    end
  end

  defp configured_root do
    config = Application.get_env(:kyuubiki_web, __MODULE__, [])
    root = config[:root] || System.get_env("KYUUBIKI_OPERATOR_PACKAGE_DISTRIBUTIONS")

    cond do
      !is_binary(root) or root == "" ->
        {:error, :operator_package_distribution_root_unconfigured}

      true ->
        expanded = Path.expand(root)

        case File.lstat(expanded) do
          {:ok, %{type: :directory}} -> {:ok, expanded}
          _ -> {:error, :operator_package_distribution_root_unavailable}
        end
    end
  end

  defp validate_identity(package_id, package_version, target) do
    if Regex.match?(@token_pattern, package_id) and
         Regex.match?(@version_pattern, package_version) and Regex.match?(@target_pattern, target),
       do: :ok,
       else: {:error, :invalid_operator_package_identity}
  end

  defp validate_distribution(distribution, package_id, package_version)
       when is_map(distribution) do
    cond do
      distribution["schema_version"] != @distribution_schema ->
        {:error, :unsupported_operator_distribution_schema}

      distribution["sdk_api_version"] != @sdk_api ->
        {:error, :unsupported_operator_sdk_api}

      distribution["package_id"] != package_id or
          distribution["package_version"] != package_version ->
        {:error, :operator_distribution_identity_mismatch}

      !is_list(distribution["artifacts"]) or distribution["artifacts"] == [] ->
        {:error, :operator_distribution_has_no_artifacts}

      duplicate_targets?(distribution["artifacts"]) ->
        {:error, :operator_distribution_has_duplicate_targets}

      true ->
        :ok
    end
  end

  defp validate_distribution(_, _, _), do: {:error, :invalid_operator_package_distribution}

  defp duplicate_targets?(artifacts) do
    targets = Enum.map(artifacts, &Map.get(&1, "target"))
    length(targets) != length(Enum.uniq(targets))
  end

  defp select_artifact(distribution, target) do
    case Enum.find(distribution["artifacts"], &(&1["target"] == target)) do
      nil -> {:error, :operator_package_target_unavailable}
      artifact when is_map(artifact) -> validate_selected_artifact(artifact, target)
      _ -> {:error, :invalid_operator_package_artifact}
    end
  end

  defp validate_selected_artifact(artifact, target) do
    with :ok <- validate_artifact_fields(artifact, "manifest"),
         :ok <- validate_artifact_fields(artifact, "entrypoint"),
         true <- artifact["manifest_path"] != artifact["entrypoint_path"],
         true <- Path.basename(artifact["manifest_path"]) == "kyuubiki-operator.json" do
      {:ok, artifact}
    else
      _ -> {:error, :invalid_operator_package_artifact}
    end
    |> ensure_target_paths(target)
  end

  defp validate_artifact_fields(artifact, prefix) do
    path = artifact["#{prefix}_path"]
    digest = artifact["#{prefix}_sha256"]
    size = artifact["#{prefix}_size_bytes"]

    if safe_relative_path?(path) and is_binary(digest) and Regex.match?(@digest_pattern, digest) and
         is_integer(size) and size > 0,
       do: :ok,
       else: {:error, :invalid_operator_package_artifact}
  end

  defp ensure_target_paths({:ok, artifact}, target) do
    prefix = target <> "/"

    if String.starts_with?(artifact["manifest_path"], prefix) and
         String.starts_with?(artifact["entrypoint_path"], prefix),
       do: {:ok, artifact},
       else: {:error, :operator_artifact_target_path_mismatch}
  end

  defp ensure_target_paths(error, _target), do: error

  defp verified_artifact(root, version_root, artifact, prefix) do
    relative_path = Path.join(version_root, artifact["#{prefix}_path"])
    expected_digest = artifact["#{prefix}_sha256"]
    expected_size = artifact["#{prefix}_size_bytes"]

    case regular_path(root, relative_path) do
      {:ok, path} ->
        verify_regular_artifact(
          path,
          relative_path,
          expected_digest,
          expected_size
        )

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp verify_regular_artifact(path, relative_path, expected_digest, expected_size) do
    with {:ok, stat} <- File.stat(path),
         :ok <- compare_artifact_size(stat.size, expected_size),
         {:ok, digest} <- sha256_file(path),
         :ok <- compare_artifact_digest(digest, expected_digest) do
      {:ok,
       %{
         path: path,
         relative_path: relative_path,
         sha256: expected_digest,
         size_bytes: expected_size
       }}
    else
      {:error, reason} -> {:error, reason}
      _ -> {:error, :operator_artifact_unavailable}
    end
  end

  defp compare_artifact_size(size, size), do: :ok
  defp compare_artifact_size(_, _), do: {:error, :operator_artifact_size_mismatch}

  defp compare_artifact_digest(digest, digest), do: :ok
  defp compare_artifact_digest(_, _), do: {:error, :operator_artifact_digest_mismatch}

  defp validate_package_manifest(path, artifact, package_id, package_version) do
    with {:ok, bytes} <- File.read(path),
         {:ok, manifest} <- Jason.decode(bytes),
         true <- manifest["schema_version"] == @package_schema,
         true <- manifest["sdk_api_version"] == @sdk_api,
         true <- manifest["package_id"] == package_id,
         true <- manifest["package_version"] == package_version,
         entrypoint when is_binary(entrypoint) <- manifest["entrypoint"],
         {:ok, expanded} <- expand_entrypoint(entrypoint, artifact["target"]),
         true <- artifact["entrypoint_path"] == Path.join(artifact["target"], expanded) do
      :ok
    else
      _ -> {:error, :operator_package_manifest_mismatch}
    end
  end

  defp expand_entrypoint(entrypoint, target) do
    with true <- safe_relative_path?(entrypoint),
         {:ok, prefix, extension} <- target_library(target) do
      {:ok,
       entrypoint
       |> String.replace("{lib_prefix}", prefix)
       |> String.replace("{lib_extension}", extension)}
    else
      _ -> {:error, :invalid_operator_entrypoint}
    end
  end

  defp target_library("windows-" <> _), do: {:ok, "", "dll"}
  defp target_library("macos-" <> _), do: {:ok, "lib", "dylib"}
  defp target_library("linux-" <> _), do: {:ok, "lib", "so"}
  defp target_library(_), do: {:error, :unsupported_operator_target}

  defp artifact_descriptor(artifact, download_path) do
    %{
      "path" => artifact.relative_path,
      "sha256" => artifact.sha256,
      "size_bytes" => artifact.size_bytes,
      "download_path" => download_path
    }
  end

  defp regular_path(root, relative_path) do
    with true <- safe_relative_path?(relative_path),
         components <- Path.split(relative_path),
         {:ok, path} <- walk_regular_path(root, components) do
      {:ok, path}
    else
      _ -> {:error, :unsafe_operator_artifact_path}
    end
  end

  defp walk_regular_path(root, components) do
    components
    |> Enum.with_index()
    |> Enum.reduce_while({:ok, root}, fn {component, index}, {:ok, parent} ->
      path = Path.join(parent, component)
      final? = index == length(components) - 1

      case File.lstat(path) do
        {:ok, %{type: :regular}} when final? -> {:cont, {:ok, path}}
        {:ok, %{type: :directory}} when not final? -> {:cont, {:ok, path}}
        _ -> {:halt, {:error, :unsafe_operator_artifact_path}}
      end
    end)
  end

  defp safe_relative_path?(path) when is_binary(path) do
    path != "" and !String.starts_with?(path, ["/", "\\"]) and !String.contains?(path, "\\") and
      Enum.all?(Path.split(path), &(&1 not in ["", ".", ".."]))
  end

  defp safe_relative_path?(_), do: false

  defp sha256_file(path) do
    digest =
      path
      |> File.stream!([:raw, :read_ahead, :binary], 65_536)
      |> Enum.reduce(:crypto.hash_init(:sha256), &:crypto.hash_update(&2, &1))
      |> :crypto.hash_final()
      |> Base.encode16(case: :lower)

    {:ok, digest}
  rescue
    _ -> {:error, :operator_artifact_hash_failed}
  end

  defp sha256(bytes),
    do: :crypto.hash(:sha256, bytes) |> Base.encode16(case: :lower)
end
