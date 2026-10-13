defmodule KyuubikiWeb.Orchestra.OperatorDispatchRetention do
  @moduledoc false

  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Storage.DispatchRetentionStore

  @terminal ~w(completed failed cancelled)

  def releasable(records) do
    ids = records |> Enum.map(&Files.job_id/1) |> Enum.reject(&is_nil/1) |> Enum.uniq()

    with {:ok, parents} <- DispatchRetentionStore.snapshot(ids) do
      {:ok, Enum.filter(records, &releasable_parent?(Files.job_id(&1), parents))}
    else
      {:error, _} -> {:error, :operator_task_dispatch_journal_unavailable}
    end
  end

  defp releasable_parent?(nil, _parents), do: true

  defp releasable_parent?(id, parents) do
    case parents[id] do
      %{job_status: status, recovery_present: false} when status in [nil | @terminal] -> true
      %{job_status: status, released_state: status} when status in @terminal -> true
      _ -> false
    end
  end
end
