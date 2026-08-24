defmodule KyuubikiWeb.Api.OperatorPackageDistributionApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.OperatorPackageDistributionStore

  @package_id "operator.example.peak_field"
  @package_version "0.1.0"
  @target "linux-x86_64"

  setup do
    root =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-operator-distributions-#{System.unique_integer([:positive, :monotonic])}"
      )

    original = Application.get_env(:kyuubiki_web, OperatorPackageDistributionStore, [])
    Application.put_env(:kyuubiki_web, OperatorPackageDistributionStore, root: root)
    fixture = write_distribution(root)

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, OperatorPackageDistributionStore, original)
      File.rm_rf(root)
    end)

    {:ok, Map.put(fixture, :root, root)}
  end

  test "resolves and downloads only the requested platform package", fixture do
    resolution_conn =
      :get
      |> conn(resolve_path())
      |> Router.call(@opts)

    assert resolution_conn.status == 200, resolution_conn.resp_body
    resolution = Jason.decode!(resolution_conn.resp_body)
    assert resolution["schema_version"] == "kyuubiki.operator-package-resolution/v1"
    assert resolution["package_ref"] == "orchestra://operator-package/#{@package_id}"
    assert resolution["target"] == @target
    assert resolution["authority_mode"] == "bound_orchestra"
    assert resolution["cache_scope"] == "task_required_disposable"
    assert resolution["manifest"]["sha256"] == sha256(fixture.manifest_bytes)
    assert resolution["entrypoint"]["sha256"] == sha256(fixture.entrypoint_bytes)

    manifest_conn =
      :get
      |> conn(resolution["manifest"]["download_path"])
      |> Router.call(@opts)

    assert manifest_conn.status == 200
    assert manifest_conn.resp_body == fixture.manifest_bytes
    assert get_resp_header(manifest_conn, "x-content-type-options") == ["nosniff"]

    entrypoint_conn =
      :get
      |> conn(resolution["entrypoint"]["download_path"])
      |> Router.call(@opts)

    assert entrypoint_conn.status == 200
    assert entrypoint_conn.resp_body == fixture.entrypoint_bytes

    assert get_resp_header(entrypoint_conn, "x-kyuubiki-sha256") == [
             sha256(fixture.entrypoint_bytes)
           ]
  end

  test "returns an explicit miss instead of substituting another platform" do
    conn =
      :get
      |> conn(
        "/api/v1/central/operator-packages/#{@package_id}/#{@package_version}/macos-aarch64/resolve"
      )
      |> Router.call(@opts)

    assert conn.status == 404
    assert Jason.decode!(conn.resp_body)["error"] == "operator_package_target_unavailable"
  end

  test "refuses a package after its entrypoint is tampered", fixture do
    File.write!(fixture.entrypoint_path, "tampered")

    conn =
      :get
      |> conn(resolve_path())
      |> Router.call(@opts)

    assert conn.status == 422
    payload = Jason.decode!(conn.resp_body)
    assert payload["error"] == "operator_package_resolution_failed"

    assert payload["reason"] in [
             "operator_artifact_size_mismatch",
             "operator_artifact_digest_mismatch"
           ]
  end

  test "refuses a symlinked artifact even when its bytes match", fixture do
    outside = fixture.entrypoint_path <> ".outside"
    File.write!(outside, fixture.entrypoint_bytes)
    File.rm!(fixture.entrypoint_path)
    File.ln_s!(outside, fixture.entrypoint_path)

    conn =
      :get
      |> conn(resolve_path())
      |> Router.call(@opts)

    assert conn.status == 422
    assert Jason.decode!(conn.resp_body)["reason"] == "unsafe_operator_artifact_path"
    File.rm(outside)
  end

  defp resolve_path do
    "/api/v1/central/operator-packages/#{@package_id}/#{@package_version}/#{@target}/resolve"
  end

  defp write_distribution(root) do
    version_root = Path.join([root, @package_id, @package_version])
    target_root = Path.join(version_root, @target)
    File.mkdir_p!(target_root)

    entrypoint_bytes = <<0x7F, ?E, ?L, ?F, 1, 2, 3, 4>>
    entrypoint_path = Path.join(target_root, "liboperator_example_peak_field.so")
    File.write!(entrypoint_path, entrypoint_bytes)

    manifest = %{
      "schema_version" => "kyuubiki.operator-package/v1",
      "sdk_api_version" => "kyuubiki.operator-sdk/v1",
      "package_id" => @package_id,
      "package_version" => @package_version,
      "minimum_host_version" => "2.15.0",
      "validation_status" => "verified",
      "validation_notes" => "Central distribution API fixture.",
      "runtime" => "rust_cdylib",
      "entrypoint" => "{lib_prefix}operator_example_peak_field.{lib_extension}",
      "operators" => [
        %{
          "operator_id" => "extract.electrostatic_peak_field",
          "kind" => "extract",
          "entry_symbol" => "register_operator"
        }
      ]
    }

    manifest_bytes = Jason.encode!(manifest)
    manifest_path = Path.join(target_root, "kyuubiki-operator.json")
    File.write!(manifest_path, manifest_bytes)

    distribution = %{
      "schema_version" => "kyuubiki.operator-package-distribution/v1",
      "sdk_api_version" => "kyuubiki.operator-sdk/v1",
      "package_id" => @package_id,
      "package_version" => @package_version,
      "artifacts" => [
        %{
          "target" => @target,
          "manifest_path" => "#{@target}/kyuubiki-operator.json",
          "manifest_sha256" => sha256(manifest_bytes),
          "manifest_size_bytes" => byte_size(manifest_bytes),
          "entrypoint_path" => "#{@target}/liboperator_example_peak_field.so",
          "entrypoint_sha256" => sha256(entrypoint_bytes),
          "entrypoint_size_bytes" => byte_size(entrypoint_bytes)
        }
      ]
    }

    File.write!(
      Path.join(version_root, "kyuubiki-operator-distribution.json"),
      Jason.encode!(distribution)
    )

    %{
      manifest_bytes: manifest_bytes,
      manifest_path: manifest_path,
      entrypoint_bytes: entrypoint_bytes,
      entrypoint_path: entrypoint_path
    }
  end

  defp sha256(bytes),
    do: :crypto.hash(:sha256, bytes) |> Base.encode16(case: :lower)
end
