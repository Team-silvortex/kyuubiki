defmodule KyuubikiWeb.Library.ProjectRequest do
  @moduledoc false

  def identify(attrs) do
    if id = attrs["request_id"] do
      %{
        request_key: digest(["project", id]),
        request_digest: digest([attrs["name"], attrs["description"]])
      }
    end
  end

  def receipt(identity, response),
    do: Map.merge(identity, %{project_id: response["project_id"], response_meta: response})

  def replay(receipt, identity, exists?) do
    cond do
      receipt.request_digest != identity.request_digest -> {:error, :project_request_conflict}
      not exists? -> {:error, :project_result_deleted}
      true -> {:ok, receipt.response_meta}
    end
  end

  defp digest(value),
    do: :crypto.hash(:sha256, Jason.encode!(value)) |> Base.encode16(case: :lower)
end
