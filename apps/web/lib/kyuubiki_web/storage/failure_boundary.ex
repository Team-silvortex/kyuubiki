defmodule KyuubikiWeb.Storage.FailureBoundary do
  @moduledoc false

  @memory_stores [
    KyuubikiWeb.Storage.AnalysisMemoryState,
    KyuubikiWeb.Orchestra.LeaseMemoryBackend
  ]

  # Preserve successful/domain-error replies. Never replace failed reads with
  # empty collections, or retry a write whose commit outcome may be unknown.
  # Wrap outside SQL transaction boundaries so exceptions roll back first.
  def run(callback, failure \\ :analysis_store_unavailable) when is_function(callback, 0) do
    callback.()
  rescue
    _error in [DBConnection.ConnectionError, Postgrex.Error, Exqlite.Error] ->
      {:error, failure}

    error in RuntimeError ->
      case __STACKTRACE__ do
        [{Ecto.Repo.Registry, :lookup, _, _} | _] -> {:error, failure}
        _ -> reraise error, __STACKTRACE__
      end
  catch
    :exit, {reason, {GenServer, :call, [store | _]}} = exit_reason ->
      if store in @memory_stores and unavailable_exit?(reason),
        do: {:error, failure},
        else: :erlang.raise(:exit, exit_reason, __STACKTRACE__)
  end

  defp unavailable_exit?(reason) when reason in [:noproc, :normal, :shutdown, :killed, :timeout],
    do: true

  defp unavailable_exit?({:shutdown, _reason}), do: true
  defp unavailable_exit?(_reason), do: false
end
