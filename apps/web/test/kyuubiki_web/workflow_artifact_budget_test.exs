defmodule KyuubikiWeb.WorkflowArtifactBudgetTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowJsonBudget
  alias KyuubikiWeb.TestSupport.WorkflowArtifactContract, as: Fixture

  for recipe <- Fixture.cases() do
    @recipe recipe
    test "shared output budget: #{recipe["id"]}" do
      result = WorkflowJsonBudget.validate_output(Fixture.value(@recipe), "test output")

      if @recipe["accept"] do
        assert result == :ok
      else
        assert {:error, reason} = result
        assert reason =~ @recipe["error"]
      end
    end
  end

  test "native non-JSON terms cannot hide inside generated artifacts" do
    values = [
      self(),
      make_ref(),
      fn -> :ok end,
      {1, 2},
      :atom,
      %{atom_key: 1},
      <<255>>,
      [1 | 2],
      [1 | "tail"],
      %URI{scheme: "https"},
      %{<<255>> => 0}
    ]

    for value <- values do
      assert {:error, _} = WorkflowJsonBudget.validate_output(%{"nested" => value}, "output")
      assert {:error, _} = WorkflowJsonBudget.validate(%{"nested" => value}, "input", 500_000)
    end
  end
end
