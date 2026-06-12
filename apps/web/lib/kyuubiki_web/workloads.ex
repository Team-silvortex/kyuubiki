defmodule KyuubikiWeb.Workloads do
  @moduledoc """
  Hub-facing workload catalog and project bundle export helpers.
  """

  alias KyuubikiWeb.Analysis
  alias KyuubikiWeb.Library

  @project_schema_version "kyuubiki.project/v2"
  @project_layout_version "kyuubiki.project-layout/v1"
  @workload_catalog_version "kyuubiki.workload-catalog/v1"

  def workload_catalog(base_url) when is_binary(base_url) do
    {:ok, projects} = Library.list_projects()
    jobs = Analysis.list_jobs()["jobs"]
    results = Analysis.list_results()["results"]

    %{
      "schema_version" => @workload_catalog_version,
      "sourceLabel" => "Kyuubiki Control Plane",
      "generated_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601(),
      "workloads" =>
        Enum.map(projects, fn project ->
          models = Map.get(project, "models", [])
          versions = Enum.flat_map(models, &list_versions_for_model/1)
          project_jobs = Enum.filter(jobs, &(&1["project_id"] == project["project_id"]))
          project_job_ids = MapSet.new(Enum.map(project_jobs, & &1["job_id"]))

          project_results =
            Enum.filter(results, fn result ->
              MapSet.member?(project_job_ids, result["job_id"])
            end)

          %{
            "label" => project["name"],
            "description" => project["description"],
            "project_id" => project["project_id"],
            "project_name" => project["name"],
            "schema" => @project_schema_version,
            "layout" => @project_layout_version,
            "download_url" => "#{base_url}/api/v1/projects/#{project["project_id"]}/bundle",
            "model_count" => length(models),
            "version_count" => length(versions),
            "job_count" => length(project_jobs),
            "result_count" => length(project_results),
            "analysis_domains" => workload_analysis_domains(models),
            "analysis_families" => workload_analysis_families(models),
            "thermal_intents" => workload_thermal_intents(models),
            "tags" => workload_tags(models)
          }
        end)
    }
  end

  def export_project_bundle(project_id) when is_binary(project_id) do
    with {:ok, project} <- Library.get_project(project_id),
         {:ok, models} <- Library.list_models(project_id) do
      versions = Enum.flat_map(models, &list_versions_for_model/1)
      jobs = project_jobs(project_id)
      job_ids = MapSet.new(Enum.map(jobs, & &1["job_id"]))

      results =
        Enum.filter(Analysis.list_results()["results"], &MapSet.member?(job_ids, &1["job_id"]))

      {:ok,
       %{
         "project_schema_version" => @project_schema_version,
         "exported_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601(),
         "project_file_manifest" => project_file_manifest(),
         "project" => project,
         "models" => models,
         "model_versions" => versions,
         "jobs" => jobs,
         "results" => results,
         "active_model_id" => active_model_id(models),
         "active_version_id" => active_version_id(models, versions)
       }}
    else
      :error -> {:error, :project_not_found}
      {:error, _reason} = error -> error
    end
  end

  def bundle_filename(project) when is_map(project) do
    slug =
      project["name"]
      |> to_string()
      |> String.downcase()
      |> String.replace(~r/[^a-z0-9]+/u, "-")
      |> String.trim("-")

    base = if slug == "", do: project["project_id"], else: slug
    "#{base || "project"}.kyuubiki.json"
  end

  defp project_jobs(project_id) do
    Analysis.list_jobs()["jobs"]
    |> Enum.filter(&(&1["project_id"] == project_id))
  end

  defp list_versions_for_model(model) do
    case Library.list_versions(model["model_id"]) do
      {:ok, versions} -> versions
      _ -> []
    end
  end

  defp workload_tags(models) do
    models
    |> Enum.map(& &1["kind"])
    |> Enum.reject(&is_nil/1)
    |> Enum.map(&to_string/1)
    |> Enum.uniq()
  end

  defp workload_analysis_domains(models) do
    models
    |> Enum.map(&extract_analysis_metadata/1)
    |> Enum.map(&Map.get(&1, "domain"))
    |> Enum.reject(&is_nil/1)
    |> Enum.uniq()
  end

  defp workload_thermal_intents(models) do
    models
    |> Enum.map(&extract_analysis_metadata/1)
    |> Enum.flat_map(fn metadata ->
      case Map.get(metadata, "thermal_intent") do
        intents when is_list(intents) -> Enum.filter(intents, &is_binary/1)
        _ -> []
      end
    end)
    |> Enum.uniq()
  end

  defp workload_analysis_families(models) do
    models
    |> Enum.map(&extract_analysis_metadata/1)
    |> Enum.map(&Map.get(&1, "family"))
    |> Enum.reject(&is_nil/1)
    |> Enum.uniq()
  end

  defp extract_analysis_metadata(model) do
    payload = Map.get(model, "payload")

    case payload do
      %{"analysis_metadata" => metadata} when is_map(metadata) -> metadata
      _ -> %{}
    end
  end

  defp active_model_id([]), do: nil
  defp active_model_id([model | _]), do: model["model_id"]

  defp active_version_id(models, versions) do
    preferred =
      models
      |> Enum.find_value(fn model ->
        latest_id = model["latest_version_id"]

        if latest_id && Enum.any?(versions, &(&1["version_id"] == latest_id)),
          do: latest_id,
          else: nil
      end)

    preferred || (List.first(versions) || %{})["version_id"]
  end

  defp project_file_manifest do
    %{
      "layout_version" => @project_layout_version,
      "engine_manifest_path" => ".kyuubiki/project.json",
      "root_manifest_path" => "project.kyuubiki.json",
      "project_record_path" => "ProjectSettings/project.json",
      "workspace_settings_path" => "Workspace/settings.json",
      "workspace_snapshot_path" => "Workspace/snapshot.json",
      "automation_presets_path" => "ProjectSettings/automation-presets.json",
      "asset_catalog_path" => "ProjectSettings/asset-catalog.json",
      "asset_references_path" => "ProjectSettings/asset-references.json",
      "model_directory" => "Assets/Models",
      "version_directory" => "Assets/ModelVersions",
      "job_directory" => "Analysis/Jobs",
      "result_directory" => "Analysis/Results"
    }
  end
end
