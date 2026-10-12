defmodule KyuubikiWeb.WorkflowSweepAdmissionTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowQualityObjectiveRuntime, as: Quality
  alias KyuubikiWeb.WorkflowParameterSweepRuntime, as: Sweep

  for {name, config} <- [
        {"null case limit", %{"max_cases" => nil}},
        {"fractional case limit", %{"max_cases" => 2.2}},
        {"negative case limit", %{"max_cases" => -1}},
        {"fractional sample count", %{"samples_per_axis" => 2.2}},
        {"bad sample count", %{"samples_per_axis" => "3"}},
        {"blank prefix", %{"id_prefix" => " "}}
      ] do
    test "plan rejects #{name}" do
      assert {:error, reason} =
               Quality.build_quality_parameter_sweep_plan(
                 request(),
                 unquote(Macro.escape(config))
               )

      assert is_binary(reason)
    end
  end

  for spec <- [[], nil, %{}, %{"values" => "bad"}, %{"min" => 0, "max" => nil}] do
    test "bad axis #{inspect(spec)} cannot disappear beside a valid axis" do
      payload =
        put_in(
          request(),
          ["request_payload", "search_space", "broken"],
          unquote(Macro.escape(spec))
        )

      assert {:error, reason} = Quality.build_quality_parameter_sweep_plan(payload, %{})
      assert reason =~ "broken"
    end
  end

  test "incomplete ranking disables planning even when search space is valid" do
    payload =
      Map.merge(request(), %{
        "source_ranking_complete" => false,
        "source_rejected_candidates" => [%{"candidate_id" => "bad", "error" => "missing"}]
      })

    assert {:ok, plan} = Quality.build_quality_parameter_sweep_plan(payload, %{})
    refute plan["sweep_enabled"]
    assert plan["sweep_blocking_reason"] == "candidate_evaluation_incomplete"
    assert {:ok, expansion} = Quality.materialize_quality_sweep_expansion(plan, %{})
    refute expansion["expansion_enabled"]
    assert {:ok, skipped} = Sweep.expand_parameter_sweep(expansion, %{})
    assert skipped["cases"] == []
    refute skipped["expansion_enabled"]

    assert {:error, _} =
             Quality.compose_quality_execution_batch(skipped, %{
               "operator_id" => "transform.evaluate_material_thermal_shock"
             })
  end

  test "case budget is checked before allocating range samples and cannot be bypassed downstream" do
    payload =
      put_in(request(), ["request_payload", "search_space"], %{
        "x" => %{"min" => -1.0e308, "max" => 1.0e308}
      })

    assert {:ok, plan} =
             Quality.build_quality_parameter_sweep_plan(payload, %{
               "samples_per_axis" => 1_000_000_000,
               "max_cases" => 2
             })

    assert plan["sweep_budget"]["case_budget_exceeded"]
    assert hd(plan["axes"])["values_deferred"]
    refute Map.has_key?(hd(plan["axes"]), "values")

    assert {:ok, expansion} =
             Quality.materialize_quality_sweep_expansion(plan, %{"max_cases" => 2_000_000_000})

    refute expansion["expansion_budget_ready"]

    assert {:ok, blocked} =
             Sweep.expand_parameter_sweep(expansion, %{"max_cases" => 2_000_000_000})

    assert blocked["cases"] == []
    refute blocked["expansion_budget_ready"]
  end

  test "large opposite-sign range uses finite interpolation" do
    payload =
      put_in(request(), ["request_payload", "search_space"], %{
        "x" => %{"min" => -1.0e308, "max" => 1.0e308}
      })

    assert {:ok, plan} = Quality.build_quality_parameter_sweep_plan(payload, %{})
    assert hd(plan["axes"])["values"] == [-1.0e308, 0.0, 1.0e308]
  end

  test "plan materialization validates present enable flag" do
    assert {:error, reason} =
             Quality.materialize_quality_sweep_expansion(
               %{"sweep_enabled" => "false", "axes" => [axis("x", [1])], "base" => %{}},
               %{}
             )

    assert reason =~ "sweep_enabled"
  end

  for path <- ["x..y", "list.-1.v", "list.+0.v", ".x", "x."] do
    test "expansion rejects ambiguous path #{path}" do
      payload = %{
        "base" => %{"x" => %{"y" => 0}, "list" => [%{"v" => 1}]},
        "axes" => [axis(unquote(path), [2])]
      }

      assert {:error, reason} = Sweep.expand_parameter_sweep(payload, %{})
      assert is_binary(reason)
    end
  end

  test "overlapping paths, aliases and duplicate labels cannot overwrite parameters" do
    for axes <- [
          [axis("list.0.v", [1]), axis("list.00.v", [2])],
          [axis("list.0", [%{"v" => 1}]), axis("list.0.v", [2])],
          [Map.put(axis("x", [1]), "label", "same"), Map.put(axis("y", [2]), "label", "same")]
        ] do
      assert {:error, _} =
               Sweep.expand_parameter_sweep(
                 %{"base" => %{"list" => [%{"v" => 0}]}, "axes" => axes},
                 %{}
               )
    end
  end

  test "array leaves including null and structured values are assignable" do
    payload = %{
      "base" => %{"list" => [nil]},
      "axes" => [axis("list.0", [%{"v" => 1}, [2]])],
      "case_metadata" => %{"round" => 4}
    }

    assert {:ok, expanded} = Sweep.expand_parameter_sweep(payload, %{})
    assert Enum.map(expanded["cases"], & &1["model"]["list"]) == [[%{"v" => 1}], [[2]]]
    assert Enum.all?(expanded["cases"], &(&1["metadata"] == %{"round" => 4}))
  end

  for config <- [%{"max_cases" => 1.5}, %{"id_prefix" => nil}] do
    test "expansion rejects malformed config #{inspect(config)}" do
      assert {:error, _} =
               Sweep.expand_parameter_sweep(
                 %{"base" => %{}, "axes" => [axis("x", [1])]},
                 unquote(Macro.escape(config))
               )
    end
  end

  test "explicit null payload axes cannot fall back to config axes" do
    assert {:error, _} =
             Sweep.expand_parameter_sweep(%{"base" => %{}, "axes" => nil}, %{
               "axes" => [axis("x", [1])]
             })
  end

  defp request,
    do: %{
      "action" => "continue",
      "base" => %{"x" => 0},
      "request_payload" => %{"search_space" => %{"x" => [1, 2]}}
    }

  defp axis(path, values), do: %{"path" => path, "values" => values}

  test "case count overflow is rejected before case generation" do
    axes = for index <- 1..64, do: axis("x#{index}", [0, 1])
    assert {:error, reason} = Sweep.expand_parameter_sweep(%{"base" => %{}, "axes" => axes}, %{})
    assert reason =~ "case_count_estimate"
  end

  test "zero case budget returns blocked diagnostics but never an executable sweep" do
    assert {:ok, plan} =
             Quality.build_quality_parameter_sweep_plan(request(), %{"max_cases" => 0})

    assert plan["sweep_budget"]["case_budget_exceeded"]
    assert {:ok, expansion} = Quality.materialize_quality_sweep_expansion(plan, %{})
    refute expansion["expansion_budget_ready"]

    assert {:error, _} =
             Sweep.expand_parameter_sweep(%{"base" => %{}, "axes" => [axis("x", [1])]}, %{
               "max_cases" => 0
             })
  end

  test "explicit axis cap is visible instead of silently narrowing search" do
    input = put_in(request(), ["request_payload", "search_space", "y"], [3, 4])
    assert {:ok, plan} = Quality.build_quality_parameter_sweep_plan(input, %{"max_axes" => 1})
    assert plan["sweep_budget"]["axis_budget_truncated"]
    assert plan["sweep_budget"]["usable_axis_count"] == 2
    assert plan["sweep_budget"]["planned_axis_count"] == 1
    assert plan["sweep_budget"]["recommendation"] == "schedule_followup_axis_batch"
  end

  test "stopped exploration remains a disabled result through expansion" do
    assert {:ok, plan} = Quality.build_quality_parameter_sweep_plan(%{"action" => "stop"}, %{})
    assert {:ok, expansion} = Quality.materialize_quality_sweep_expansion(plan, %{})
    assert {:ok, skipped} = Sweep.expand_parameter_sweep(expansion, %{})
    assert skipped["sweep_action"] == "stop"
    assert skipped["case_count"] == 0
    refute skipped["sweep_enabled"]
  end

  test "materialization refuses inconsistent plan counts" do
    assert {:ok, plan} = Quality.build_quality_parameter_sweep_plan(request(), %{})

    assert {:error, reason} =
             Quality.materialize_quality_sweep_expansion(
               Map.put(plan, "case_count_estimate", 1),
               %{}
             )

    assert reason =~ "case_count_estimate"
  end

  test "quality expansion rejects malformed wrappers rather than raising" do
    for {key, value} <- [
          {"quality_sweep_expansion_contract", "unknown"},
          {"config", []},
          {"expansion_enabled", nil},
          {"expansion_budget_ready", "false"}
        ] do
      envelope = %{
        "quality_sweep_expansion_contract" => "kyuubiki.quality_sweep_expansion/v1",
        "payload" => %{"base" => %{}, "axes" => [axis("x", [1])]},
        "config" => %{}
      }

      assert {:error, reason} = Sweep.expand_parameter_sweep(Map.put(envelope, key, value), %{})
      assert is_binary(reason)
    end
  end

  test "last axis varies fastest and numeric object keys remain distinct" do
    assert {:ok, expanded} =
             Sweep.expand_parameter_sweep(
               %{
                 "base" => %{"keys" => %{}},
                 "axes" => [axis("keys.0", [1, 2]), axis("keys.00", [3, 4])]
               },
               %{}
             )

    assert Enum.map(expanded["cases"], & &1["model"]["keys"]) == [
             %{"0" => 1, "00" => 3},
             %{"0" => 1, "00" => 4},
             %{"0" => 2, "00" => 3},
             %{"0" => 2, "00" => 4}
           ]
  end

  test "plan metadata cannot smuggle non-JSON values into generated artifacts" do
    payload =
      put_in(request(), ["request_payload", "seed_metadata"], %{"bad" => {:internal, :tuple}})

    assert {:error, reason} = Quality.build_quality_parameter_sweep_plan(payload, %{})
    assert reason =~ "seed_metadata"
  end
end
