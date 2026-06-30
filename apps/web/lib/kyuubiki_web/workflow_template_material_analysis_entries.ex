defmodule KyuubikiWeb.WorkflowTemplateMaterialAnalysisEntries do
  @moduledoc false

  alias KyuubikiWeb.WorkflowCatalogSupport

  def list do
    [material_experiment_result_entry()]
  end

  def result_input_node do
    %{
      "id" => "results_input",
      "kind" => "input",
      "outputs" => [port("results", "report/summary_collection", "material_experiment_results")]
    }
  end

  def analyze_node do
    %{
      "id" => "analyze_results",
      "kind" => "transform",
      "operator_id" => "transform.analyze_material_experiment_results",
      "config" => %{},
      "inputs" => [port("results", "report/summary_collection", "material_experiment_results")],
      "outputs" => [
        port("analysis", "report/summary", "material_experiment_result_analysis")
      ]
    }
  end

  def output_node do
    %{
      "id" => "json_output",
      "kind" => "output",
      "inputs" => [port("json", "export/json", "summary_json")],
      "outputs" => []
    }
  end

  def edge(id, from_node, from_port, to_node, to_port, dataset_value) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => artifact_type_for(dataset_value),
      "dataset_value" => dataset_value
    }
  end

  def port(id, artifact_type, dataset_value),
    do: %{"id" => id, "artifact_type" => artifact_type, "dataset_value" => dataset_value}

  def value(id, data_class, semantic_type, element_type \\ "json_object"),
    do:
      WorkflowCatalogSupport.workflow_dataset_value_info(
        id,
        data_class,
        semantic_type,
        element_type
      )

  def experiment_result_values do
    [
      value("material_experiment_results", "result", "report/summary_collection"),
      value("material_experiment_result_analysis", "result", "report/summary"),
      value("summary_json", "export", "export/json", "utf8_text")
    ]
  end

  def artifact_type_for("summary_json"), do: "export/json"
  def artifact_type_for("material_experiment_results"), do: "report/summary_collection"
  def artifact_type_for(_dataset_value), do: "report/summary"

  defp material_experiment_result_entry do
    %{
      "id" => "workflow.material-experiment-result-analysis-json",
      "name" => "Material experiment result analysis JSON",
      "version" => "1.0.0",
      "summary" =>
        "Analyzes observed material experiment results against planned scores and exports validated candidate rankings.",
      "domains" => ["material"],
      "capability_tags" => [
        "material",
        "optimization",
        "experiment_result",
        "validation",
        "candidate_selection",
        "headless_safe"
      ],
      "graph" => graph(),
      "entry_inputs" => [
        %{"node_id" => "results_input", "artifact_type" => "report/summary_collection"}
      ],
      "output_artifacts" => [%{"node_id" => "json_output", "artifact_type" => "export/json"}]
    }
  end

  defp graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.material-experiment-result-analysis-json",
      "name" => "Material experiment result analysis JSON",
      "version" => "1.0.0",
      "dataset_contract" =>
        WorkflowCatalogSupport.workflow_dataset_contract(
          "kyuubiki.dataset.material_experiment_result_analysis/v1",
          experiment_result_values(),
          %{"workflow_family" => "material_experiment_result_analysis"}
        ),
      "entry_nodes" => ["results_input"],
      "output_nodes" => ["json_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => [
        result_input_node(),
        analyze_node(),
        export_node(),
        output_node()
      ],
      "edges" => [
        edge(
          "e0",
          "results_input",
          "results",
          "analyze_results",
          "results",
          "material_experiment_results"
        ),
        edge(
          "e1",
          "analyze_results",
          "analysis",
          "export_json",
          "summary",
          "material_experiment_result_analysis"
        ),
        edge("e2", "export_json", "json", "json_output", "json", "summary_json")
      ]
    }
  end

  defp export_node do
    %{
      "id" => "export_json",
      "kind" => "export",
      "operator_id" => "export.summary_json",
      "inputs" => [port("summary", "report/summary", "material_experiment_result_analysis")],
      "outputs" => [port("json", "export/json", "summary_json")]
    }
  end
end
