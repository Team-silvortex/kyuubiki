defmodule KyuubikiWeb.TestSupport.WorkflowReportingContract do
  @moduledoc false
  @fixture Path.expand("../../../../tests/fixtures/workflow-report-contract.json", __DIR__)

  def request(hotspots \\ false) do
    request = @fixture |> File.read!() |> Jason.decode!()

    if hotspots do
      request
      |> node("field", &Map.put(&1, "operator_id", "extract.field_hotspots"))
      |> node("guard", &put_in(&1, ["config", "rules", Access.at(0), "field"], "v_hotspot_max"))
    else
      request
    end
  end

  def node(request, id, update) do
    update_in(request, ["graph", "nodes"], fn nodes ->
      Enum.map(nodes, fn node -> if node["id"] == id, do: update.(node), else: node end)
    end)
  end

  def failures(request) do
    [
      {put_in(request, ["input_artifacts", "input", "nodes", Access.at(1), "v"], nil), "field",
       "nodes[1].v"},
      {put_in(request, ["input_artifacts", "extra", "converged"], false), "bundle",
       "payload.extra.converged"},
      {put_in(request, ["input_artifacts", "extra", "diagnostic_node_count"], -1), "bundle",
       "payload.extra.diagnostic_node_count"},
      {node(request, "guard", &put_in(&1, ["config", "rules", Access.at(0), "field"], "missing")),
       "guard", "payload.bundle_payloads.field.missing"},
      {node(
         request,
         "guard",
         &put_in(&1, ["config", "rules", Access.at(0), "severity"], "fatal")
       ), "guard", "config.rules[0].severity"},
      {node(request, "bundle", &put_in(&1, ["config", "include_payloads"], false)), "guard",
       "bundle_payloads"}
    ]
  end
end
