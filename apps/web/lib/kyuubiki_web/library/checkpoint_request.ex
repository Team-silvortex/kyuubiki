defmodule KyuubikiWeb.Library.CheckpointRequest do
  @moduledoc false

  def normalize(attrs) do
    case Map.get(attrs, "request_id", Map.get(attrs, :request_id)) do
      nil ->
        {:ok, nil}

      id when is_binary(id) ->
        if Regex.match?(~r/\A[A-Za-z0-9_-]{16,128}\z/, id),
          do: {:ok, id},
          else: {:error, :invalid_checkpoint_request_id}

      _ ->
        {:error, :invalid_checkpoint_request_id}
    end
  end

  def identify(operation, attrs) do
    if id = attrs["request_id"] do
      parent = if operation == :model, do: attrs["project_id"], else: attrs["model_id"]
      content = Map.take(attrs, ~w(name kind material model_schema_version payload))

      %{
        request_key: lookup_key(operation, parent, id),
        request_digest: digest(content)
      }
    end
  end

  def lookup_key(operation, parent, id), do: digest([Atom.to_string(operation), parent, id])

  # A read-only receipt lookup never needs or returns the submitted mesh payload.
  # Unknown means not observed yet, not permission to submit a replacement write.
  def status(nil, _exists?), do: %{"status" => "unknown"}

  def status(receipt, exists?) do
    %{
      "status" => if(exists?, do: "committed", else: "deleted"),
      "project_id" => receipt.project_id,
      "model_id" => receipt.model_id,
      "version_id" => receipt.version_id
    }
  end

  def receipt(identity, response, operation) do
    version_id =
      if operation == :model, do: response["latest_version_id"], else: response["version_id"]

    Map.merge(identity, %{
      project_id: response["project_id"],
      model_id: response["model_id"],
      version_id: version_id,
      response_meta: without_payload(response)
    })
  end

  def replay(receipt, identity, attrs, exists?) do
    cond do
      receipt.request_digest != identity.request_digest -> {:error, :checkpoint_request_conflict}
      not exists? -> {:error, :checkpoint_result_deleted}
      true -> {:ok, with_payload(receipt.response_meta, attrs["payload"])}
    end
  end

  # Receipts keep only small response metadata, never a second copy of mesh geometry.
  defp without_payload(response) do
    response = Map.delete(response, "payload")

    if Map.has_key?(response, "versions"),
      do:
        Map.update!(
          response,
          "versions",
          &Enum.map(&1, fn version -> Map.delete(version, "payload") end)
        ),
      else: response
  end

  defp with_payload(response, payload) do
    response = Map.put(response, "payload", payload)

    if Map.has_key?(response, "versions"),
      do:
        Map.update!(
          response,
          "versions",
          &Enum.map(&1, fn version -> Map.put(version, "payload", payload) end)
        ),
      else: response
  end

  defp digest(value), do: :crypto.hash(:sha256, canonical(value)) |> Base.encode16(case: :lower)

  defp canonical(value) when is_map(value) do
    entries =
      value
      |> Enum.sort_by(fn {key, _} -> key end)
      |> Enum.map(fn {key, item} -> [Jason.encode!(key), ":", canonical(item)] end)

    ["{", Enum.intersperse(entries, ","), "}"]
  end

  defp canonical(value) when is_list(value),
    do: ["[", Enum.intersperse(Enum.map(value, &canonical/1), ","), "]"]

  defp canonical(value), do: Jason.encode!(value)
end
