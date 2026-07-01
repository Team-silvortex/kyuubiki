defmodule KyuubikiWeb.WorkflowQualityObjectiveRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowGraphRunner
  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes quality objective transforms" do
    assert {:ok, %{"operator" => operator}} =
             WorkflowOperatorCatalog.fetch("transform.compose_quality_objective")

    assert operator["kind"] == "transform"
    assert operator["family"] == "compose_quality_objective"
    assert "objective" in operator["capability_tags"]
    assert "multiphysics" in operator["capability_tags"]

    assert {:ok, %{"operator" => ranking_operator}} =
             WorkflowOperatorCatalog.fetch("transform.rank_quality_candidates")

    assert ranking_operator["kind"] == "transform"
    assert ranking_operator["family"] == "rank_quality_candidates"
    assert "ranking" in ranking_operator["capability_tags"]

    assert {:ok, %{"operator" => next_round_operator}} =
             WorkflowOperatorCatalog.fetch("transform.prepare_quality_next_round_request")

    assert next_round_operator["family"] == "prepare_quality_next_round_request"
    assert "next_round" in next_round_operator["capability_tags"]

    assert {:ok, %{"operator" => sweep_operator}} =
             WorkflowOperatorCatalog.fetch("transform.build_quality_parameter_sweep_plan")

    assert sweep_operator["family"] == "build_quality_parameter_sweep_plan"
    assert "parameter_sweep" in sweep_operator["capability_tags"]

    assert {:ok, %{"operator" => materialize_operator}} =
             WorkflowOperatorCatalog.fetch("transform.materialize_quality_sweep_expansion")

    assert materialize_operator["family"] == "materialize_quality_sweep_expansion"
    assert "materialize" in materialize_operator["capability_tags"]

    assert {:ok, %{"operator" => expand_operator}} =
             WorkflowOperatorCatalog.fetch("transform.expand_parameter_sweep")

    assert expand_operator["family"] == "parameter_sweep"
    assert "parameter_sweep" in expand_operator["capability_tags"]
  end

  test "composes weighted quality scores into a multiphysics objective" do
    payload = %{
      "qualities" => %{
        "thermal" => %{
          "thermal_quality_contract" => "kyuubiki.thermal_quality_score/v1",
          "thermal_quality_score" => 2.0,
          "thermal_quality_grade" => "excellent",
          "thermal_quality_ready" => true,
          "thermal_quality_missing_metric_count" => 0
        },
        "transport" => %{
          "transport_quality_contract" => "kyuubiki.transport_quality_score/v1",
          "transport_quality_score" => 3.0,
          "transport_quality_grade" => "good",
          "transport_quality_ready" => true,
          "transport_quality_missing_metric_count" => 0
        }
      }
    }

    assert {:ok, objective} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_quality_objective",
               payload,
               %{"weights" => %{"thermal" => 2.0, "transport" => 1.0}, "max_ready_score" => 12.0}
             )

    assert objective["composite_quality_contract"] == "kyuubiki.composite_quality_objective/v1"
    assert objective["composite_quality_ready"] == true
    assert objective["composite_quality_grade"] == "good"
    assert objective["composite_quality_term_count"] == 2
    assert objective["composite_quality_score"] == 7.0
  end

  test "blocks composite objective when a domain quality is not ready" do
    payload = %{
      "thermal" => %{
        "thermal_quality_score" => 1.0,
        "thermal_quality_ready" => true,
        "thermal_quality_missing_metric_count" => 0
      },
      "magnetostatic" => %{
        "magnetostatic_quality_score" => 3.0,
        "magnetostatic_quality_grade" => "block",
        "magnetostatic_quality_ready" => false,
        "magnetostatic_quality_missing_metric_count" => 2
      }
    }

    assert {:ok, objective} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_quality_objective",
               payload,
               %{"missing_metric_penalty" => 4.0, "not_ready_penalty" => 20.0}
             )

    assert objective["composite_quality_ready"] == false
    assert objective["composite_quality_grade"] == "block"
    assert objective["composite_quality_missing_metric_count"] == 2
    assert objective["composite_quality_blocked_term_count"] == 1
    assert objective["composite_quality_score"] == 32.0
  end

  test "ranks quality candidates by composite objective readiness and score" do
    payload = %{
      "candidates" => %{
        "candidate_a" => %{
          "qualities" => %{
            "thermal" => %{
              "thermal_quality_score" => 2.0,
              "thermal_quality_ready" => true,
              "thermal_quality_missing_metric_count" => 0
            },
            "cfd" => %{
              "cfd_quality_score" => 5.0,
              "cfd_quality_ready" => true,
              "cfd_quality_missing_metric_count" => 0
            }
          }
        },
        "candidate_b" => %{
          "qualities" => %{
            "thermal" => %{
              "thermal_quality_score" => 1.0,
              "thermal_quality_ready" => true,
              "thermal_quality_missing_metric_count" => 0
            },
            "cfd" => %{
              "cfd_quality_score" => 1.5,
              "cfd_quality_ready" => true,
              "cfd_quality_missing_metric_count" => 0
            }
          }
        },
        "candidate_blocked" => %{
          "qualities" => %{
            "thermal" => %{
              "thermal_quality_score" => 0.5,
              "thermal_quality_ready" => false,
              "thermal_quality_missing_metric_count" => 1
            }
          }
        }
      }
    }

    assert {:ok, ranking} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.rank_quality_candidates",
               payload,
               %{"objective" => %{"weights" => %{"cfd" => 2.0}, "not_ready_penalty" => 20.0}}
             )

    assert ranking["quality_candidate_ranking_contract"] ==
             "kyuubiki.quality_candidate_ranking/v1"

    assert ranking["candidate_count"] == 3
    assert ranking["ready_candidate_count"] == 2
    assert ranking["best_candidate_id"] == "candidate_b"
    assert ranking["best_candidate_ready"] == true
    assert hd(ranking["ranking"])["rank"] == 1
  end

  test "prepares next round request from quality ranking" do
    ranking = %{
      "quality_candidate_ranking_contract" => "kyuubiki.quality_candidate_ranking/v1",
      "ranking" => [
        %{
          "rank" => 1,
          "candidate_id" => "candidate_b",
          "score" => 2.5,
          "ready" => true,
          "objective" => %{"composite_quality_score" => 2.5}
        }
      ]
    }

    assert {:ok, request} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.prepare_quality_next_round_request",
               ranking,
               %{
                 "target_score" => 2.0,
                 "max_candidates" => 12,
                 "search_space" => %{"thickness_mm" => [1.0, 4.0]}
               }
             )

    assert request["quality_next_round_contract"] == "kyuubiki.quality_next_round_request/v1"
    assert request["action"] == "continue"
    assert request["selected_candidate_id"] == "candidate_b"
    assert request["request_payload"]["max_candidates"] == 12.0
  end

  test "builds parameter sweep plan from quality next round request" do
    request = %{
      "quality_next_round_contract" => "kyuubiki.quality_next_round_request/v1",
      "action" => "continue",
      "selected_candidate_id" => "candidate_b",
      "target_score" => 2.0,
      "request_payload" => %{
        "max_candidates" => 12,
        "search_space" => %{
          "elements.0.thickness" => %{"min" => 0.01, "max" => 0.03},
          "material.density" => [2700.0, 7800.0]
        }
      }
    }

    assert {:ok, plan} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.build_quality_parameter_sweep_plan",
               request,
               %{
                 "samples_per_axis" => 3,
                 "id_prefix" => "quality_candidate",
                 "base" => %{"elements" => [%{"thickness" => 0.02}]}
               }
             )

    assert plan["quality_parameter_sweep_plan_contract"] ==
             "kyuubiki.quality_parameter_sweep_plan/v1"

    assert plan["sweep_enabled"] == true
    assert plan["source_candidate_id"] == "candidate_b"
    assert plan["id_prefix"] == "quality_candidate"
    assert plan["case_count_estimate"] == 6
    assert length(plan["axes"]) == 2
  end

  test "materializes parameter sweep expansion payload from quality plan" do
    plan = %{
      "quality_parameter_sweep_plan_contract" => "kyuubiki.quality_parameter_sweep_plan/v1",
      "sweep_enabled" => true,
      "source_candidate_id" => "candidate_b",
      "id_prefix" => "quality_candidate",
      "max_cases" => 12,
      "case_count_estimate" => 2,
      "base" => %{"model" => %{"thickness" => 0.01}},
      "axes" => [%{"label" => "thickness", "path" => "model.thickness", "values" => [0.01, 0.02]}]
    }

    assert {:ok, expansion} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.materialize_quality_sweep_expansion",
               plan,
               %{}
             )

    assert expansion["quality_sweep_expansion_contract"] ==
             "kyuubiki.quality_sweep_expansion/v1"

    assert expansion["expansion_enabled"] == true
    assert expansion["payload"]["base"]["model"]["thickness"] == 0.01
    assert hd(expansion["payload"]["axes"])["path"] == "model.thickness"
    assert expansion["config"]["id_prefix"] == "quality_candidate"
    assert expansion["config"]["max_cases"] == 12.0
  end

  test "expands materialized quality sweep through graph runner" do
    graph = %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.quality-sweep-expansion",
      "name" => "Quality sweep expansion",
      "version" => "1.0.0",
      "entry_nodes" => ["quality_plan"],
      "output_nodes" => ["expanded_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("quality_plan"),
        %{
          "id" => "materialize",
          "kind" => "transform",
          "operator_id" => "transform.materialize_quality_sweep_expansion",
          "config" => %{},
          "inputs" => [port("plan", "report/summary", "quality_parameter_sweep_plan")],
          "outputs" => [port("expansion", "report/summary", "quality_sweep_expansion")]
        },
        %{
          "id" => "expand",
          "kind" => "transform",
          "operator_id" => "transform.expand_parameter_sweep",
          "config" => %{},
          "inputs" => [port("expansion", "report/summary", "quality_sweep_expansion")],
          "outputs" => [port("cases", "report/summary", "parameter_sweep_cases")]
        },
        %{
          "id" => "expanded_output",
          "kind" => "output",
          "inputs" => [port("cases", "report/summary", "parameter_sweep_cases")],
          "outputs" => []
        }
      ],
      "edges" => [
        edge(
          "e0",
          "quality_plan",
          "summary",
          "materialize",
          "plan",
          "quality_parameter_sweep_plan"
        ),
        edge("e1", "materialize", "expansion", "expand", "expansion", "quality_sweep_expansion"),
        edge("e2", "expand", "cases", "expanded_output", "cases", "parameter_sweep_cases")
      ]
    }

    input_artifacts = %{
      "quality_plan" => %{
        "quality_parameter_sweep_plan_contract" => "kyuubiki.quality_parameter_sweep_plan/v1",
        "sweep_enabled" => true,
        "id_prefix" => "quality_candidate",
        "max_cases" => 4,
        "base" => %{
          "elements" => [%{"thickness" => 0.01}],
          "material" => %{"density" => 2700.0}
        },
        "axes" => [
          %{"path" => "elements.0.thickness", "values" => [0.01, 0.02]},
          %{"path" => "material.density", "values" => [2700.0, 7800.0]}
        ]
      }
    }

    assert {:ok, run} =
             WorkflowGraphRunner.run(graph, input_artifacts,
               dataset_contract: graph["dataset_contract"],
               execute_solve: &WorkflowOperatorRuntime.run_solve_operator/3,
               execute_transform: &WorkflowOperatorRuntime.run_transform_operator/3,
               execute_extract: &WorkflowOperatorRuntime.run_extract_operator/3,
               execute_export: &WorkflowOperatorRuntime.run_export_operator/3
             )

    expanded = run["artifacts"]["expand.cases"]
    assert expanded["case_count"] == 4
    assert expanded["axis_count"] == 2
    assert hd(expanded["cases"])["id"] == "quality_candidate_0"
    assert List.last(expanded["cases"])["model"]["material"]["density"] == 7800.0
  end

  test "runs composite quality objective through graph runner with named inputs" do
    graph = %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.multiphysics-quality-objective-json",
      "name" => "Multiphysics quality objective JSON",
      "version" => "1.0.0",
      "entry_nodes" => ["thermal_quality", "transport_quality"],
      "output_nodes" => ["objective_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        input_node("thermal_quality"),
        input_node("transport_quality"),
        %{
          "id" => "compose_objective",
          "kind" => "transform",
          "operator_id" => "transform.compose_quality_objective",
          "config" => %{"weights" => %{"thermal" => 2.0, "transport" => 1.0}},
          "inputs" => [
            port("thermal", "report/summary", "thermal_quality"),
            port("transport", "report/summary", "transport_quality")
          ],
          "outputs" => [port("objective", "report/summary", "composite_quality_objective")]
        },
        %{
          "id" => "objective_output",
          "kind" => "output",
          "inputs" => [port("objective", "report/summary", "composite_quality_objective")],
          "outputs" => []
        }
      ],
      "edges" => [
        edge(
          "e0",
          "thermal_quality",
          "summary",
          "compose_objective",
          "thermal",
          "thermal_quality"
        ),
        edge(
          "e1",
          "transport_quality",
          "summary",
          "compose_objective",
          "transport",
          "transport_quality"
        ),
        edge(
          "e2",
          "compose_objective",
          "objective",
          "objective_output",
          "objective",
          "composite_quality_objective"
        )
      ]
    }

    input_artifacts = %{
      "thermal_quality" => %{
        "thermal_quality_score" => 2.0,
        "thermal_quality_ready" => true,
        "thermal_quality_missing_metric_count" => 0
      },
      "transport_quality" => %{
        "transport_quality_score" => 3.0,
        "transport_quality_ready" => true,
        "transport_quality_missing_metric_count" => 0
      }
    }

    assert {:ok, run} =
             WorkflowGraphRunner.run(graph, input_artifacts,
               dataset_contract: graph["dataset_contract"],
               execute_solve: &WorkflowOperatorRuntime.run_solve_operator/3,
               execute_transform: &WorkflowOperatorRuntime.run_transform_operator/3,
               execute_extract: &WorkflowOperatorRuntime.run_extract_operator/3,
               execute_export: &WorkflowOperatorRuntime.run_export_operator/3
             )

    objective = run["artifacts"]["compose_objective.objective"]
    assert objective["composite_quality_score"] == 7.0
    assert objective["composite_quality_ready"] == true
  end

  defp input_node(id) do
    %{
      "id" => id,
      "kind" => "input",
      "outputs" => [port("summary", "report/summary", id)]
    }
  end

  defp port(id, artifact_type, dataset_value),
    do: %{"id" => id, "artifact_type" => artifact_type, "dataset_value" => dataset_value}

  defp edge(id, from_node, from_port, to_node, to_port, dataset_value) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => "report/summary",
      "dataset_value" => dataset_value
    }
  end
end
