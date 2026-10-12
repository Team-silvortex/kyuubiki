defmodule KyuubikiWeb.WorkflowQualitySweepRequestAdmissionTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @fixtures Path.expand(
              "../../../../tests/fixtures/workflow-quality-sweep-request-admission.json",
              __DIR__
            )
            |> File.read!()
            |> Jason.decode!()

  for fixture <- @fixtures do
    test "direct sweep request: #{fixture["id"]}" do
      fixture = unquote(Macro.escape(fixture))
      payload = fixture["payload"] |> Jason.encode!() |> Jason.decode!()

      result =
        Runtime.run_transform_operator(
          "transform.build_quality_parameter_sweep_plan",
          payload,
          %{"search_space" => %{"temperature" => [1, 2]}}
        )

      if fixture["error"] do
        assert {:error, reason} = result
        assert is_binary(reason)
      else
        assert {:ok, plan} = result
        for {key, expected} <- fixture["expected"], do: assert(plan[key] == expected)
      end
    end
  end

  test "625 sweep alias placements preserve evidence through expansion" do
    reports = [
      nil,
      %{"coupled_readiness_state" => "ready"},
      %{"coupled_readiness_state" => "warn"},
      %{"coupled_readiness_state" => "block"},
      %{"coupled_readiness_ready" => false}
    ]

    for a <- 0..4, b <- 0..4, c <- 0..4, d <- 0..4 do
      distinct = [a, b, c, d] |> Enum.reject(&(&1 == 0)) |> Enum.uniq() |> Enum.sort()
      selected = List.first(distinct) || 0

      hint =
        case selected do
          2 -> %{"action" => "review_coupled_readiness"}
          n when n >= 3 -> %{"action" => "fix_coupled_readiness"}
          _ -> nil
        end

      payload =
        %{
          "action" => if(selected >= 3, do: "replan", else: "continue"),
          "selected_iteration_hint" => hint,
          "selected_coupled_readiness" => Enum.at(reports, b),
          "coupled_readiness" => Enum.at(reports, c),
          "request_payload" => %{
            "coupled_readiness" => Enum.at(reports, a),
            "seed_objective" => %{"coupled_readiness" => Enum.at(reports, d)},
            "optimization_hint" => hint,
            "search_space" => %{"temperature" => [1, 2]}
          }
        }
        |> Jason.encode!()
        |> Jason.decode!()

      result =
        Runtime.run_transform_operator(
          "transform.build_quality_parameter_sweep_plan",
          payload,
          %{}
        )

      if length(distinct) > 1 do
        assert {:error, _} = result
      else
        assert {:ok, plan} = result
        assert plan["coupled_readiness"] == Enum.at(reports, selected)

        assert {:ok, materialized} =
                 Runtime.run_transform_operator(
                   "transform.materialize_quality_sweep_expansion",
                   plan,
                   %{}
                 )

        assert {:ok, cases} =
                 Runtime.run_transform_operator(
                   "transform.expand_parameter_sweep",
                   materialized,
                   %{}
                 )

        assert cases["case_count"] == 2

        for item <- cases["cases"],
            do: assert(item["metadata"]["coupled_readiness"] == Enum.at(reports, selected))
      end
    end
  end
end
