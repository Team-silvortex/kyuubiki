defmodule KyuubikiWeb.Api.WorkflowDecisionRecoveryApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  @domains ~w(structural acoustic modal dynamic transport electrostatic magnetostatic cfd thermal)

  for domain <- @domains, kind <- ~w(guard benchmark) do
    @domain domain
    @kind kind

    test "#{domain} #{kind}: bad evidence cannot export, and explicit fresh submission recovers" do
      graph = graph(@domain, @kind)
      valid = evidence(@kind)

      for {bad, reason} <- invalid_evidence(@kind) do
        assert_recovery(graph, bad, valid, reason, @kind)
      end
    end

    test "#{domain} #{kind}: damaged aliases and derived samples cannot publish a report" do
      aliased = %{
        "von_mises_peak" => 300,
        "frequencies" => [
          %{"velocity_amplitude" => 1},
          %{"velocity_amplitude" => 3}
        ]
      }

      config =
        if @kind == "guard",
          do: %{
            "rules" => [
              %{"field" => "max_stress", "threshold" => 250},
              %{"field" => "max_velocity", "threshold" => 2, "severity" => "block"}
            ]
          },
          else: %{"criteria" => [%{"field" => "max_stress"}, %{"field" => "max_velocity"}]}

      graph = graph(@domain, @kind, config)

      for {corrupt, path} <- [
            {Map.put(aliased, "peak_stress", nil), "peak_stress"},
            {Map.put(aliased, "frequencies", [%{"velocity_amplitude" => 1}, %{}]),
             "frequencies[1].max_velocity"}
          ] do
        if @kind == "guard" do
          assert_recovery(graph, corrupt, aliased, "payload.#{path}", @kind)
        else
          right = %{"max_stress" => 200, "max_velocity" => 1}

          assert_recovery(
            graph,
            %{"left" => corrupt, "right" => right},
            %{"left" => aliased, "right" => right},
            "payload.left.#{path}",
            @kind
          )
        end
      end
    end
  end

  defp assert_recovery(graph, bad, valid, reason, kind) do
    failed = submit(graph, bad)
    assert failed["job"]["status"] == "failed"
    assert failed["job"]["message"] =~ reason
    refute get_in(failed, ["result", "artifacts", "export.json"])
    assert {:ok, retained} = AnalysisResultStore.get(failed["job"]["job_id"])
    assert retained["_workflow_recovery"]["state"] == "failed"
    refute Map.has_key?(retained["artifacts"], "decision.result")
    refute Map.has_key?(retained["artifacts"], "export.json")
    refute Enum.any?(retained["progress_events"], &(&1["node_id"] in ~w(decision export output)))

    recovered = submit(graph, valid)
    assert recovered["job"]["status"] == "completed", inspect(recovered["job"])
    refute recovered["job"]["job_id"] == failed["job"]["job_id"]
    assert recovered["result"]["failed_nodes"] == []
    assert recovered["result"]["completed_nodes"] == ~w(input decision export output)
    summary = Jason.decode!(recovered["result"]["artifacts"]["export.json"]["content"])

    if kind == "guard" do
      assert summary["guard_status"] == "block"
      assert summary["guard_checked_rule_count"] == 2
    else
      assert summary["benchmark_winner"] == "right"
      assert summary["benchmark_criteria_count"] == 2
    end

    assert {:ok, still_failed} = AnalysisResultStore.get(failed["job"]["job_id"])
    assert still_failed["_workflow_recovery"]["state"] == "failed"
    refute Map.has_key?(still_failed["artifacts"], "export.json")
  end

  defp submit(graph, evidence) do
    response =
      :post
      |> conn(
        "/api/v1/workflows/graph/jobs",
        Jason.encode!(%{
          "graph" => graph,
          "input_artifacts" => %{"input" => evidence}
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert response.status == 202, response.resp_body
    job_id = Jason.decode!(response.resp_body)["job"]["job_id"]
    WorkflowApi.wait_for_job(job_id, @opts, 400)
  end

  defp evidence("guard"), do: %{"temperature" => 140, "flux" => 3000}

  defp evidence("benchmark"),
    do: %{
      "left" => %{"temperature" => 140, "flux" => 3000},
      "right" => %{"temperature" => 120, "flux" => 2000}
    }

  defp invalid_evidence("guard"),
    do: [
      {%{"temperature" => 140}, "config.rules[1]: payload.flux"},
      {Map.put(evidence("guard"), "converged", false), "payload.converged=false"}
    ]

  defp invalid_evidence("benchmark"),
    do: [
      {put_in(evidence("benchmark"), ["right"], %{"temperature" => 120}),
       "config.criteria[1]: payload.right.flux"},
      {put_in(evidence("benchmark"), ["left", "converged"], false),
       "payload.left.converged=false"}
    ]

  defp graph(domain, kind, config \\ nil) do
    benchmark_domain = if domain == "thermal", do: "coupled_heat", else: domain

    operator =
      if kind == "guard",
        do: "transform.evaluate_#{domain}_guard",
        else: "transform.benchmark_#{benchmark_domain}_pair"

    config =
      config ||
        if kind == "guard",
          do: %{
            "rules" => [
              %{"field" => "temperature", "threshold" => 100},
              %{"field" => "flux", "threshold" => 2000, "severity" => "block"}
            ]
          },
          else: %{"criteria" => [%{"field" => "temperature"}, %{"field" => "flux"}]}

    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.#{domain}-#{kind}-recovery",
      "entry_nodes" => ["input"],
      "output_nodes" => ["output"],
      "nodes" => [
        %{"id" => "input", "kind" => "input", "outputs" => [port("summary")]},
        %{
          "id" => "decision",
          "kind" => "transform",
          "operator_id" => operator,
          "config" => config,
          "inputs" => [port("payload")],
          "outputs" => [port("result")]
        },
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
      ],
      "edges" => [
        edge("e0", "input", "summary", "decision", "payload"),
        edge("e1", "decision", "result", "export", "summary"),
        edge("e2", "export", "json", "output", "json", "export/json")
      ]
    }
  end

  defp port(id, type \\ "report/summary"), do: %{"id" => id, "artifact_type" => type}

  defp edge(id, from, from_port, to, to_port, type \\ "report/summary"),
    do: %{
      "id" => id,
      "from" => %{"node" => from, "port" => from_port},
      "to" => %{"node" => to, "port" => to_port},
      "artifact_type" => type
    }
end
