defmodule KyuubikiWeb.WorkflowThermalDecisionIntegrityTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowThermalRuntime, as: Runtime

  @rules [
    %{"field" => "temperature", "threshold" => 100, "severity" => "warn"},
    %{"field" => "flux", "threshold" => 2000, "severity" => "block"}
  ]
  @criteria [%{"field" => "temperature"}, %{"field" => "flux"}]

  test "missing or invalid required guard evidence cannot silently pass or downgrade" do
    for partial <- [%{}, %{"flux" => nil}, %{"flux" => "3000"}, %{"flux" => false}] do
      assert {:error, reason} =
               Runtime.evaluate_thermal_guard(Map.put(partial, "temperature", 140), %{
                 "rules" => @rules
               })

      assert reason =~ "config.rules[1]: payload.flux"
    end

    assert {:ok, %{"guard_status" => "block", "guard_checked_rule_count" => 2}} =
             Runtime.evaluate_thermal_guard(%{"temperature" => 140, "flux" => 3000}, %{
               "rules" => @rules
             })

    assert {:ok, %{"guard_status" => "pass", "guard_checked_rule_count" => 2}} =
             Runtime.evaluate_thermal_guard(%{"temperature" => 0, "flux" => 0}, %{
               "rules" => @rules
             })
  end

  test "malformed guard rules are not removed or silently defaulted" do
    for bad <- [
          nil,
          false,
          %{},
          %{"field" => "flux"},
          %{"field" => "flux", "threshold" => 2, "comparison" => "typo"},
          %{"field" => "flux", "threshold" => 2, "severity" => "typo"}
        ] do
      assert {:error, reason} =
               Runtime.evaluate_thermal_guard(
                 %{"temperature" => 140, "flux" => 3000},
                 %{"rules" => @rules ++ [bad]}
               )

      assert reason =~ "config.rules[2]"
    end
  end

  test "coupled scoring requires every criterion on both candidates" do
    valid = %{"temperature" => 70, "flux" => 3000}

    for side <- ["left", "right"], missing <- [%{}, %{"flux" => nil}, %{"flux" => "3000"}] do
      pair = %{"left" => valid, "right" => valid}
      bad = Map.put(pair, side, Map.put(missing, "temperature", 70))

      assert {:error, reason} =
               Runtime.benchmark_coupled_heat_pair(bad, %{"criteria" => @criteria})

      assert reason =~ "config.criteria[1]: payload.#{side}.flux"
    end

    assert {:ok, %{"benchmark_criteria_count" => 2, "benchmark_winner" => "tie"}} =
             Runtime.benchmark_coupled_heat_pair(
               %{"left" => valid, "right" => valid},
               %{"criteria" => @criteria}
             )
  end

  test "a malformed benchmark entry is not dropped from the denominator" do
    assert {:error, reason} =
             Runtime.benchmark_coupled_heat_pair(
               %{"left" => %{"temperature" => 1}, "right" => %{"temperature" => 2}},
               %{"criteria" => [%{"field" => "temperature"}, nil]}
             )

    assert reason =~ "config.criteria[1]"
  end
end
