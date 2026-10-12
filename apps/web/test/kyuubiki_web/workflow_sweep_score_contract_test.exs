defmodule KyuubikiWeb.WorkflowSweepScoreContractTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @fixtures Path.expand("../../../../tests/fixtures/workflow-sweep-score-contract.json", __DIR__)
  for fixture <- @fixtures |> File.read!() |> Jason.decode!() do
    @fixture fixture
    test "shared sweep score contract: #{fixture["id"]}" do
      result =
        Runtime.run_transform_operator(
          "transform.score_parameter_sweep",
          @fixture["payload"],
          @fixture["config"]
        )

      if @fixture["error"] do
        assert {:error, _} = result
      else
        assert {:ok, actual} = result
        assert_fields(actual, @fixture["expected"])
      end
    end
  end

  defp assert_fields(actual, expected) when is_map(expected) do
    for {key, value} <- expected do
      assert Map.has_key?(actual, key)
      assert_fields(actual[key], value)
    end
  end

  defp assert_fields(actual, expected) when is_list(expected) do
    assert is_list(actual)
    assert length(actual) == length(expected)
    for {value, target} <- Enum.zip(actual, expected), do: assert_fields(value, target)
  end

  defp assert_fields(actual, expected), do: assert(actual == expected)
end
