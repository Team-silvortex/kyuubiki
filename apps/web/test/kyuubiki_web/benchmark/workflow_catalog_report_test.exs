defmodule KyuubikiWeb.Benchmark.WorkflowCatalogReportTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.TestSupport.WorkflowCatalogBenchmark

  test "writes a workflow catalog benchmark report when requested" do
    output_path = default_output_path()
    case_ids = parse_case_ids(System.get_env("WORKFLOW_CATALOG_BENCHMARK_CASES"))
    repeat = parse_repeat(System.get_env("WORKFLOW_CATALOG_BENCHMARK_REPEAT"))

    report =
      WorkflowCatalogBenchmark.benchmark_report(
        KyuubikiWeb.Router.init([]),
        case_ids,
        repeat
      )

    File.mkdir_p!(Path.dirname(output_path))
    File.write!(output_path, Jason.encode_to_iodata!(report, pretty: true))

    assert File.exists?(output_path)
    assert length(report["cases"]) == length(case_ids)
    assert report["summary"]["case_count"] == length(case_ids)
  end

  defp parse_case_ids(nil), do: WorkflowCatalogBenchmark.default_case_ids()
  defp parse_case_ids(""), do: WorkflowCatalogBenchmark.default_case_ids()

  defp parse_case_ids(raw_case_ids) do
    raw_case_ids
    |> String.split(",", trim: true)
    |> Enum.map(&String.trim/1)
  end

  defp parse_repeat(nil), do: 3
  defp parse_repeat(""), do: 3

  defp parse_repeat(raw_repeat) do
    case Integer.parse(raw_repeat) do
      {repeat, ""} when repeat > 0 -> repeat
      _ -> raise "invalid workflow catalog benchmark repeat: #{raw_repeat}"
    end
  end

  defp default_output_path do
    Path.expand("../../../../../tmp/workflow-catalog-benchmark.json", __DIR__)
  end
end
