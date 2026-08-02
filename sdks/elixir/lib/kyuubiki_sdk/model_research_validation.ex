defmodule KyuubikiSdk.ModelResearchValidation do
  @moduledoc "Verified handoff from a model research frontier to retained validation evidence."

  alias KyuubikiSdk.Error
  alias KyuubikiSdk.MaterialResearchBundle
  alias KyuubikiSdk.ModelResearchExecution
  alias KyuubikiSdk.ModelResearchFrontier
  alias KyuubikiSdk.WorkflowResults

  @schema_version "kyuubiki.model-research-validation-report/v2"
  @claim_boundary "screening_only_not_qualification"

  def schema_version, do: @schema_version

  def validate(frontier, result_receipt, graph, bundle, frontier_verifier, receipt_verifier)
      when is_function(frontier_verifier, 1) and is_function(receipt_verifier, 1) do
    with :ok <- validate_frontier_binding(frontier),
         :ok <- verify(frontier_verifier, frontier, "frontier"),
         {:ok, record} <- validate_result_receipt(frontier, result_receipt),
         :ok <- verify(receipt_verifier, result_receipt, "receipt"),
         :ok <- validate_graph_binding(frontier, graph),
         {:ok, validated} <-
           WorkflowResults.validate_result_against_graph(graph, record["output"]),
         :ok <- validate_runtime(frontier, validated["workflow_runtime"]),
         :ok <- validate_artifacts(validated["artifacts"]),
         {:ok, stage, bundle_evidence, next_actions} <- validate_bundle(bundle) do
      {:ok,
       %{
         "schema_version" => @schema_version,
         "session_id" => frontier["session_id"],
         "workflow_id" => frontier["workflow_id"],
         "job_id" => frontier["job_id"],
         "origin_plan_digest" => frontier["origin_plan_digest"],
         "result_plan_digest" => result_receipt["plan_digest"],
         "stage" => stage,
         "claim_boundary" => @claim_boundary,
         "external_validation_required" => true,
         "workflow_result" => %{
           "graph_id" => validated["graph_id"],
           "graph_version" => validated["graph_version"],
           "runtime_status" => get_in(validated, ["workflow_runtime", "status"]),
           "artifact_keys" => validated["artifacts"] |> Map.keys() |> Enum.sort()
         },
         "material_bundle" => bundle_evidence,
         "next_actions" => next_actions
       }}
    end
  end

  def validate(_frontier, _receipt, _graph, _bundle, _frontier_verifier, _receipt_verifier),
    do: validation_error("frontier and receipt verifiers must be one-argument functions")

  defp validate_frontier_binding(%{} = frontier) do
    with :ok <- ModelResearchFrontier.validate(frontier) do
      valid =
        frontier["stage"] == "ready_to_validate" and is_nil(frontier["next_action"]) and
          is_nil(frontier["blocking_reason"]) and present?(frontier["job_id"])

      if valid,
        do: :ok,
        else: validation_error("research frontier is not ready for result validation")
    end
  end

  defp validate_frontier_binding(_frontier),
    do: validation_error("research frontier is not ready for result validation")

  defp validate_result_receipt(frontier, %{} = receipt) do
    record = if is_list(receipt["records"]), do: List.last(receipt["records"])

    valid =
      receipt["schema_version"] == ModelResearchExecution.receipt_schema_version() and
        receipt["execution_authority"] == "kyuubiki-headless-sdk" and
        receipt["status"] == "completed" and
        receipt["session_id"] == frontier["session_id"] and
        receipt["workflow_id"] == frontier["workflow_id"] and is_map(record) and
        receipt["plan_digest"] == get_in(frontier, ["evidence", "plan_digest"]) and
        record["action"] == "result_fetch" and record["job_id"] == frontier["job_id"] and
        present?(record["authority"]) and not is_nil(record["output"]) and
        is_nil(record["error"])

    if valid,
      do: {:ok, record},
      else: validation_error("result receipt does not match the verified research frontier")
  end

  defp validate_result_receipt(_frontier, _receipt),
    do: validation_error("result receipt does not match the verified research frontier")

  defp validate_graph_binding(frontier, %{"id" => graph_id}) do
    if graph_id == frontier["workflow_id"],
      do: :ok,
      else: validation_error("workflow graph id does not match research frontier workflow_id")
  end

  defp validate_graph_binding(_frontier, _graph),
    do: validation_error("workflow graph id does not match research frontier workflow_id")

  defp validate_runtime(frontier, %{} = runtime) do
    cond do
      runtime["status"] != "completed" ->
        validation_error("workflow result runtime status must be completed")

      not is_nil(runtime["workflow_id"]) and runtime["workflow_id"] != frontier["workflow_id"] ->
        validation_error("workflow result runtime workflow_id does not match frontier")

      true ->
        :ok
    end
  end

  defp validate_runtime(_frontier, _runtime),
    do: validation_error("workflow result runtime status is required")

  defp validate_artifacts(artifacts) when is_map(artifacts) and map_size(artifacts) > 0, do: :ok

  defp validate_artifacts(_artifacts),
    do: validation_error("workflow result validation produced no retained artifacts")

  defp validate_bundle(nil) do
    {:ok, "workflow_result_validated", nil,
     ["build_or_attach_material_research_bundle", "external_validation_required"]}
  end

  defp validate_bundle(%{} = bundle) do
    with {:ok, bundle} <- MaterialResearchBundle.validate(bundle) do
      readiness = get_in(bundle, ["validation_evidence", "validation_readiness"])

      actions =
        readiness["next_validation_actions"] |> Enum.uniq() |> append_external_validation()

      {:ok, "screening_bundle_validated",
       %{
         "schema_version" => bundle["schema_version"],
         "bundle_id" => bundle["bundle_id"],
         "study" => bundle["study"],
         "reliability_decision" => get_in(bundle, ["summary", "reliability_decision"]),
         "validation_readiness_score" => readiness["score"]
       }, actions}
    end
  end

  defp validate_bundle(_bundle),
    do: validation_error("material research bundle must be an object or nil")

  defp append_external_validation(actions) do
    if "external_validation_required" in actions,
      do: actions,
      else: actions ++ ["external_validation_required"]
  end

  defp verify(verifier, value, kind) do
    case verifier.(value) do
      true -> :ok
      :ok -> :ok
      {:ok, _evidence} -> :ok
      _ -> validation_error("caller #{kind} verifier rejected research evidence")
    end
  end

  defp present?(value), do: is_binary(value) and String.trim(value) != ""

  defp validation_error(message), do: {:error, Error.model_research_execution([message])}
end
