defmodule KyuubikiWeb.WorkflowTemplateOperatorCoverageTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowTemplateCatalog

  test "catalog workflow graphs only reference cataloged operators" do
    missing =
      WorkflowTemplateCatalog.list()
      |> Enum.flat_map(&workflow_operator_refs/1)
      |> Enum.uniq()
      |> Enum.reject(fn {_workflow_id, operator_id} ->
        match?({:ok, %{"operator" => _operator}}, WorkflowOperatorCatalog.fetch(operator_id))
      end)

    assert missing == []
  end

  defp workflow_operator_refs(%{"id" => workflow_id}) do
    case WorkflowTemplateCatalog.graph_by_id(workflow_id) do
      {:ok, %{"nodes" => nodes}} when is_list(nodes) ->
        nodes
        |> Enum.flat_map(fn
          %{"operator_id" => operator_id} when is_binary(operator_id) ->
            [{workflow_id, operator_id}]

          _node ->
            []
        end)

      _error ->
        []
    end
  end
end
