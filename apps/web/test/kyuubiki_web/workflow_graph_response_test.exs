defmodule KyuubikiWeb.WorkflowGraphResponseTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowGraphResponse

  test "resolves automatic response modes by graph size" do
    assert %{"response_mode" => "auto-full", "include_node_runs" => true} =
             WorkflowGraphResponse.resolve_options(graph_with_nodes(255), nil)

    assert %{"response_mode" => "auto-compact", "include_node_runs" => false} =
             WorkflowGraphResponse.resolve_options(graph_with_nodes(256), nil)
  end

  test "honors explicit full and compact response modes" do
    assert %{"response_mode" => "full", "include_artifacts" => true} =
             WorkflowGraphResponse.resolve_options(graph_with_nodes(512), %{"response_mode" => "full"})

    assert %{"response_mode" => "compact", "include_artifacts" => false} =
             WorkflowGraphResponse.resolve_options(graph_with_nodes(1), %{"response_mode" => "compact"})
  end

  test "marks partial boolean overrides as custom" do
    assert %{"response_mode" => "custom", "include_artifacts" => false, "include_node_runs" => true} =
             WorkflowGraphResponse.resolve_options(graph_with_nodes(1), %{"include_artifacts" => false})
  end

  defp graph_with_nodes(count) do
    %{"nodes" => Enum.map(1..count, &%{"id" => "node_#{&1}"})}
  end
end
