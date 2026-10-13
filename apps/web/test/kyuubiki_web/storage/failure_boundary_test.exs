defmodule KyuubikiWeb.Storage.FailureBoundaryTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Storage.{AnalysisMemoryState, FailureBoundary}

  test "completion encoding and schema errors retain a non-retryable payload classification" do
    for exception <- [
          %Jason.EncodeError{message: "invalid UTF-8"},
          %Protocol.UndefinedError{protocol: Jason.Encoder, value: fn -> :not_json end},
          %Ecto.ChangeError{message: "invalid persisted value"},
          %Ecto.Query.CastError{message: "private result payload", type: :map, value: :invalid}
        ] do
      assert {:completion_persistence_failed, {:invalid_payload, message}} =
               FailureBoundary.completion_error(exception)

      assert is_binary(message)
      refute message =~ "private result payload"
    end

    assert {:completion_persistence_failed, "injected write failure"} =
             FailureBoundary.completion_error(%Exqlite.Error{message: "injected write failure"})
  end

  test "successful values and domain errors retain their exact shape" do
    for value <- [[], %{}, :ok, {:ok, 42}, {:error, :stale_workflow_execution_claim}] do
      assert FailureBoundary.run(fn -> value end) === value
    end
  end

  for exception <- [DBConnection.ConnectionError, Postgrex.Error, Exqlite.Error] do
    test "#{inspect(exception)} becomes a bounded storage error without internal details" do
      assert {:error, :analysis_store_unavailable} =
               FailureBoundary.run(fn -> raise unquote(exception), message: "private detail" end)
    end
  end

  test "known memory store exit reasons are bounded including unknown timeout outcomes" do
    for store <- [
          AnalysisMemoryState,
          KyuubikiWeb.Library.MemoryBackend,
          KyuubikiWeb.SecurityEvents.MemoryBackend
        ],
        reason <- [:noproc, :normal, :shutdown, :killed, :timeout, {:shutdown, :unavailable}] do
      assert {:error, :analysis_store_unavailable} =
               FailureBoundary.run(fn ->
                 exit({reason, {GenServer, :call, [store, :get, 10]}})
               end)
    end
  end

  test "ordinary exceptions cannot masquerade as a storage outage" do
    assert_raise RuntimeError, "could not lookup Ecto repo", fn ->
      FailureBoundary.run(fn -> raise "could not lookup Ecto repo" end)
    end

    assert_raise ArgumentError, fn -> FailureBoundary.run(fn -> raise ArgumentError end) end
  end

  test "an Ecto registry entry lost during repo restart is a bounded storage outage" do
    {pid, ref} = spawn_monitor(fn -> :ok end)
    assert_receive {:DOWN, ^ref, :process, ^pid, :normal}

    assert {:error, :analysis_store_unavailable} =
             FailureBoundary.run(fn -> Ecto.Repo.Registry.lookup(pid) end)

    assert {:error, :custom_unavailable} =
             FailureBoundary.run(fn -> Ecto.Repo.Registry.lookup(pid) end, :custom_unavailable)
  end

  test "a missing named repo remains a bounded storage outage" do
    assert {:error, :analysis_store_unavailable} =
             FailureBoundary.run(fn -> Ecto.Repo.Registry.lookup(__MODULE__) end)
  end

  test "unrelated ETS errors including direct registry access are not swallowed" do
    table = :ets.new(:failure_boundary, [])

    assert_raise ArgumentError, fn ->
      FailureBoundary.run(fn -> :ets.lookup_element(table, :missing, 4) end)
    end

    assert_raise ArgumentError, fn ->
      FailureBoundary.run(fn -> :ets.lookup_element(Ecto.Repo.Registry, self(), 4) end)
    end
  end

  test "unrelated process failures and storage programming crashes are not swallowed" do
    for reason <- [
          :shutdown,
          {:noproc, {GenServer, :call, [SomeOtherProcess, :get, 10]}},
          {{:badarith, []}, {GenServer, :call, [AnalysisMemoryState, :get, 10]}}
        ] do
      assert catch_exit(FailureBoundary.run(fn -> exit(reason) end)) == reason
    end

    assert catch_throw(FailureBoundary.run(fn -> throw(:domain_abort) end)) == :domain_abort
  end
end
