defmodule KyuubikiWeb.WorkflowDomainDecisionIntegrityTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  @domains ~w(structural acoustic modal dynamic transport electrostatic magnetostatic cfd thermal)
  @rules [
    %{"field" => "temperature", "threshold" => 100},
    %{"field" => "flux", "threshold" => 2000, "severity" => "block"}
  ]
  @criteria [%{"field" => "temperature"}, %{"field" => "flux"}]
  @valid %{"temperature" => 140, "flux" => 3000}

  for domain <- @domains do
    @domain domain

    test "#{domain}: missing guard evidence cannot pass or downgrade a decision" do
      for partial <- [%{}, %{"flux" => nil}, %{"flux" => "3000"}, %{"flux" => false}] do
        assert_error(
          guard(@domain, Map.put(partial, "temperature", 140), @rules),
          "config.rules[1]",
          "payload.flux"
        )
      end
    end

    test "#{domain}: every rule is validated instead of filtered or defaulted" do
      for bad <- [
            nil,
            false,
            %{},
            %{"field" => "flux"},
            %{"field" => "flux", "threshold" => nil, "value" => 1},
            %{"field" => "flux", "threshold" => 2, "value" => 1},
            %{"field" => "flux", "threshold" => 2, "comparison" => "typo"},
            %{"field" => "flux", "threshold" => 2, "severity" => "typo"},
            %{"field" => "flux", "threshold" => 2, "label" => false}
          ] do
        assert_error(guard(@domain, @valid, @rules ++ [bad]), "config.rules[2]")
      end
    end

    test "#{domain}: failed and malformed convergence markers cannot become successful decisions" do
      for marker <- [
            %{"converged" => false},
            %{"converged" => "true"},
            %{"stability_result" => %{"converged" => false}},
            %{"stability_result" => nil}
          ] do
        assert_error(guard(@domain, Map.merge(@valid, marker), @rules), "payload")

        pair = %{"left" => @valid, "right" => @valid}
        assert_error(benchmark(@domain, Map.merge(pair, marker)), "payload")

        for side <- ~w(left right) do
          assert_error(
            benchmark(@domain, Map.update!(pair, side, &Map.merge(&1, marker))),
            "payload.#{side}"
          )
        end
      end
    end

    test "#{domain}: partial pair evidence never shrinks the scoring denominator" do
      for side <- ~w(left right), partial <- [%{}, %{"flux" => nil}, %{"flux" => "3000"}] do
        pair =
          Map.put(
            %{"left" => @valid, "right" => @valid},
            side,
            Map.put(partial, "temperature", 140)
          )

        assert_error(benchmark(@domain, pair), "config.criteria[1]", "payload.#{side}.flux")
      end
    end

    test "#{domain}: malformed criteria and explicit invalid options cannot disappear" do
      for bad <- [
            nil,
            false,
            %{},
            %{"field" => "flux", "left_field" => nil},
            %{"field" => "flux", "right_field" => " "},
            %{"field" => "flux", "goal" => "typo"},
            %{"field" => "flux", "weight" => 0},
            %{"field" => "flux", "weight" => -1},
            %{"field" => "flux", "weight" => "2"}
          ] do
        assert_error(
          benchmark(@domain, %{"left" => @valid, "right" => @valid}, %{
            "criteria" => @criteria ++ [bad]
          }),
          "config.criteria[2]"
        )
      end
    end

    test "#{domain}: invalid labels cannot overwrite candidate scores or masquerade as a tie" do
      for labels <- [
            %{"left_label" => "same", "right_label" => "same"},
            %{"left_label" => "tie"},
            %{"right_label" => "TIE"},
            %{"left_label" => nil},
            %{"right_label" => " "}
          ] do
        assert_error(
          benchmark(@domain, %{"left" => @valid, "right" => @valid}, labels),
          "config."
        )
      end
    end

    test "#{domain}: an invalid canonical metric is not replaced by a favorable alias" do
      assert_error(
        guard(@domain, %{"max_stress" => nil, "von_mises_peak" => 0}, [
          %{"field" => "max_stress", "threshold" => 250}
        ]),
        "payload.max_stress"
      )
    end

    test "#{domain}: arithmetic overflow returns an error instead of crashing the workflow" do
      assert_error(
        benchmark(
          @domain,
          %{"left" => %{"flux" => -1.0e308}, "right" => %{"flux" => 1.0e308}},
          %{"criteria" => [%{"field" => "flux"}]}
        ),
        "non-finite"
      )

      assert_error(
        benchmark(
          @domain,
          %{"left" => %{"flux" => 0}, "right" => %{"flux" => 1}},
          %{"criteria" => List.duplicate(%{"field" => "flux", "weight" => 1.0e308}, 2)}
        ),
        "non-finite"
      )
    end

    test "#{domain}: valid zeros, defaults and aliases preserve all checked evidence" do
      assert {:ok, %{"guard_status" => "pass", "guard_checked_rule_count" => 2}} =
               guard(@domain, %{"temperature" => 0, "flux" => 0}, @rules)

      assert {:ok, %{"guard_status" => "block", "guard_checked_rule_count" => 2}} =
               guard(@domain, @valid, @rules)

      assert {:ok, %{"guard_status" => "warn"}} =
               guard(@domain, %{"von_mises_peak" => 300}, [
                 %{"field" => "max_stress", "value" => 250}
               ])

      assert {:ok, %{"benchmark_criteria_count" => 2, "benchmark_winner" => "tie"}} =
               benchmark(@domain, %{"left" => @valid, "right" => @valid})
    end

    test "#{domain}: custom fields and trimmed choices are evaluated rather than defaulted" do
      assert {:ok, %{"guard_status" => "block"}} =
               guard(@domain, %{"flux" => 0}, [
                 %{
                   "field" => " flux ",
                   "threshold" => 1,
                   "comparison" => " lt ",
                   "severity" => " block "
                 }
               ])

      assert {:ok, result} =
               benchmark(
                 @domain,
                 %{"left" => %{"a" => 3}, "right" => %{"b" => 2}},
                 %{
                   "left_label" => "baseline",
                   "right_label" => "candidate",
                   "criteria" => [
                     %{
                       "left_field" => " a ",
                       "right_field" => " b ",
                       "goal" => " max ",
                       "weight" => 2
                     }
                   ]
                 }
               )

      assert result["benchmark_winner"] == "baseline"
      assert result["baseline_score"] == 2.0
      assert result["candidate_score"] == 0.0
      assert result["benchmark_criteria_count"] == 1
    end
  end

  test "domain label normalization must not collapse distinct candidates" do
    for domain <- @domains -- ["cfd", "thermal"] do
      assert_error(
        benchmark(domain, %{"left" => @valid, "right" => @valid}, %{
          "left_label" => "Candidate A",
          "right_label" => "Candidate-A"
        }),
        "distinct"
      )
    end
  end

  test "empty declarations and malformed candidate objects are rejected" do
    for domain <- @domains, entries <- [nil, [], %{}] do
      assert_error(guard(domain, @valid, entries), "config.rules")

      assert_error(
        benchmark(domain, %{"left" => @valid, "right" => @valid}, %{"criteria" => entries}),
        "config.criteria"
      )
    end

    for domain <- @domains, side <- ~w(left right), bad <- [nil, [], false, 0] do
      assert_error(
        benchmark(domain, Map.put(%{"left" => @valid, "right" => @valid}, side, bad)),
        "payload.#{side}"
      )
    end
  end

  test "unrepresentable integers return contract errors instead of escaping as arithmetic exceptions" do
    huge = Integer.pow(10, 400)

    for domain <- @domains do
      assert_error(
        guard(domain, %{"flux" => huge}, [%{"field" => "flux", "threshold" => 1}]),
        "payload.flux",
        "non-finite"
      )

      assert_error(
        guard(domain, %{"flux" => 0}, [%{"field" => "flux", "threshold" => huge}]),
        "config.rules[0].threshold",
        "non-finite"
      )
    end
  end

  defp guard(domain, payload, rules) do
    Runtime.run_transform_operator("transform.evaluate_#{domain}_guard", payload, %{
      "rules" => rules
    })
  end

  defp benchmark(domain, payload, config \\ %{}) do
    domain = if domain == "thermal", do: "coupled_heat", else: domain

    Runtime.run_transform_operator(
      "transform.benchmark_#{domain}_pair",
      payload,
      Map.merge(%{"criteria" => @criteria}, config)
    )
  end

  defp assert_error(result, path, detail \\ "") do
    assert {:error, reason} = result
    assert is_binary(reason)
    assert reason =~ path
    assert reason =~ detail
  end
end
