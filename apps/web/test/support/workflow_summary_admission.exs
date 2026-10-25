defmodule KyuubikiWeb.TestSupport.WorkflowSummaryAdmission do
  @moduledoc false
  @fixture Path.expand("../../../../tests/fixtures/workflow-summary-admission.json", __DIR__)
           |> File.read!()
           |> Jason.decode!()

  def cases, do: @fixture["cases"]
  def invalid_markers, do: @fixture["invalid_markers"]
  def valid_markers, do: @fixture["valid_markers"]

  def comparison_cases do
    Path.expand("../../../../tests/fixtures/workflow-summary-comparison-range.json", __DIR__)
    |> File.read!()
    |> Jason.decode!()
    |> Map.fetch!("cases")
    |> Enum.map(
      &Map.merge(&1, %{
        "operator_id" => "transform.compare_summary_pair",
        "kind" => "transform",
        "shape" => "pair"
      })
    )
  end

  def locations(%{"shape" => "single"}), do: [nil]
  def locations(_sample), do: [nil, "left", "right"]
  def marked(payload, nil, marker), do: Map.merge(payload, marker)
  def marked(payload, key, marker), do: Map.update!(payload, key, &Map.merge(&1, marker))

  def reduction_cases do
    Path.expand("../../../../tests/fixtures/workflow-summary-reduction-range.json", __DIR__)
    |> File.read!()
    |> Jason.decode!()
    |> Map.fetch!("cases")
    |> Enum.map(&Map.put(&1, "kind", "transform"))
  end

  def direct(sample, payload) do
    runtime = KyuubikiWeb.WorkflowOperatorRuntime

    if sample["kind"] == "extract",
      do: runtime.run_extract_operator(sample["operator_id"], payload, sample["config"]),
      else: runtime.run_transform_operator(sample["operator_id"], payload, sample["config"])
  end

  def request(sample, marker, source \\ "left", skip \\ false) do
    single = sample["shape"] == "single"
    inputs = if single, do: %{"left" => sample["payload"]}, else: sample["payload"]
    inputs = Map.update!(inputs, source, &Map.merge(&1, marker))
    sources = Enum.sort(Map.keys(inputs))
    ports = if single, do: ["payload"], else: sources

    nodes =
      Enum.map(
        sources,
        &%{"id" => &1, "kind" => "input", "inputs" => [], "outputs" => [port("payload")]}
      )

    config = if skip, do: Map.put(sample["config"], "on_error", "skip"), else: sample["config"]

    nodes =
      nodes ++
        [
          %{
            "id" => "assess",
            "kind" => sample["kind"],
            "operator_id" => sample["operator_id"],
            "config" => config,
            "inputs" => Enum.map(ports, &port/1),
            "outputs" => [port("payload")]
          },
          %{"id" => "result", "kind" => "output", "inputs" => [port("payload")], "outputs" => []},
          %{"id" => "raw", "kind" => "output", "inputs" => [port("payload")], "outputs" => []}
        ]

    edges = Enum.zip(sources, ports) |> Enum.map(fn {id, port} -> edge(id, "assess", port) end)

    %{
      "graph" => %{
        "schema_version" => "kyuubiki.workflow-graph/v1",
        "id" => "summary-admission",
        "name" => "Summary admission",
        "version" => "1.0.0",
        "entry_nodes" => sources,
        "output_nodes" => ["result", "raw"],
        "nodes" => nodes,
        "edges" => edges ++ [edge("assess", "result", "payload"), edge(source, "raw", "payload")]
      },
      "input_artifacts" => inputs
    }
  end

  defp port(id), do: %{"id" => id, "artifact_type" => "artifact/json"}

  defp edge(from, to, port),
    do: %{
      "id" => "#{from}-#{to}",
      "from" => %{"node" => from, "port" => "payload"},
      "to" => %{"node" => to, "port" => port},
      "artifact_type" => "artifact/json"
    }
end
