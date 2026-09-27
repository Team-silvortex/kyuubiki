defmodule KyuubikiWeb.WorkflowGraphBudgetTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.{WorkflowGraphPreflight, WorkflowTemplateCatalog}
  alias KyuubikiWeb.TestSupport.WorkflowPreflightContract, as: Fixture

  for budget <- Fixture.budgets() do
    @budget budget
    test "shared budget boundary: #{budget["id"]}" do
      request = Fixture.budget_request(@budget)
      result = WorkflowGraphPreflight.validate(request["graph"], request["input_artifacts"])
      if @budget["accept"], do: assert(result == :ok), else: assert(match?({:error, _}, result))
    end
  end

  test "raw callback requests cannot hide non-JSON config or artifact values" do
    for value <- [self(), make_ref(), fn -> :ok end, {1, 2}, %{not_a_string: 1}, <<255>>] do
      request = put_in(Fixture.request(), ["input_artifacts", "input"], value)

      assert {:error, {:invalid_workflow_graph, _}} =
               WorkflowGraphPreflight.validate(request["graph"], request["input_artifacts"])
    end
  end

  test "metadata obeys the same config JSON budget" do
    request =
      put_in(Fixture.request(), ["graph", "dataset_contract"], %{"label" => "bad" <> <<0>>})

    assert {:error, {:invalid_workflow_graph, reason}} =
             WorkflowGraphPreflight.validate(request["graph"], %{})

    assert reason =~ "dataset_contract"
  end

  test "every built-in template remains structurally admissible without executing a solver" do
    templates = WorkflowTemplateCatalog.list()
    assert length(templates) > 0

    for template <- templates do
      assert {:ok, graph} = WorkflowTemplateCatalog.graph_by_id(template["id"])
      assert WorkflowGraphPreflight.validate(graph, %{}) == :ok, template["id"]
    end
  end
end
