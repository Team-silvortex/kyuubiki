defmodule KyuubikiWeb.Benchmark.WorkflowLargeGraphReportTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.TestSupport.WorkflowLargeGraphBenchmark

  test "writes a large-graph benchmark report when requested" do
    output_path = default_output_path()
    counts = parse_counts(System.get_env("WORKFLOW_LARGE_GRAPH_COUNTS"))

    report =
      WorkflowLargeGraphBenchmark.benchmark_report(
        KyuubikiWeb.Router.init([]),
        counts,
        parse_response_options(System.get_env("WORKFLOW_LARGE_GRAPH_RESPONSE_MODE"))
      )

    File.mkdir_p!(Path.dirname(output_path))
    File.write!(output_path, Jason.encode_to_iodata!(report, pretty: true))

    assert File.exists?(output_path)
    assert length(report["cases"]) == length(counts)
    assert report["summary"]["case_count"] == length(counts)
  end

  defp parse_counts(nil), do: WorkflowLargeGraphBenchmark.default_pass_through_counts()
  defp parse_counts(""), do: WorkflowLargeGraphBenchmark.default_pass_through_counts()

  defp parse_counts(raw_counts) do
    raw_counts
    |> String.split(",", trim: true)
    |> Enum.map(&String.trim/1)
    |> Enum.map(fn value ->
      case Integer.parse(value) do
        {count, ""} when count > 0 -> count
        _ -> raise "invalid workflow large graph benchmark count: #{value}"
      end
    end)
  end

  defp parse_response_options("compact"),
    do: %{"response_mode" => "compact"}

  defp parse_response_options("full"), do: %{"response_mode" => "full"}
  defp parse_response_options(_mode), do: %{}

  defp default_output_path do
    Path.expand("../../../../../tmp/workflow-large-graph-benchmark.json", __DIR__)
  end
end
