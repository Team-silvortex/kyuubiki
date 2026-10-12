defmodule KyuubikiWeb.WorkflowQualityIntegrityTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowDomainQualityRuntime, as: Quality
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  for {domain, field, other} <- [
        {"structural", "max_displacement", "max_stress"},
        {"thermal", "thermal_temperature_max", "thermal_flux_peak_magnitude"},
        {"acoustic", "max_sound_pressure_level_db", "max_acoustic_intensity"},
        {"modal", "total_mass", "frequency_span_hz"},
        {"dynamic", "max_displacement", "max_force"},
        {"electrostatic", "electrostatic_field_peak_magnitude",
         "electrostatic_peak_energy_density"},
        {"magnetostatic", "magnetostatic_field_peak_magnitude",
         "magnetostatic_flux_peak_magnitude"},
        {"cfd", "cfd_divergence_error_peak", "cfd_reynolds_number_peak"},
        {"transport", "transport_total_flux_peak_magnitude", "transport_peclet_peak"}
      ] do
    @domain domain
    @field field
    @other other
    @config %{"enabled_terms" => [field], "targets" => %{field => 1}, "weights" => %{field => 1}}

    test "#{domain}: explicit nonconvergence cannot become a ready quality score" do
      for marker <- [
            %{"converged" => false},
            %{"converged" => nil},
            %{"converged" => "true"},
            %{"stability_result" => nil},
            %{"stability_result" => %{"converged" => false}}
          ] do
        rejects(@domain, Map.put(marker, @field, 0.1), @config, "payload")
      end
    end

    test "#{domain}: selected terms cannot be dropped defaulted or duplicated" do
      for terms <- [
            nil,
            false,
            [],
            %{},
            [@field, "unknown"],
            [@field, nil],
            [@field, " "],
            [@field, @field],
            [@field, " #{@field} "]
          ] do
        rejects(
          @domain,
          %{@field => 0.1},
          Map.put(@config, "enabled_terms", terms),
          "config.enabled_terms"
        )
      end
    end

    test "#{domain}: every declared override is validated even for unselected terms" do
      for group <- ~w(targets weights), bad <- [nil, false, [], "default"] do
        rejects(@domain, %{@field => 1}, Map.put(@config, group, bad), "config.#{group}")
      end

      for {group, invalid} <- [
            {"targets", [nil, false, "1", 0, -1]},
            {"weights", [nil, false, "1", -1]}
          ],
          bad <- invalid,
          field <- [@field, @other] do
        rejects(
          @domain,
          %{@field => 1},
          put_in(@config, [group, field], bad),
          "config.#{group}.#{field}"
        )
      end

      for group <- ~w(targets weights) do
        rejects(
          @domain,
          %{@field => 1},
          put_in(@config, [group, "unknown"], 1),
          "config.#{group}.unknown"
        )
      end

      for bad <- [nil, "8", false, -1] do
        rejects(
          @domain,
          %{@field => 1},
          Map.put(@config, "max_ready_score", bad),
          "config.max_ready_score"
        )
      end
    end

    test "#{domain}: damaged metrics are errors but truly absent metrics remain blocking reports" do
      for value <- [nil, "1", false, [], %{}, Integer.pow(10, 400)] do
        rejects(@domain, %{@field => value}, @config, "payload.#{@field}")
      end

      assert {:ok, report} = run(@domain, %{}, @config)
      assert report["#{@domain}_quality_grade"] == "block"
      refute report["#{@domain}_quality_ready"]
      assert report["#{@domain}_quality_missing_metric_count"] == 1
      assert [%{"status" => "missing"}] = report["#{@domain}_quality_blocking_terms"]
    end

    test "#{domain}: positive tiny targets and explicit zero weights are not silently rewritten" do
      config = put_in(@config, ["targets", @field], 1.0e-15)
      assert {:ok, report} = run(@domain, %{@field => 1.0e-15}, config)
      assert report["#{@domain}_quality_score"] == 1
      assert hd(report["#{@domain}_quality_terms"])["target"] == 1.0e-15

      config = config |> put_in(["weights", @field], 0) |> Map.put("max_ready_score", 0)
      assert {:ok, report} = run(@domain, %{@field => 1.0e308}, config)
      assert report["#{@domain}_quality_score"] == 0
      assert report["#{@domain}_quality_ready"]
      assert report["#{@domain}_quality_watch_count"] == 1
      rejects(@domain, %{@field => nil}, config, "payload.#{@field}")
      assert {:ok, missing} = run(@domain, %{}, config)
      refute missing["#{@domain}_quality_ready"]
    end

    test "#{domain}: ratio penalty and total overflow return contextual errors" do
      rejects(
        @domain,
        %{@field => 1.0e308},
        put_in(@config, ["targets", @field], 0.1),
        "#{@field}.ratio"
      )

      rejects(
        @domain,
        %{@field => 1.0e308},
        put_in(@config, ["weights", @field], 2),
        "#{@field}.penalty"
      )

      config = %{
        "enabled_terms" => [@field, @other],
        "targets" => %{@field => 1, @other => 1},
        "weights" => %{@field => 1, @other => 1}
      }

      rejects(@domain, %{@field => 1.0e308, @other => 1.0e308}, config, "quality_score")
    end

    test "#{domain}: valid term order scores grade boundaries and last ties are stable" do
      config = %{
        "enabled_terms" => [" #{@other} ", @field],
        "targets" => %{@field => 2, @other => 2},
        "weights" => %{@field => 2, @other => 4},
        "max_ready_score" => 8
      }

      for {value, grade} <- [{0.5, "excellent"}, {1, "good"}, {2, "review"}, {3, "block"}] do
        assert {:ok, report} = run(@domain, %{@field => value, @other => value}, config)
        assert report["#{@domain}_quality_score"] == 3 * value
        assert report["#{@domain}_quality_grade"] == grade
        assert Enum.map(report["#{@domain}_quality_terms"], & &1["field"]) == [@other, @field]
        assert report["#{@domain}_quality_dominant_term"]["field"] == @other
      end

      config = put_in(config, ["weights", @other], 2)
      assert {:ok, report} = run(@domain, %{@field => 1, @other => 1}, config)
      assert report["#{@domain}_quality_dominant_term"]["field"] == @field
    end

    test "#{domain}: malformed public inputs reject without function clause crashes" do
      operator = "transform.score_#{@domain}_quality"

      for bad <- [nil, [], false, 0] do
        assert {:error, _} = Quality.score(operator, bad, @config)
        assert {:error, _} = Runtime.run_transform_operator(operator, bad, @config)
      end

      for bad <- [[], false, "default"] do
        assert {:error, _} = Quality.score(operator, %{@field => 1}, bad)
        assert {:error, _} = Runtime.run_transform_operator(operator, %{@field => 1}, bad)
      end

      assert run(@domain, %{}, nil) == run(@domain, %{}, %{})
    end
  end

  test "canonical corruption and incomplete derived samples are not disguised as missing metrics" do
    for {domain, field, payload, path} <- [
          {"structural", "max_stress", %{"max_stress" => nil, "von_mises_peak" => 0},
           "payload.max_stress"},
          {"dynamic", "max_displacement", %{"frequencies" => [%{"max_displacement" => 1}, %{}]},
           "frequencies[1]"},
          {"modal", "frequency_span_hz", %{"min_frequency_hz" => 1, "max_frequency_hz" => nil},
           "max_frequency_hz"}
        ] do
      assert {:error, reason} =
               Runtime.run_transform_operator("transform.score_#{domain}_quality", payload, %{
                 "enabled_terms" => [field]
               })

      assert reason =~ path
    end
  end

  test "inverse goals preserve the native ratio while checking overflow" do
    for {domain, field} <- [
          {"structural", "stiffness_margin"},
          {"modal", "min_frequency_hz"},
          {"dynamic", "peak_frequency_hz"},
          {"acoustic", "total_damping_loss"},
          {"electrostatic", "electrostatic_potential_span"}
        ] do
      config = %{
        "enabled_terms" => [field],
        "targets" => %{field => 20},
        "weights" => %{field => 2}
      }

      operator = "transform.score_#{domain}_quality"
      assert {:ok, report} = Runtime.run_transform_operator(operator, %{field => 40}, config)
      assert report["#{domain}_quality_score"] == 1

      assert {:error, reason} =
               Runtime.run_transform_operator(
                 operator,
                 %{field => 1.0e-12},
                 put_in(config, ["targets", field], 1.0e308)
               )

      assert reason =~ "#{field}.ratio"
    end
  end

  defp run(domain, payload, config),
    do: Runtime.run_transform_operator("transform.score_#{domain}_quality", payload, config)

  defp rejects(domain, payload, config, path) do
    assert {:error, reason} = run(domain, payload, config)
    assert is_binary(reason)
    assert reason =~ path
  end
end
