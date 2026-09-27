defmodule KyuubikiWeb.Storage.FailureBoundaryTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Storage.{AnalysisMemoryState, FailureBoundary}

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
    for reason <- [:noproc, :normal, :shutdown, :killed, :timeout, {:shutdown, :unavailable}] do
      assert {:error, :analysis_store_unavailable} =
               FailureBoundary.run(fn ->
                 exit({reason, {GenServer, :call, [AnalysisMemoryState, :get, 10]}})
               end)
    end
  end

  test "ordinary exceptions cannot masquerade as a storage outage" do
    assert_raise RuntimeError, "could not lookup Ecto repo", fn ->
      FailureBoundary.run(fn -> raise "could not lookup Ecto repo" end)
    end

    assert_raise ArgumentError, fn -> FailureBoundary.run(fn -> raise ArgumentError end) end
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
