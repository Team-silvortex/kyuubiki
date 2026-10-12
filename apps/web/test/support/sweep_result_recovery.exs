defmodule KyuubikiWeb.TestSupport.SweepResultRecovery do
  @moduledoc false
  import ExUnit.Assertions
  alias KyuubikiWeb.TestSupport.QualitySweepRecovery, as: Execution
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor, as: Executor
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  def valid do
    %{
      "cases" => [
        %{"id" => "a", "summary" => %{"mass" => 0}, "metadata" => %{"round" => 1}},
        %{"id" => "b", "summary" => %{"mass" => 0}, "metadata" => %{"round" => 1}}
      ],
      "results" => [
        %{"case_id" => "b", "status" => "ok", "summary" => %{"mass" => 2}},
        %{"case_id" => "a", "status" => "ok", "summary" => %{"mass" => 1}}
      ]
    }
  end

  def corruptions do
    input = valid()

    [
      {"missing", Map.update!(input, "results", &Enum.take(&1, 1)), "summary", %{}},
      {"failed",
       Map.update!(
         input,
         "results",
         &Enum.map(&1, fn row -> Map.put(row, "status", "failed") end)
       ), "summary", %{}},
      {"foreign",
       Map.update!(
         input,
         "results",
         &(&1 ++ [%{"case_id" => "foreign", "summary" => %{"mass" => 0}}])
       ), "summary", %{}},
      {"duplicate", Map.update!(input, "results", &(&1 ++ [hd(&1)])), "join", %{}},
      {"missing_field",
       Map.update!(input, "results", fn [first, second] ->
         [Map.put(first, "summary", %{}), second]
       end), "summary", %{}},
      {"invalid_objective", input, "score",
       %{"score" => %{"objectives" => [%{"field" => "mass", "weight" => -1}]}}},
      {"overflow", input, "score",
       %{"score" => %{"objectives" => [%{"field" => "mass", "weight" => 1.0e308}]}}}
    ]
  end

  def graph(policy \\ "fail", overrides \\ %{}) do
    stages = [
      {"join", "join_parameter_sweep_results", %{}},
      {"summary", "summarize_parameter_sweep", %{"fields" => ["mass"]}},
      {"score", "score_parameter_sweep", %{"objectives" => [%{"field" => "mass"}]}}
    ]

    nodes =
      [%{"id" => "input", "kind" => "input", "outputs" => [port("result")]}] ++
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
        end) ++
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
      "id" => "workflow.sweep-results-#{policy}",
      "entry_nodes" => ["input"],
      "output_nodes" => ["output"],
      "nodes" => nodes,
      "edges" => edges
    }
  end

  def actual_batch do
    assert {:ok, plan} =
             transform("build_quality_parameter_sweep_plan", Execution.request(), %{
               "base" => Execution.base()
             })

    assert {:ok, expansion} = transform("materialize_quality_sweep_expansion", plan, %{})
    assert {:ok, cases} = transform("expand_parameter_sweep", expansion, %{})

    assert {:ok, batch} =
             transform("compose_quality_execution_batch", cases, %{
               "operator_id" => "transform.evaluate_material_thermal_shock",
               "operator_config" => %{"constraint_factor" => 0.7}
             })

    batch = batch |> Jason.encode!() |> Jason.decode!()
    assert {:ok, execution} = Executor.execute_batch(batch)
    Execution.assert_execution(execution)

    # The caller explicitly maps actual operator assessments to the generic metric table.
    results =
      Enum.map(execution["results"], fn entry ->
        assessment = hd(entry["result"]["material_thermal_shock_assessments"])

        %{
          "case_id" => entry["case_id"],
          "status" => entry["status"],
          "summary" => %{
            "stress" => assessment["thermal_shock_estimated_stress"],
            "safety" => assessment["thermal_shock_safety_factor"]
          }
        }
      end)

    {Map.put(cases, "results", Enum.reverse(results)),
     %{
       "summary" => %{"fields" => ["stress", "safety"]},
       "score" => %{
         "objectives" => [
           %{"field" => "stress", "goal" => "min", "max_allowed" => 1.2e8},
           %{"field" => "safety", "weight" => 0, "min_allowed" => 2}
         ]
       }
     }}
  end

  def assert_actual(report) do
    assert report["scored_count"] == 2
    assert report["feasible_count"] == 1
    assert report["best"]["case_id"] == "quality_round_0"
    assert report["best"]["metadata"]["seed_metadata"] == %{"round" => 1}
    assert_in_delta report["best"]["stress"], 70.0e9 * 1.2e-5 * 120 * 0.7 / 0.67, 1.0e-6
  end

  defp transform(name, payload, config),
    do: Runtime.run_transform_operator("transform.#{name}", payload, config)

  defp port(id, type \\ "report/summary"), do: %{"id" => id, "artifact_type" => type}
end
