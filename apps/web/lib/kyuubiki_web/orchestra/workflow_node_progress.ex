defmodule KyuubikiWeb.Orchestra.WorkflowNodeProgress do
  @moduledoc false

  def normalize(
        %{"node_id" => id, "completed_nodes" => completed, "total_nodes" => total} = event
      )
      when is_binary(id) and id != "" and is_integer(total) and total > 0 do
    skipped = Map.get(event, "skipped_nodes", 0)
    failed = Map.get(event, "failed_nodes", 0)
    status = Map.get(event, "status", "completed")

    with true <- Enum.all?([completed, skipped, failed], &(is_integer(&1) and &1 >= 0)),
         resolved <- completed + skipped + failed,
         true <- resolved <= total,
         true <- Map.get(event, "resolved_nodes", resolved) == resolved,
         true <- status in ["completed", "skipped", "failed"] do
      {:ok,
       %{
         "node_id" => id,
         "status" => status,
         "completed_nodes" => completed,
         "skipped_nodes" => skipped,
         "failed_nodes" => failed,
         "resolved_nodes" => resolved,
         "total_nodes" => total
       }}
    else
      _ -> {:error, :invalid_workflow_progress}
    end
  end

  def normalize(_event), do: {:error, :invalid_workflow_progress}

  def event(progress, value) do
    Map.merge(progress, %{
      "progress" => value,
      "emitted_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601()
    })
  end

  def persist?(progress, previous) do
    case normalize(progress) do
      {:ok, event} ->
        now = System.monotonic_time(:millisecond)
        resolved = event["resolved_nodes"]
        stride = max(div(event["total_nodes"], 100), 1)

        is_nil(previous) or event["status"] == "failed" or resolved == event["total_nodes"] or
          resolved - previous.resolved >= stride or now - previous.persisted_at_ms >= 250

      {:error, _} ->
        true
    end
  end

  def remember(state, job_id, progress) do
    case normalize(progress) do
      {:ok, event} ->
        put_in(state, [:progress, job_id], %{
          resolved: event["resolved_nodes"],
          persisted_at_ms: System.monotonic_time(:millisecond)
        })

      {:error, _} ->
        state
    end
  end
end
