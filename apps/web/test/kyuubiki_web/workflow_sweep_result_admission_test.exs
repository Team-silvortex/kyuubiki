defmodule KyuubikiWeb.WorkflowSweepResultAdmissionTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowParameterSweepRuntime, as: Runtime

  test "keyed results never borrow another case position or stale summaries" do
    assert {:ok, joined} = join([%{"case_id" => "b", "summary" => %{"mass" => 2}}])
    assert joined["join_complete"] == false
    assert joined["missing_case_ids"] == ["a"]
    assert joined["joined_summary_count"] == 1
    refute Map.has_key?(hd(joined["cases"]), "summary")
    refute Map.has_key?(hd(joined["cases"]), "result")
    assert List.last(joined["cases"])["summary"]["mass"] == 2
    assert {:error, _} = Runtime.summarize_parameter_sweep(joined, %{})
    assert {:error, _} = join([%{"case_id" => "b", "summary" => %{}}], %{"strict" => true})
  end

  test "unmatched keyed results cannot be silently ignored even beside complete evidence" do
    results = good_results() ++ [%{"case_id" => "foreign", "summary" => %{"mass" => 0}}]
    assert {:ok, joined} = join(results)
    assert joined["unmatched_result_ids"] == ["foreign"]
    refute joined["join_complete"]
    assert {:error, _} = Runtime.summarize_parameter_sweep(joined, %{})
    assert {:error, _} = join(results, %{"strict" => true})
  end

  for results <- [
        [%{"case_id" => "a", "summary" => %{}}, %{"id" => "a", "summary" => %{}}],
        [%{"case_id" => "a", "id" => "b", "summary" => %{}}],
        [%{"case_id" => nil, "summary" => %{}}],
        [%{"case_id" => " ", "summary" => %{}}],
        [%{"case_id" => "a", "summary" => %{}}, %{"mass" => 2}],
        [%{"mass" => 1}],
        [%{"mass" => 1}, %{"mass" => 2}, %{"mass" => 3}],
        [nil, %{"mass" => 2}]
      ] do
    @results results
    test "ambiguous result identity rejects #{inspect(results)}" do
      assert {:error, _} = join(@results)
    end
  end

  test "positional mode requires a complete unlabelled batch and exposes its mode" do
    assert {:ok, joined} = join([%{"mass" => 1}, %{"summary" => %{"mass" => 2}}])
    assert joined["matching_mode"] == "position"
    assert joined["join_complete"]
    assert Enum.map(joined["cases"], & &1["summary"]["mass"]) == [1, 2]
  end

  for result <- [
        %{"status" => "failed", "summary" => %{"mass" => 0}},
        %{"status" => "pending", "summary" => %{"mass" => 0}},
        %{"status" => nil, "summary" => %{"mass" => 0}},
        %{"status" => "ok", "error" => "failed", "summary" => %{"mass" => 0}},
        %{"summary" => nil, "result" => %{"mass" => 0}},
        %{"summary" => %{"mass" => 0, "converged" => false}},
        %{"status" => "ok"}
      ] do
    @result result
    test "unusable result becomes explicit rejection #{inspect(result)}" do
      assert {:ok, joined} = join([Map.put(@result, "case_id", "a")])
      assert joined["joined_summary_count"] == 0
      assert joined["rejected_result_count"] == 1
      assert [%{"case_id" => "a", "reason" => reason}] = joined["rejected_results"]
      assert is_binary(reason)
      assert reason != ""
      refute Map.has_key?(hd(joined["cases"]), "summary")
    end
  end

  test "custom summary selection is explicit and cannot replace identity or provenance" do
    for field <- ~w(id case_id caseId model parameters metadata result_status result_error) do
      assert {:error, _} = join(good_results(), %{"output_field" => field})
    end

    config = %{"summary_field" => "quality", "output_field" => "fresh", "strict" => true}
    assert {:error, _} = join(good_results(), config)
    results = Enum.map(good_results(), &Map.put(&1, "quality", &1["summary"]))
    assert {:ok, joined} = join(results, config)
    assert joined["joined_summary_field"] == "fresh"
    assert {:ok, summary} = Runtime.summarize_parameter_sweep(joined, %{})
    assert Enum.map(summary["rows"], & &1["mass"]) == [1, 2]

    assert {:error, _} =
             Runtime.summarize_parameter_sweep(joined, %{"summary_field" => "summary"})
  end

  test "malformed collections counts and options never become defaults" do
    for config <- [[], %{"strict" => nil}, %{"strict" => "false"}, %{"summary_field" => nil}] do
      assert {:error, _} = join(good_results(), config)
    end

    for extra <- [%{"summaries" => nil}, %{"case_count" => 1}, %{"case_count" => 2.5}] do
      assert {:error, _} =
               Runtime.join_parameter_sweep_results(
                 Map.merge(%{"cases" => cases(), "results" => good_results()}, extra),
                 %{}
               )
    end

    for cases <- [[%{"id" => "a"}, %{"id" => "a"}], [%{}, %{"id" => "case_0"}], [nil]] do
      assert {:error, _} =
               Runtime.join_parameter_sweep_results(%{"cases" => cases, "results" => []}, %{})
    end
  end

  test "fresh results replace old failures while preserving metadata and exact identity" do
    cases = [
      %{
        "caseId" => " a ",
        "result_error" => "old",
        "result_status" => "failed",
        "metadata" => %{"round" => 1},
        "parameters" => %{"x" => 2}
      }
    ]

    assert {:ok, joined} =
             Runtime.join_parameter_sweep_results(
               %{
                 "cases" => cases,
                 "results" => [
                   %{
                     "case_id" => " a ",
                     "status" => "ok",
                     "error" => nil,
                     "summary" => %{"mass" => 3}
                   }
                 ]
               },
               %{}
             )

    assert {:ok, table} = Runtime.summarize_parameter_sweep(joined, %{})
    assert hd(table["rows"])["case_id"] == " a "
    assert hd(table["rows"])["metadata"] == %{"round" => 1}
    refute Map.has_key?(hd(joined["cases"]), "result_error")
  end

  test "large shuffled results preserve one-to-one matching" do
    cases = for i <- 0..4095, do: %{"id" => "c#{i}"}
    results = for i <- 4095..0//-1, do: %{"case_id" => "c#{i}", "summary" => %{"mass" => i}}

    assert {:ok, joined} =
             Runtime.join_parameter_sweep_results(%{"cases" => cases, "results" => results}, %{
               "strict" => true
             })

    assert joined["join_complete"]
    assert Enum.map(joined["cases"], & &1["summary"]["mass"]) == Enum.to_list(0..4095)
  end

  test "diagnostic rejection cannot leak non-JSON result envelopes" do
    for extra <- [
          %{"status" => :failed},
          %{"error" => {:failed, 1}},
          %{"summary" => %{"mass" => :unknown}}
        ] do
      assert {:error, _} = join([Map.merge(%{"case_id" => "a"}, extra)])
    end
  end

  defp join(results, config \\ %{}),
    do: Runtime.join_parameter_sweep_results(%{"cases" => cases(), "results" => results}, config)

  defp cases,
    do: [%{"id" => "a", "summary" => %{"mass" => 0}, "result" => %{"mass" => 0}}, %{"id" => "b"}]

  defp good_results,
    do: [
      %{"case_id" => "b", "summary" => %{"mass" => 2}},
      %{"case_id" => "a", "summary" => %{"mass" => 1}}
    ]
end
