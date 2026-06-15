defmodule KyuubikiSdk.WorkflowResultsTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.WorkflowResults

  defp load_graph do
    "../../../schemas/examples.workflow-graph.json"
    |> Path.expand(__DIR__)
    |> File.read!()
    |> Jason.decode!()
  end

  test "builds output manifest from graph" do
    assert {:ok, manifest} = WorkflowResults.build_output_manifest(load_graph())
    assert manifest["graph_id"] == "workflow.heat-to-thermo-quad-2d"
    assert hd(manifest["outputs"])["key"] == "thermo_summary.result"
  end

  test "validates result payload with artifact_type fallback" do
    assert {:ok, validated} =
             WorkflowResults.validate_result_against_graph(load_graph(), %{
               "result" => %{
                 "workflow_id" => "workflow.demo",
                 "run_id" => "run-1",
                 "status" => "completed",
                 "current_node" => "solve_thermo",
                 "completed_nodes" => ["input", "solve_thermo"],
                 "progress_events" => [%{"node_id" => "solve_thermo", "status" => "completed"}],
                 "artifacts" => %{
                   "result/thermal_plane_quad_2d" => %{
                     "artifact_id" => "artifact.thermo.result",
                     "artifact_type" => "result/thermal_plane_quad_2d",
                     "dataset_value" => "thermo_result"
                   }
                 }
               }
             })

    assert get_in(validated, ["artifacts", "thermo_summary.result", "artifact_id"]) ==
             "artifact.thermo.result"
    assert get_in(validated, ["workflow_runtime", "run_id"]) == "run-1"
  end

  test "rejects missing required output artifact" do
    assert {:error, error} =
             WorkflowResults.validate_result_against_graph(load_graph(), %{
               "result" => %{"artifacts" => %{}}
             })

    assert error.message =~ "thermo_summary.result"
  end

  test "normalizes workflow runtime" do
    assert {:ok, runtime} =
             WorkflowResults.normalize_runtime(%{
               "result" => %{
                 "workflow_id" => "workflow.demo",
                 "run_id" => "run-1",
                 "status" => "running",
                 "current_node" => "solve",
                 "completed_nodes" => ["input"],
                 "progress_events" => [%{"node_id" => "input", "status" => "completed"}]
               }
             })

    assert runtime["current_node"] == "solve"
  end

  test "normalizes workflow progression" do
    assert {:ok, progression} =
             WorkflowResults.normalize_progression(
               [
                 %{
                   "job" => %{
                     "job_id" => "job-1",
                     "status" => "running",
                     "progress" => 0.5,
                     "current_node" => "solve",
                     "completed_nodes" => ["input"],
                     "progress_events" => [%{"node_id" => "input", "status" => "completed"}]
                   }
                 }
               ],
               %{
                 "result" => %{
                   "workflow_id" => "workflow.demo",
                   "run_id" => "run-1",
                   "status" => "completed",
                   "current_node" => "output",
                   "completed_nodes" => ["input", "solve", "output"],
                   "progress_events" => [%{"node_id" => "output", "status" => "completed"}],
                   "artifacts" => %{}
                 }
               }
             )

    assert hd(progression["snapshots"])["current_node"] == "solve"
    assert progression["latest"]["run_id"] == "run-1"
  end
end
