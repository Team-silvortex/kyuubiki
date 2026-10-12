defmodule KyuubikiWeb.TestSupport.QualitySweepRecovery do
  @moduledoc false
  import ExUnit.Assertions
  alias KyuubikiWeb.Orchestra.OperatorTaskIR

  def request do
    %{
      "action" => "continue",
      "selected_candidate_id" => "seed",
      "request_payload" => %{
        "seed_metadata" => %{"round" => 1},
        "search_space" => %{"temperature_delta" => [120.0, 160.0]},
        "max_candidates" => 4
      }
    }
  end

  def base,
    do: %{
      "temperature_delta" => 1.0,
      "thermal_expansion" => 1.2e-5,
      "youngs_modulus" => 70.0e9,
      "poisson_ratio" => 0.33,
      "yield_strength" => 320.0e6
    }

  def graph(policy \\ "fail", overrides \\ %{}) do
    stages = [
      {"plan", "build_quality_parameter_sweep_plan", %{"base" => base()}},
      {"materialize", "materialize_quality_sweep_expansion", %{}},
      {"expand", "expand_parameter_sweep", %{}},
      {"batch", "compose_quality_execution_batch",
       %{
         "operator_id" => "transform.evaluate_material_thermal_shock",
         "operator_config" => %{"constraint_factor" => 0.7}
       }}
    ]

    transforms =
      Enum.map(stages, fn {id, operator, config} ->
        %{
          "id" => id,
          "kind" => "transform",
          "operator_id" => "transform.#{operator}",
          "config" =>
            config |> Map.put("on_error", policy) |> Map.merge(Map.get(overrides, id, %{})),
          "inputs" => [port("payload")],
          "outputs" => [port("result")]
        }
      end)

    nodes =
      [%{"id" => "input", "kind" => "input", "outputs" => [port("result")]}] ++
        transforms ++
        [
          %{
            "id" => "export",
            "kind" => "export",
            "operator_id" => "export.summary_json",
            "config" => %{},
            "inputs" => [port("summary")],
            "outputs" => [port("json", "export/json")]
          },
          %{
            "id" => "output",
            "kind" => "output",
            "inputs" => [port("json", "export/json")],
            "outputs" => []
          }
        ]

    edges =
      nodes
      |> Enum.chunk_every(2, 1, :discard)
      |> Enum.with_index()
      |> Enum.map(fn {[from, to], index} ->
        output = hd(from["outputs"])

        %{
          "id" => "e#{index}",
          "from" => %{"node" => from["id"], "port" => output["id"]},
          "to" => %{"node" => to["id"], "port" => hd(to["inputs"])["id"]},
          "artifact_type" => output["artifact_type"]
        }
      end)

    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.quality-sweep-#{policy}",
      "entry_nodes" => ["input"],
      "output_nodes" => ["output"],
      "nodes" => nodes,
      "edges" => edges
    }
  end

  def assert_batch(batch) do
    assert batch["task_count"] == 2
    assert Enum.map(batch["tasks"], & &1["case_id"]) == ["quality_round_0", "quality_round_1"]

    for entry <- batch["tasks"] do
      assert entry["task_digest"] == OperatorTaskIR.compute_task_digest(entry["task_ir"])
      assert entry["metadata"]["seed_metadata"] == %{"round" => 1}
      assert entry["agent_rpc"]["params"]["task_ir"] == entry["task_ir"]
    end
  end

  def assert_execution(execution) do
    assert execution["executed_count"] == 2
    assert execution["error_count"] == 0
    assert execution["blocked_count"] == 0

    for {entry, delta} <- Enum.zip(execution["results"], [120.0, 160.0]) do
      assert entry["status"] == "ok"
      assessment = hd(entry["result"]["material_thermal_shock_assessments"])
      expected_stress = 70.0e9 * 1.2e-5 * delta * 0.7 / 0.67
      assert_in_delta assessment["thermal_shock_estimated_stress"], expected_stress, 1.0e-6

      assert_in_delta assessment["thermal_shock_safety_factor"],
                      320.0e6 / expected_stress,
                      1.0e-12
    end
  end

  defp port(id, type \\ "report/summary"), do: %{"id" => id, "artifact_type" => type}
end
