defmodule KyuubikiWeb.Storage.FailureBoundary do
  @moduledoc false

  @memory_stores [
    KyuubikiWeb.Storage.AnalysisMemoryState,
    KyuubikiWeb.Library.MemoryBackend,
    KyuubikiWeb.SecurityEvents.MemoryBackend,
    KyuubikiWeb.Orchestra.LeaseMemoryBackend
  ]

  # Serialization/schema failures cannot be repaired by retrying the same payload.
  def completion_error(%kind{})
      when kind in [
             Jason.EncodeError,
             Protocol.UndefinedError,
             Ecto.ChangeError,
             Ecto.Query.CastError
           ],
      do:
        {:completion_persistence_failed,
         {:invalid_payload, "completion payload cannot be encoded for its storage schema"}}

  def completion_error(error), do: {:completion_persistence_failed, Exception.message(error)}

  # Preserve successful/domain-error replies. Never replace failed reads with
  # empty collections, or retry a write whose commit outcome may be unknown.
  # Wrap outside SQL transaction boundaries so exceptions roll back first.
  def run(callback, failure \\ :analysis_store_unavailable) when is_function(callback, 0) do
    callback.()
  rescue
    _error in [DBConnection.ConnectionError, Postgrex.Error, Exqlite.Error] ->
      {:error, failure}

    error in [RuntimeError, ArgumentError] ->
      if unavailable_repo_lookup?(__STACKTRACE__),
        do: {:error, failure},
        else: reraise(error, __STACKTRACE__)
  catch
    :exit, {reason, {GenServer, :call, [store | _]}} = exit_reason ->
      if store in @memory_stores and unavailable_exit?(reason),
        do: {:error, failure},
        else: :erlang.raise(:exit, exit_reason, __STACKTRACE__)
  end

  defp unavailable_repo_lookup?([{Ecto.Repo.Registry, :lookup, _, _} | _]), do: true

  # A repo PID can disappear after name resolution but before its ETS lookup.
  defp unavailable_repo_lookup?([
         {:ets, :lookup_element, [Ecto.Repo.Registry, pid, 4], _},
         {Ecto.Repo.Registry, :lookup, _, _} | _
       ])
       when is_pid(pid), do: true

  defp unavailable_repo_lookup?(_), do: false

  defp unavailable_exit?(reason) when reason in [:noproc, :normal, :shutdown, :killed, :timeout],
    do: true

  defp unavailable_exit?({:shutdown, _reason}), do: true
  defp unavailable_exit?(_reason), do: false
end
