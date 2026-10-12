defmodule KyuubikiWeb.WorkflowCoupledReadinessAdmissionTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @fixtures Path.expand(
              "../../../../tests/fixtures/workflow-coupled-readiness-admission.json",
              __DIR__
            )
  @cases @fixtures |> File.read!() |> Jason.decode!()

  for fixture <- @cases do
    @fixture fixture
    test "coupled readiness admission: #{fixture["id"]}" do
      result =
        Runtime.run_transform_operator(
          "transform.prepare_quality_next_round_request",
          @fixture["payload"],
          @fixture["config"]
        )

      if @fixture["error"] do
        assert {:error, _} = result
      else
        assert {:ok, result} = result
        for {key, value} <- @fixture["expected"], do: assert(result[key] == value)
      end
    end
  end

  test "alias placement and JSON exchange preserve coupled readiness decisions" do
    reports =
      Enum.map(~w(all_null row_ready row_warning row_block row_false_only), fn id ->
        fixture = Enum.find(@cases, &(&1["id"] == id))
        hd(fixture["payload"]["ranking"])["coupled_readiness"]
      end)

    for row <- 0..4, objective <- 0..4, payload <- 0..4 do
      distinct = [row, objective, payload] |> Enum.reject(&(&1 == 0)) |> Enum.uniq()

      input =
        %{
          "ranking" => [
            %{
              "candidate_id" => "a",
              "score" => 1,
              "ready" => true,
              "coupled_readiness" => Enum.at(reports, row),
              "objective" => %{"coupled_readiness" => Enum.at(reports, objective)}
            }
          ],
          "coupled_readiness" => Enum.at(reports, payload)
        }
        |> Jason.encode!()
        |> Jason.decode!()

      result =
        Runtime.run_transform_operator("transform.prepare_quality_next_round_request", input, %{})

      if length(distinct) > 1 do
        assert {:error, _} = result
      else
        assert {:ok, decision} = result
        selected = List.first(distinct) || 0

        expected =
          case selected do
            2 -> "continue"
            value when value in [3, 4] -> "replan"
            _ -> "stop"
          end

        assert decision["action"] == expected
        assert decision["selected_coupled_readiness"] == Enum.at(reports, selected)
        assert decision["request_payload"]["coupled_readiness"] == Enum.at(reports, selected)

        if selected >= 2 do
          assert decision["selected_iteration_hint"]["action"] ==
                   if(selected == 2,
                     do: "review_coupled_readiness",
                     else: "fix_coupled_readiness"
                   )
        end
      end
    end
  end
end
