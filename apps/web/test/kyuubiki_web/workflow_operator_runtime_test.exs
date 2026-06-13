defmodule KyuubikiWeb.WorkflowOperatorRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowCatalogSupport
  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  defmodule StubSolveClient do
    def solve_electrostatic_bar_1d(payload),
      do: {:ok, %{"solver" => "electrostatic_bar_1d", "payload" => payload}}

    def solve_heat_plane_triangle_2d(payload),
      do: {:ok, %{"solver" => "heat_plane_triangle_2d", "payload" => payload}}

    def solve_frame_3d(payload), do: {:ok, %{"solver" => "frame_3d", "payload" => payload}}
  end

  setup do
    original = Application.get_env(:kyuubiki_web, WorkflowOperatorRuntime, [])

    Application.put_env(
      :kyuubiki_web,
      WorkflowOperatorRuntime,
      Keyword.put(original, :solve_runtime_client, StubSolveClient)
    )

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, original)
    end)

    :ok
  end

  test "dispatches supported solve operators through the configured runtime client" do
    assert {:ok, %{"solver" => "electrostatic_bar_1d", "payload" => %{"model" => 1}}} =
             WorkflowOperatorRuntime.run_solve_operator(
               "solve.electrostatic_bar_1d",
               %{"model" => 1}
             )

    assert {:ok, %{"solver" => "heat_plane_triangle_2d", "payload" => %{"model" => 2}}} =
             WorkflowOperatorRuntime.run_solve_operator(
               "solve.heat_plane_triangle_2d",
               %{"model" => 2}
             )

    assert {:ok, %{"solver" => "frame_3d", "payload" => %{"model" => 3}}} =
             WorkflowOperatorRuntime.run_solve_operator("solve.frame_3d", %{"model" => 3})
  end

  test "rejects unsupported solve operators" do
    assert {:error, {:unsupported_workflow_solve_operator, "solve.unknown"}} =
             WorkflowOperatorRuntime.run_solve_operator("solve.unknown", %{})
  end

  test "catalog exposes newly wired workflow solve operators" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "solve.electrostatic_bar_1d")
    assert MapSet.member?(operators, "solve.heat_plane_triangle_2d")
    assert MapSet.member?(operators, "solve.frame_3d")
  end

  test "catalog exposes triangle bridge operators that local chains depend on" do
    operators = WorkflowOperatorCatalog.list() |> Enum.map(& &1["id"]) |> MapSet.new()

    assert MapSet.member?(operators, "bridge.electrostatic_field_to_heat_triangle_2d")
    assert MapSet.member?(operators, "bridge.temperature_field_to_thermo_triangle_2d")
    assert MapSet.member?(operators, "transform.compare_summary_pair")
    assert MapSet.member?(operators, "extract.field_statistics")
    assert MapSet.member?(operators, "export.alert_markdown")
  end

  test "bridges electrostatic triangle results into a heat triangle seed model" do
    electrostatic_result = %{
      "nodes" => [
        %{"id" => "e0", "x" => 0.0, "y" => 0.0},
        %{"id" => "e1", "x" => 1.0, "y" => 0.0},
        %{"id" => "e2", "x" => 0.0, "y" => 1.0}
      ],
      "elements" => [
        %{
          "id" => "et0",
          "node_i" => 0,
          "node_j" => 1,
          "node_k" => 2,
          "electric_field_magnitude" => 4.0
        }
      ]
    }

    config = %{
      "seed_model" => %{
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "heat_load" => 0.0},
          %{"id" => "h2", "x" => 0.0, "y" => 1.0, "heat_load" => 0.0}
        ],
        "elements" => [%{"id" => "ht0", "node_i" => 0, "node_j" => 1, "node_k" => 2}]
      },
      "contract" =>
        WorkflowCatalogSupport.electrostatic_to_heat_bridge_contract_example(10.0, [
          "node_i",
          "node_j",
          "node_k"
        ])
    }

    assert {:ok, %{"nodes" => nodes}} =
             WorkflowOperatorRuntime.run_transform_operator(
               "bridge.electrostatic_field_to_heat_triangle_2d",
               electrostatic_result,
               config
             )

    assert Enum.map(nodes, & &1["heat_load"]) == [40.0, 40.0, 40.0]
  end

  test "compares summary pairs through the transform runtime" do
    payload = %{
      "left" => %{"max_stress" => 10.0, "max_temperature" => 40.0},
      "right" => %{"max_stress" => 13.0, "max_temperature" => 44.0}
    }

    assert {:ok, compared} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compare_summary_pair",
               payload,
               %{"left_prefix" => "mechanical", "right_prefix" => "thermal"}
             )

    assert compared["mechanical_max_stress"] == 10.0
    assert compared["thermal_max_stress"] == 13.0
    assert compared["delta_max_stress"] == 3.0
    assert compared["ratio_max_stress"] == 1.3
    assert compared["summary_shared_numeric_field_count"] == 2
  end

  test "extracts field statistics and hotspots from numeric result collections" do
    payload = %{
      "nodes" => [
        %{"id" => "n0", "temperature" => 20.0},
        %{"id" => "n1", "temperature" => 50.0},
        %{"id" => "n2", "temperature" => 80.0}
      ],
      "elements" => [
        %{"id" => "e0", "electric_field_magnitude" => 2.0},
        %{"id" => "e1", "electric_field_magnitude" => 5.0},
        %{"id" => "e2", "electric_field_magnitude" => 9.0}
      ]
    }

    assert {:ok, stats} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.field_statistics",
               payload,
               %{"source" => "nodes", "field" => "temperature", "percentiles" => [50]}
             )

    assert stats["temperature_min"] == 20.0
    assert stats["temperature_max"] == 80.0
    assert stats["temperature_p50"] == 50.0

    assert {:ok, hotspots} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.field_hotspots",
               payload,
               %{"field" => "electric_field_magnitude", "threshold" => 5.0}
             )

    assert hotspots["electric_field_magnitude_hotspot_count"] == 2
    assert hotspots["electric_field_magnitude_hotspot_ids"] == ["e2", "e1"]
  end

  test "exports hotspot summaries as markdown alerts" do
    payload = %{
      "field_hotspot_count" => 2,
      "field_hotspot_samples" => [
        %{"id" => "e2", "electric_field_magnitude" => 9.0},
        %{"id" => "e1", "electric_field_magnitude" => 5.0}
      ]
    }

    assert {:ok, exported} =
             WorkflowOperatorRuntime.run_export_operator(
               "export.alert_markdown",
               payload,
               %{"title" => "Electrostatic Hotspots", "severity" => "critical"}
             )

    assert exported["format"] == "markdown"
    assert String.contains?(exported["content"], "# Electrostatic Hotspots")
    assert String.contains?(exported["content"], "- Severity: critical")
    assert String.contains?(exported["content"], "## Sample Context")
  end
end
