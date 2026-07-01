defmodule KyuubikiWeb.WorkflowOperatorCategoryTaxonomy do
  @moduledoc false

  @categories [
    %{
      "id" => "physics_solver",
      "label" => "Physics Solvers",
      "lane" => "physics",
      "summary" => "Primary FEM/physics solve operators that turn study models into results."
    },
    %{
      "id" => "multiphysics_bridge",
      "label" => "Multiphysics Bridges",
      "lane" => "coupling",
      "summary" =>
        "Cross-domain bridge operators that map one result family into another model family."
    },
    %{
      "id" => "material_research",
      "label" => "Material Research",
      "lane" => "research",
      "summary" =>
        "Material cards, screening, experiment planning, and exploration-loop operators."
    },
    %{
      "id" => "postprocess_extract",
      "label" => "Postprocess Extractors",
      "lane" => "inspection",
      "summary" =>
        "Result extraction operators for summaries, field statistics, peaks, and diagnostics."
    },
    %{
      "id" => "diagnostics_guard",
      "label" => "Diagnostics and Guards",
      "lane" => "safety",
      "summary" =>
        "Diagnostic composition and threshold guard operators used before automation continues."
    },
    %{
      "id" => "optimization_selection",
      "label" => "Optimization and Selection",
      "lane" => "optimization",
      "summary" => "Benchmarking, ranking, scoring, Pareto, and candidate-selection operators."
    },
    %{
      "id" => "automation_loop",
      "label" => "Automation Loop",
      "lane" => "automation",
      "summary" =>
        "Iteration decision, next-round request, and snapshot operators for closed-loop runs."
    },
    %{
      "id" => "data_transform",
      "label" => "Data Transforms",
      "lane" => "dataflow",
      "summary" =>
        "General dataflow transforms for merging, normalizing, selecting, and composing payloads."
    },
    %{
      "id" => "delivery_export",
      "label" => "Delivery and Export",
      "lane" => "delivery",
      "summary" => "Export operators that package workflow results into user-facing artifacts."
    }
  ]

  @category_by_id Map.new(@categories, &{&1["id"], &1})

  def list, do: @categories

  def assign(%{"id" => _operator_id} = descriptor) do
    category = classify(descriptor)

    descriptor
    |> Map.put_new("operator_category", category)
    |> Map.put_new("operator_category_id", category["id"])
  end

  def assign(descriptor), do: descriptor

  def category(category_id) when is_binary(category_id),
    do: Map.get(@category_by_id, category_id, Map.fetch!(@category_by_id, "data_transform"))

  def classify(descriptor) when is_map(descriptor) do
    tags = Map.get(descriptor, "capability_tags", [])
    kind = Map.get(descriptor, "kind", "")
    domain = Map.get(descriptor, "domain", "")
    operator_id = Map.get(descriptor, "id", "")

    category_id =
      cond do
        kind == "solver" ->
          "physics_solver"

        kind == "workflow_bridge" or "workflow_bridge" in tags ->
          "multiphysics_bridge"

        kind == "export" ->
          "delivery_export"

        kind == "extract" ->
          "postprocess_extract"

        "material" in tags ->
          material_category(tags)

        guard_or_diagnostic?(tags) ->
          "diagnostics_guard"

        optimization?(tags) ->
          "optimization_selection"

        automation?(tags) ->
          "automation_loop"

        String.starts_with?(operator_id, "transform.") ->
          "data_transform"

        domain in ["mechanical", "thermal", "electromagnetic", "fluid", "acoustic"] ->
          "physics_solver"

        true ->
          "data_transform"
      end

    category(category_id)
  end

  def classify(_descriptor), do: category("data_transform")

  defp material_category(tags) do
    cond do
      automation?(tags) -> "automation_loop"
      optimization?(tags) -> "optimization_selection"
      true -> "material_research"
    end
  end

  defp guard_or_diagnostic?(tags) do
    Enum.any?(tags, &(&1 in ["diagnostics", "guard", "threshold", "preflight", "unit_check"]))
  end

  defp optimization?(tags) do
    Enum.any?(
      tags,
      &(&1 in [
          "optimization",
          "benchmark",
          "ranking",
          "score",
          "candidate_selection",
          "pareto",
          "multi_objective"
        ])
    )
  end

  defp automation?(tags) do
    Enum.any?(
      tags,
      &(&1 in [
          "automation",
          "iteration_decision",
          "next_round",
          "orchestration",
          "snapshot",
          "state"
        ])
    )
  end
end
