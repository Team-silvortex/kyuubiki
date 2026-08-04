defmodule KyuubikiWeb.ModelArtifactStore do
  @moduledoc """
  Content-addressed storage for large FEM model payloads.
  """

  alias KyuubikiWeb.ContentArtifactStore

  defdelegate init(options), to: ContentArtifactStore
  defdelegate call(conn, options), to: ContentArtifactStore
  def media_type, do: ContentArtifactStore.media_type(:model)
  def descriptor, do: ContentArtifactStore.descriptor(:model)
  def put_conn(conn), do: ContentArtifactStore.put_conn(conn, :model)
  def metadata(artifact_id), do: ContentArtifactStore.metadata(:model, artifact_id)

  def send_content(conn, artifact_id),
    do: ContentArtifactStore.send_content(conn, :model, artifact_id)

  def prepare_agent_params(%{"model_artifact_ref" => reference}) when is_map(reference) do
    with {:ok, _artifact_id, artifact} <-
           ContentArtifactStore.validate_reference(:model, reference) do
      {:ok, %{"model_artifact_ref" => artifact}}
    end
  end

  def prepare_agent_params(params) when is_map(params), do: {:inline, params}

  def resolve_model_params(%{"model_artifact_ref" => reference} = params)
      when is_map(reference) do
    with {:ok, artifact_id, _artifact} <-
           ContentArtifactStore.validate_reference(:model, reference),
         {:ok, model} when is_map(model) <-
           ContentArtifactStore.read_verified_json(:model, artifact_id) do
      {:ok, Map.merge(model, Map.drop(params, ["model_artifact_ref"]))}
    else
      {:error, {:artifact_not_found, :model, artifact_id}} ->
        {:error, {:model_artifact_not_found, artifact_id}}

      {:error, reason} ->
        {:error, reason}

      _ ->
        {:error, :invalid_model_artifact}
    end
  end

  def resolve_model_params(params) when is_map(params), do: {:ok, params}
end
