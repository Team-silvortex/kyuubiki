defmodule KyuubikiWeb.WorkflowTemplateCatalog do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowCatalogQuery
  alias KyuubikiWeb.WorkflowTemplateRegistry

  def list(filters \\ %{}) do
    normalized_filters = WorkflowCatalogQuery.normalize_filters(filters)

    WorkflowTemplateRegistry.list()
    |> Enum.map(&WorkflowCatalogSupport.enrich_workflow_descriptor/1)
    |> Enum.filter(&WorkflowCatalogQuery.matches_workflow?(&1, normalized_filters))
  end

  def fetch(workflow_id) when is_binary(workflow_id) do
    case WorkflowTemplateRegistry.fetch(workflow_id) do
      nil -> {:error, {:workflow_not_found, workflow_id}}
      workflow -> {:ok, %{"workflow" => WorkflowCatalogSupport.enrich_workflow_descriptor(workflow)}}
    end
  end

  def graph_by_id(workflow_id) do
    case WorkflowTemplateRegistry.graph_by_id(workflow_id) do
      {:ok, graph} -> {:ok, WorkflowCatalogSupport.enrich_workflow_graph(graph)}
      error -> error
    end
  end
end
