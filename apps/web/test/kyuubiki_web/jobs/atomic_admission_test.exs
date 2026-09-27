defmodule KyuubikiWeb.Jobs.AtomicAdmissionTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{AnalysisResultStore, Storage}
  alias KyuubikiWeb.Jobs.Store

  setup do
    Store.reset()
    AnalysisResultStore.reset()
    :ok
  end

  test "admission creates a queued job and its initial result together" do
    attrs = attrs()
    result = %{"state" => "pending"}
    assert {:ok, job} = Store.create_with_result(attrs, result)
    assert job.status == :queued
    assert job.model_version_id == attrs.model_version_id
    assert {:ok, ^job} = Store.get(attrs.job_id)
    assert {:ok, ^result} = AnalysisResultStore.get(attrs.job_id)
  end

  test "duplicate admission cannot replace either existing record" do
    attrs = attrs()
    assert {:ok, job} = Store.create_with_result(attrs, %{"original" => true})
    assert {:error, {:job_already_exists, _}} = Store.create_with_result(attrs, %{"bad" => true})
    assert {:ok, ^job} = Store.get(attrs.job_id)
    assert {:ok, %{"original" => true}} = AnalysisResultStore.get(attrs.job_id)
  end

  @tag skip: not Storage.memory?()
  test "an orphan result rejects admission without leaving a newly inserted job" do
    attrs = attrs()
    assert :ok = AnalysisResultStore.put(attrs.job_id, %{"retained" => true})
    assert {:error, {:result_already_exists, _}} = Store.create_with_result(attrs, %{})
    assert :error = Store.get(attrs.job_id)
    assert {:ok, %{"retained" => true}} = AnalysisResultStore.get(attrs.job_id)
  end

  test "an unencodable initial result cannot leave half an admission" do
    attrs = attrs()

    assert {:error, _} = Store.create_with_result(attrs, %{"invalid" => fn -> :bad end})
    assert :error = Store.get(attrs.job_id)
    assert :error = AnalysisResultStore.get(attrs.job_id)
    assert {:ok, _} = Store.create_with_result(attrs, %{"retry" => true})
  end

  test "nonqueued and invalid admissions are rejected without creating records" do
    attrs = attrs()

    assert {:error, {:job_not_queued, _}} =
             Store.create_with_result(Map.put(attrs, :status, :solving), %{})

    assert {:error, _} = Store.create_with_result(Map.delete(attrs, :project_id), %{})
    assert {:error, :invalid_analysis_admission} = Store.create_with_result(attrs, [])
    assert {:error, :invalid_analysis_admission} = Store.initialize_result(attrs.job_id, nil)
    assert Store.list() == []
    assert AnalysisResultStore.list() == []
  end

  test "concurrent admissions have one winner and preserve its exact result" do
    attrs = attrs()

    replies =
      1..8
      |> Task.async_stream(
        fn n -> {n, Store.create_with_result(attrs, %{"winner" => n})} end,
        max_concurrency: 8,
        timeout: 10_000
      )
      |> Enum.map(fn {:ok, reply} -> reply end)

    assert [{winner, {:ok, job}}] = Enum.filter(replies, &match?({_, {:ok, _}}, &1))
    assert Enum.count(replies, &match?({_, {:error, _}}, &1)) == 7
    assert {:ok, ^job} = Store.get(attrs.job_id)
    assert {:ok, %{"winner" => ^winner}} = AnalysisResultStore.get(attrs.job_id)
  end

  test "concurrent initialization cannot replace the first result" do
    attrs = attrs()
    assert {:ok, job} = Store.create(attrs)

    replies =
      1..8
      |> Task.async_stream(
        fn n -> {n, Store.initialize_result(attrs.job_id, %{"winner" => n})} end,
        max_concurrency: 8,
        timeout: 10_000
      )
      |> Enum.map(fn {:ok, reply} -> reply end)

    assert [{winner, {:ok, ^job}}] = Enum.filter(replies, &match?({_, {:ok, _}}, &1))
    assert Enum.count(replies, &match?({_, {:error, _}}, &1)) == 7
    assert {:ok, %{"winner" => ^winner}} = AnalysisResultStore.get(attrs.job_id)
  end

  defp attrs do
    %{
      job_id: "atomic-admission-#{System.unique_integer([:positive])}",
      project_id: "admission",
      model_version_id: "version-1",
      simulation_case_id: "case-1"
    }
  end
end
