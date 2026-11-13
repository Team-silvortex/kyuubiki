defmodule KyuubikiWeb.WorkflowHeatSeedInputTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.{WorkflowOperatorRuntime, WorkflowGraphInputs, WorkflowGraphPreflight}

  test "separate seed data maps temperature and rejects conflicting or missing inputs" do
    heat = %{
      "nodes" => [%{"id" => "h", "x" => 0.0, "y" => 0.0, "temperature" => 40.0}],
      "input" => %{"elements" => []},
      "elements" => []
    }

    seed = %{
      "nodes" => [%{"id" => "s", "x" => 0.0, "y" => 0.0, "fix_x" => true}],
      "elements" => []
    }

    config = %{
      "contract" => %{
        "source" => %{"distribution" => "node_to_node", "field" => "temperature"},
        "transform" => %{"reference_temperature" => 20.0}
      }
    }

    for shape <- ["quad", "triangle"] do
      id = "bridge.temperature_field_to_thermo_#{shape}_2d"
      payload = %{"heat_result" => heat, "seed_model" => seed}

      assert {:ok, expected} =
               WorkflowOperatorRuntime.run_transform_operator(
                 id,
                 heat,
                 Map.put(config, "seed_model", seed)
               )

      assert {:ok, ^expected} =
               WorkflowOperatorRuntime.run_transform_operator(id, payload, config)

      assert hd(expected["nodes"])["temperature_delta"] == 20.0
      assert hd(expected["nodes"])["fix_x"]

      assert {:ok, result} =
               KyuubikiWeb.Orchestra.Engine.run_workflow_graph(%{
                 "graph" => graph(id, config),
                 "input_artifacts" => %{"heat" => heat, "seed" => seed}
               })

      assert result["artifacts"]["out.model"] == expected
      lineage = Enum.find(result["artifact_lineage"], &(&1["artifact_key"] == "bridge.model"))
      assert lineage["source_artifacts"] == ["heat.payload", "seed.payload"]

      for {bad_payload, bad_config} <- [
            {Map.put(payload, "unexpected", %{}), config},
            {Map.put(payload, "seed_model", nil), config},
            {payload, seed},
            {payload, nil}
          ] do
        assert {:error, _} =
                 WorkflowOperatorRuntime.run_transform_operator(id, bad_payload, bad_config)
      end

      assert {:error, _} =
               WorkflowOperatorRuntime.run_transform_operator(
                 id,
                 payload,
                 Map.put(config, "seed_model", seed)
               )

      for field <- ["heat_result", "seed_model"] do
        assert {:error, _} =
                 WorkflowOperatorRuntime.run_transform_operator(
                   id,
                   Map.delete(payload, field),
                   config
                 )
      end

      assert WorkflowGraphInputs.named_inputs?(%{
               "kind" => "transform",
               "operator_id" => id,
               "inputs" => [%{"id" => "heat_result"}, %{"id" => "seed_model"}]
             })

      refute WorkflowGraphInputs.named_inputs?(%{
               "kind" => "transform",
               "operator_id" => id,
               "inputs" => [%{"id" => "legacy_result"}]
             })
    end
  end

  defp graph(operator \\ "bridge.temperature_field_to_thermo_quad_2d", config \\ %{}) do
    port = fn id -> %{"id" => id, "artifact_type" => "artifact/json"} end

    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "seed-budget",
      "entry_nodes" => ["heat", "seed"],
      "output_nodes" => ["out"],
      "nodes" => [
        %{"id" => "heat", "kind" => "input", "outputs" => [port.("payload")]},
        %{"id" => "seed", "kind" => "input", "outputs" => [port.("payload")]},
        %{
          "id" => "bridge",
          "kind" => "transform",
          "operator_id" => operator,
          "inputs" => [port.("heat_result"), port.("seed_model")],
          "outputs" => [port.("model")],
          "config" => config
        },
        %{"id" => "out", "kind" => "output", "inputs" => [port.("model")]}
      ],
      "edges" =>
        Enum.map(
          [
            {"heat", "payload", "bridge", "heat_result"},
            {"seed", "payload", "bridge", "seed_model"},
            {"bridge", "model", "out", "model"}
          ],
          fn {from, fp, to, tp} ->
            %{
              "id" => "#{from}-#{to}",
              "artifact_type" => "artifact/json",
              "from" => %{"node" => from, "port" => fp},
              "to" => %{"node" => to, "port" => tp}
            }
          end
        )
    }
  end

  test "large seeds use the data budget without weakening configuration or artifact limits" do
    graph = graph()
    seed = %{"nodes" => List.duplicate(0, 20_001)}
    inputs = %{"heat" => %{}, "seed" => seed}
    assert :ok = WorkflowGraphPreflight.validate(graph, inputs)

    config_graph =
      update_in(graph["nodes"], fn nodes ->
        Enum.map(nodes, fn node ->
          if node["id"] == "bridge",
            do: Map.put(node, "config", %{"seed_model" => seed}),
            else: node
        end)
      end)

    assert {:error, {:invalid_workflow_graph, reason}} =
             WorkflowGraphPreflight.validate(config_graph, inputs)

    assert reason =~ "config"

    assert {:error, {:invalid_workflow_graph, _}} =
             WorkflowGraphPreflight.validate(
               graph,
               Map.put(inputs, "seed", %{"nodes" => List.duplicate(0, 500_001)})
             )
  end
end
