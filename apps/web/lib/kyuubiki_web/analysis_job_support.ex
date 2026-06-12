defmodule KyuubikiWeb.AnalysisJobSupport do
  @moduledoc """
  Shared helpers for creating and serializing analysis jobs.
  """

  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Library

  @spec create_job(map()) :: {:ok, term()} | {:error, term()}
  def create_job(attrs) when is_map(attrs) do
    Store.create(%{
      job_id: random_id(),
      project_id: Map.get(attrs, :project_id, random_id()),
      model_version_id: Map.get(attrs, :model_version_id),
      simulation_case_id: Map.get(attrs, :simulation_case_id, random_id())
    })
  end

  @spec derive_job_context(map()) :: {:ok, map()} | {:error, term()}
  def derive_job_context(params) when is_map(params) do
    project_id = fetch_optional_string(params, ["project_id", :project_id])
    model_version_id = fetch_optional_string(params, ["model_version_id", :model_version_id])

    cond do
      is_binary(model_version_id) and model_version_id != "" ->
        case Library.get_version(model_version_id) do
          {:ok, version} ->
            {:ok,
             %{
               project_id: version["project_id"],
               model_version_id: version["version_id"],
               simulation_case_id: version["version_id"]
             }}

          :error ->
            {:error, {:model_version_not_found, model_version_id}}
        end

      is_binary(project_id) and project_id != "" ->
        {:ok, %{project_id: project_id}}

      true ->
        {:ok, %{}}
    end
  end

  @spec serialize_payload(term()) :: map()
  def serialize_payload(job) do
    %{
      "job" => %{
        "job_id" => job.job_id,
        "project_id" => job.project_id,
        "model_version_id" => job.model_version_id,
        "simulation_case_id" => job.simulation_case_id,
        "worker_id" => job.worker_id,
        "message" => job.message,
        "status" => Atom.to_string(job.status),
        "progress" => job.progress,
        "residual" => job.residual,
        "iteration" => job.iteration,
        "created_at" => DateTime.to_iso8601(job.created_at),
        "updated_at" => DateTime.to_iso8601(job.updated_at),
        "has_result" => false
      }
    }
  end

  @spec stringify_keys(term()) :: term()
  def stringify_keys(value) when is_list(value), do: Enum.map(value, &stringify_keys/1)

  def stringify_keys(value) when is_map(value) do
    value
    |> Enum.map(fn {key, nested} -> {to_string(key), stringify_keys(nested)} end)
    |> Map.new()
  end

  def stringify_keys(value), do: value

  defp random_id do
    :crypto.strong_rand_bytes(8) |> Base.encode16(case: :lower)
  end

  defp fetch_optional_string(params, [key | rest]) do
    case Map.fetch(params, key) do
      {:ok, value} when is_binary(value) ->
        trimmed = String.trim(value)

        if byte_size(trimmed) > 0 do
          trimmed
        else
          fetch_optional_string(params, rest)
        end

      _ ->
        fetch_optional_string(params, rest)
    end
  end

  defp fetch_optional_string(_params, []), do: nil
end
