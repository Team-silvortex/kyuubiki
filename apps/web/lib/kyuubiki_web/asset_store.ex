defmodule KyuubikiWeb.AssetStore do
  @moduledoc """
  Unified catalog surface for operators, workflow templates, and frontend DSL templates.
  """

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowTemplateCatalog

  @builtin_source_id "builtin.local"
  @entry_kinds ["operator", "workflow_template", "frontend_dsl_template"]

  def catalog(filters \\ %{}) when is_map(filters) do
    sources = sources()

    entries =
      (builtin_entries() ++ external_entries(sources))
      |> Enum.filter(&matches_filters?(&1, filters))
      |> Enum.sort_by(&{&1["kind"], &1["id"]})

    %{
      "entries" => entries,
      "sources" => sources,
      "summary" => %{
        "entry_count" => length(entries),
        "kinds" => summarize_by(entries, "kind"),
        "sources" => summarize_by(entries, "source_id")
      }
    }
  end

  def sources do
    [builtin_source() | configured_sources()]
  end

  def fetch(kind, id) when is_binary(kind) and is_binary(id) do
    catalog(%{"kind" => kind})
    |> Map.fetch!("entries")
    |> Enum.find(&(&1["id"] == id))
    |> case do
      nil -> {:error, {:store_entry_not_found, kind, id}}
      entry -> {:ok, %{"entry" => entry}}
    end
  end

  defp builtin_entries do
    operator_entries() ++ workflow_template_entries() ++ frontend_dsl_template_entries()
  end

  defp operator_entries do
    WorkflowOperatorCatalog.list()
    |> Enum.map(fn operator ->
      %{
        "id" => operator["id"],
        "kind" => "operator",
        "title" => operator["id"],
        "summary" => operator["summary"] || operator["family"] || operator["kind"],
        "version" => operator["version"] || "1.0.0",
        "source_id" => @builtin_source_id,
        "source_kind" => "builtin",
        "package_ref" => get_in(operator, ["execution", "package_ref"]),
        "domain" => operator["domain"],
        "category" => operator["kind"],
        "tags" => operator["capability_tags"] || [],
        "install" => %{
          "mode" => "builtin_reference",
          "requires_download" => false,
          "target" => get_in(operator, ["execution", "source_ref"])
        },
        "payload" => operator
      }
    end)
  end

  defp workflow_template_entries do
    WorkflowTemplateCatalog.list()
    |> Enum.map(fn workflow ->
      %{
        "id" => workflow["id"],
        "kind" => "workflow_template",
        "title" => workflow["name"] || workflow["id"],
        "summary" => workflow["summary"] || workflow["description"] || workflow["name"],
        "version" => workflow["version"] || "1.0.0",
        "source_id" => @builtin_source_id,
        "source_kind" => "builtin",
        "package_ref" => "orchestra://workflow-template/#{workflow["id"]}",
        "domain" => workflow["domain"],
        "category" => "workflow",
        "tags" => workflow["capability_tags"] || workflow["tags"] || [],
        "install" => %{
          "mode" => "builtin_reference",
          "requires_download" => false,
          "target" => workflow["id"]
        },
        "payload" => workflow
      }
    end)
  end

  defp frontend_dsl_template_entries do
    [
      %{
        "id" => "frontend.dsl.layout_report",
        "kind" => "frontend_dsl_template",
        "title" => "Frontend layout report",
        "summary" =>
          "Built-in Workbench DSL script for layout anchors, runtime tabs, and UI state checks.",
        "version" => "kyuubiki.frontend-dsl/v1",
        "source_id" => @builtin_source_id,
        "source_kind" => "builtin",
        "package_ref" => "workbench://frontend-dsl-template/layout_report",
        "domain" => "frontend_automation",
        "category" => "ui_validation",
        "tags" => ["frontend_dsl", "layout", "ui_validation", "wasm_python_ready"],
        "install" => %{
          "mode" => "builtin_reference",
          "requires_download" => false,
          "target" => "buildDefaultWorkbenchFrontendDslDocument"
        },
        "payload" => %{
          "dsl_version" => "kyuubiki.frontend-dsl/v1",
          "entrypoint" => "buildDefaultWorkbenchFrontendDslDocument",
          "source_module" => "@/lib/scripting/workbench-script-dsl-templates"
        }
      }
    ]
  end

  defp external_entries(sources) do
    sources
    |> Enum.reject(&(&1["id"] == @builtin_source_id))
    |> Enum.flat_map(&external_source_entries/1)
  end

  defp external_source_entries(%{"enabled" => false}), do: []

  defp external_source_entries(%{"type" => "catalog_file", "path" => path} = source)
       when is_binary(path) do
    case File.read(path) do
      {:ok, content} ->
        content
        |> Jason.decode!()
        |> Map.get("entries", [])
        |> Enum.filter(&valid_external_entry?/1)
        |> Enum.map(&normalize_external_entry(&1, source))

      {:error, _reason} ->
        []
    end
  rescue
    _ -> []
  end

  defp external_source_entries(_source), do: []

  defp normalize_external_entry(entry, source) do
    entry
    |> Map.take([
      "id",
      "kind",
      "title",
      "summary",
      "version",
      "package_ref",
      "domain",
      "category",
      "tags",
      "install",
      "payload"
    ])
    |> Map.put("source_id", source["id"])
    |> Map.put("source_kind", source["type"])
    |> Map.put_new("tags", [])
    |> Map.put_new("install", %{"mode" => "source_reference", "requires_download" => true})
  end

  defp valid_external_entry?(%{"id" => id, "kind" => kind})
       when is_binary(id) and kind in @entry_kinds,
       do: true

  defp valid_external_entry?(_entry), do: false

  defp matches_filters?(entry, filters) do
    matches_kind?(entry, filters["kind"]) and
      matches_source?(entry, filters["source_id"]) and
      matches_query?(entry, filters["q"])
  end

  defp matches_kind?(_entry, value) when value in [nil, ""], do: true
  defp matches_kind?(entry, value), do: entry["kind"] == value

  defp matches_source?(_entry, value) when value in [nil, ""], do: true
  defp matches_source?(entry, value), do: entry["source_id"] == value

  defp matches_query?(_entry, value) when value in [nil, ""], do: true

  defp matches_query?(entry, value) when is_binary(value) do
    haystack =
      [
        entry["id"],
        entry["title"],
        entry["summary"],
        entry["domain"],
        entry["category"],
        Enum.join(entry["tags"] || [], " ")
      ]
      |> Enum.reject(&is_nil/1)
      |> Enum.join(" ")
      |> String.downcase()

    String.contains?(haystack, String.downcase(value))
  end

  defp summarize_by(entries, key) do
    Enum.reduce(entries, %{}, fn entry, acc ->
      Map.update(acc, entry[key] || "unknown", 1, &(&1 + 1))
    end)
  end

  defp builtin_source do
    %{
      "id" => @builtin_source_id,
      "type" => "builtin",
      "label" => "Built-in Kyuubiki Store",
      "enabled" => true,
      "editable" => false,
      "status" => "ready",
      "supports" => @entry_kinds
    }
  end

  defp configured_sources do
    env_sources()
    |> Kernel.++(Application.get_env(:kyuubiki_web, __MODULE__, [])[:sources] || [])
    |> Enum.map(&normalize_source/1)
    |> Enum.reject(&is_nil/1)
  end

  defp env_sources do
    case System.get_env("KYUUBIKI_STORE_SOURCES") do
      value when is_binary(value) and value != "" ->
        case Jason.decode(value) do
          {:ok, sources} when is_list(sources) -> sources
          _ -> []
        end

      _ ->
        []
    end
  end

  defp normalize_source(source) when is_map(source) do
    id = source["id"] || source[:id]
    type = source["type"] || source[:type] || "remote_http"

    if is_binary(id) and id != "" do
      %{
        "id" => id,
        "type" => type,
        "label" => source["label"] || source[:label] || id,
        "enabled" => Map.get(source, "enabled", Map.get(source, :enabled, true)),
        "editable" => Map.get(source, "editable", Map.get(source, :editable, true)),
        "status" => source_status(type),
        "url" => source["url"] || source[:url],
        "path" => source["path"] || source[:path],
        "supports" => source["supports"] || source[:supports] || @entry_kinds
      }
    end
  end

  defp normalize_source(_source), do: nil

  defp source_status("catalog_file"), do: "configured"
  defp source_status("remote_http"), do: "configured"
  defp source_status(_type), do: "configured"
end
