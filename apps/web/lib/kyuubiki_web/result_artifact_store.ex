defmodule KyuubikiWeb.ResultArtifactStore do
  @moduledoc """
  Content-addressed storage for solver results too large for an RPC frame.
  """

  alias KyuubikiWeb.ContentArtifactStore

  def media_type, do: ContentArtifactStore.media_type(:result)
  def descriptor, do: ContentArtifactStore.descriptor(:result)
  def put_conn(conn), do: ContentArtifactStore.put_conn(conn, :result)
  def metadata(artifact_id), do: ContentArtifactStore.metadata(:result, artifact_id)

  def send_content(conn, artifact_id),
    do: ContentArtifactStore.send_content(conn, :result, artifact_id)
end
