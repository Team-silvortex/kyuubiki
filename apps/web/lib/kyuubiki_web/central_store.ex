defmodule KyuubikiWeb.CentralStore do
  @moduledoc """
  Future center-server facade for marketplace, auth, and distributable UI assets.
  """

  alias KyuubikiWeb.AssetStore
  alias KyuubikiWeb.Security

  @repo_root Path.expand("../../../../", __DIR__)
  @language_catalog_path Path.join(@repo_root, "language-packs/catalog.json")
  @language_pack_root Path.join(@repo_root, "language-packs")

  @store_kinds [
    "operator",
    "workflow_template",
    "frontend_dsl_template",
    "language_pack"
  ]

  def catalog(filters \\ %{}) when is_map(filters) do
    asset_catalog = AssetStore.catalog(filters)
    language_entries = language_pack_entries(filters)
    entries = asset_catalog["entries"] ++ language_entries

    %{
      "schema_version" => "kyuubiki.central-store-catalog/v1",
      "service" => "kyuubiki-central",
      "status" => "local_preview",
      "entries" => entries,
      "sources" => sources(asset_catalog["sources"]),
      "summary" => summary(entries),
      "capabilities" => capabilities()
    }
  end

  def fetch(kind, id) when kind != "language_pack" and is_binary(id) do
    AssetStore.fetch(kind, id)
  end

  def fetch("language_pack", id) when is_binary(id) do
    language_pack_entries(%{})
    |> Enum.find(&(&1["id"] == id))
    |> case do
      nil -> {:error, {:central_store_entry_not_found, "language_pack", id}}
      entry -> {:ok, %{"entry" => entry}}
    end
  end

  def session_policy do
    %{
      "schema_version" => "kyuubiki.central-session-policy/v1",
      "status" => "preview",
      "current_auth" => %{
        "mode" => "local_orchestra_token",
        "descriptor" => Security.descriptor()
      },
      "planned_auth" => [
        %{
          "id" => "oidc",
          "label" => "OpenID Connect",
          "status" => "planned",
          "intended_clients" => ["hub", "workbench", "headless_sdk"]
        },
        %{
          "id" => "device_code",
          "label" => "Device-code login",
          "status" => "planned",
          "intended_clients" => ["cli", "headless_sdk", "remote_agent"]
        },
        %{
          "id" => "personal_access_token",
          "label" => "Personal access token",
          "status" => "planned",
          "intended_clients" => ["headless_sdk", "installer", "ci"]
        }
      ],
      "session_rules" => %{
        "store_download_requires_session" => false,
        "publish_requires_session" => true,
        "agent_registration_requires_cluster_identity" => true,
        "credential_storage" => "client_platform_keychain_or_memory_only"
      }
    }
  end

  def capabilities do
    %{
      "operator_store" => %{"status" => "ready", "backing" => "AssetStore.operator"},
      "workflow_template_store" => %{
        "status" => "ready",
        "backing" => "AssetStore.workflow_template"
      },
      "frontend_dsl_template_store" => %{
        "status" => "ready",
        "backing" => "AssetStore.frontend_dsl_template"
      },
      "language_pack_store" => %{"status" => "ready", "backing" => "language-packs/catalog.json"},
      "login_system" => %{"status" => "preview_contract", "backing" => "Security.descriptor"},
      "signed_downloads" => %{"status" => "planned"},
      "publisher_accounts" => %{"status" => "planned"}
    }
  end

  defp language_pack_entries(filters) do
    filters = normalize_filters(filters)

    @language_catalog_path
    |> File.read()
    |> case do
      {:ok, raw} -> Jason.decode!(raw)["packs"] || []
      {:error, _reason} -> []
    end
    |> Enum.map(&language_pack_entry/1)
    |> Enum.filter(&matches_filters?(&1, filters))
    |> Enum.sort_by(&{&1["kind"], &1["id"]})
  rescue
    _ -> []
  end

  defp language_pack_entry(pack) do
    %{
      "id" => pack["id"],
      "kind" => "language_pack",
      "title" => pack["name"],
      "summary" => "#{pack["surface"]} language pack for #{pack["language"]}",
      "version" => current_language_pack_version(pack["path"]),
      "source_id" => "builtin.language-packs",
      "source_kind" => "builtin",
      "package_ref" => "language-pack://#{pack["surface"]}/#{pack["language"]}",
      "domain" => "localization",
      "category" => pack["surface"],
      "tags" => ["language_pack", pack["surface"], pack["language"], pack["status"]],
      "install" => %{
        "mode" => "catalog_file",
        "requires_download" => false,
        "target" => "language-packs/#{pack["path"]}"
      },
      "payload" => pack
    }
  end

  defp current_language_pack_version(relative_path) when is_binary(relative_path) do
    @language_pack_root
    |> Path.join(relative_path)
    |> File.read()
    |> case do
      {:ok, raw} -> Jason.decode!(raw)["version"] || "unknown"
      {:error, _reason} -> "unknown"
    end
  rescue
    _ -> "unknown"
  end

  defp sources(asset_sources) do
    asset_sources ++
      [
        %{
          "id" => "builtin.language-packs",
          "type" => "builtin",
          "label" => "Built-in Language Packs",
          "enabled" => true,
          "editable" => false,
          "status" => "ready",
          "supports" => ["language_pack"]
        }
      ]
  end

  defp summary(entries) do
    %{
      "entry_count" => length(entries),
      "kinds" => summarize_by(entries, "kind"),
      "sources" => summarize_by(entries, "source_id"),
      "store_kinds" => @store_kinds
    }
  end

  defp normalize_filters(filters) do
    Map.new(filters, fn {key, value} -> {to_string(key), value} end)
  end

  defp matches_filters?(entry, filters) do
    matches?(entry["kind"], filters["kind"]) and
      matches?(entry["source_id"], filters["source_id"]) and
      matches_query?(entry, filters["q"])
  end

  defp matches?(_actual, value) when value in [nil, ""], do: true
  defp matches?(actual, value), do: actual == value

  defp matches_query?(_entry, value) when value in [nil, ""], do: true

  defp matches_query?(entry, value) when is_binary(value) do
    [entry["id"], entry["title"], entry["summary"], entry["domain"], entry["category"]]
    |> Enum.concat(entry["tags"] || [])
    |> Enum.reject(&is_nil/1)
    |> Enum.join(" ")
    |> String.downcase()
    |> String.contains?(String.downcase(value))
  end

  defp summarize_by(entries, key) do
    Enum.reduce(entries, %{}, fn entry, acc ->
      Map.update(acc, entry[key] || "unknown", 1, &(&1 + 1))
    end)
  end
end
