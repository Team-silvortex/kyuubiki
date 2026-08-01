defmodule KyuubikiSdk.MaterialResearchBundle do
  @moduledoc "Validator for retained automated material research bundle artifacts."

  alias KyuubikiSdk.Error

  @schema_version "kyuubiki.material-research-bundle/v1"
  @posture "screening_research_bundle"
  @exploration_schema_version "kyuubiki.material-exploration-run/v1"
  @next_round_execution_schema_version "kyuubiki.material-exploration-next-round-execution/v1"
  @chain_schema_version "kyuubiki.material-exploration-chain/v1"
  @authority_trace_schema_version "kyuubiki.research-execution-authority-trace/v1"
  @execution_authority_schema_version "kyuubiki.execution-authority/v1"
  @research_evidence_schema_version "kyuubiki.material-research-evidence/v1"
  @validation_evidence_schema_version "kyuubiki.material-validation-evidence/v1"

  def schema_version, do: @schema_version

  def validate(%{} = bundle) do
    errors =
      []
      |> require_equal(bundle["schema_version"], @schema_version, "schema_version")
      |> require_equal(bundle["posture"], @posture, "posture")
      |> require_string(bundle["bundle_id"], "bundle_id")
      |> require_string(bundle["generated_at_utc"], "generated_at_utc")
      |> require_string(bundle["study"], "study")
      |> validate_checksums(bundle["artifact_checksums"])
      |> validate_reproducibility(bundle["reproducibility"])
      |> validate_execution_trace(bundle["execution_trace"])
      |> require_artifact_schema(
        bundle["initial_exploration"],
        @exploration_schema_version,
        "initial_exploration"
      )
      |> require_artifact_schema(
        bundle["next_round_execution_plan"],
        @next_round_execution_schema_version,
        "next_round_execution_plan"
      )
      |> require_artifact_schema(
        bundle["next_exploration"],
        @exploration_schema_version,
        "next_exploration"
      )
      |> require_artifact_schema(bundle["chain"], @chain_schema_version, "chain")
      |> validate_summary_artifact_consistency(bundle)
      |> validate_summary(bundle["summary"])
      |> validate_research_evidence(bundle)
      |> validate_validation_evidence(bundle)
      |> validate_material_card_refs(bundle)

    if errors == [], do: {:ok, bundle}, else: {:error, Error.validation(Enum.reverse(errors))}
  end

  def validate(_bundle),
    do: {:error, Error.validation(["material research bundle must be an object"])}

  defp validate_checksums(errors, %{} = checksums) do
    Enum.reduce(
      [
        "initial_exploration_sha256",
        "next_round_execution_plan_sha256",
        "next_exploration_sha256",
        "chain_sha256"
      ],
      errors,
      fn key, acc -> require_sha256(acc, checksums[key], "artifact_checksums.#{key}") end
    )
  end

  defp validate_checksums(errors, _checksums),
    do: ["artifact_checksums must be an object" | errors]

  defp validate_reproducibility(errors, %{} = reproducibility) do
    errors
    |> require_string(reproducibility["workspace"], "reproducibility.workspace")
    |> require_argv(reproducibility["initial_command"], "reproducibility.initial_command")
    |> require_argv(
      reproducibility["plan_next_command_template"],
      "reproducibility.plan_next_command_template"
    )
    |> require_argv(
      reproducibility["run_next_command_template"],
      "reproducibility.run_next_command_template"
    )
    |> require_argv(
      reproducibility["chain_next_command_template"],
      "reproducibility.chain_next_command_template"
    )
  end

  defp validate_reproducibility(errors, _reproducibility),
    do: ["reproducibility must be an object" | errors]

  defp validate_execution_trace(errors, %{} = trace) do
    errors =
      Enum.reduce(
        ~w(initial_duration_ms plan_next_duration_ms run_next_duration_ms chain_next_duration_ms),
        errors,
        fn key, acc ->
          require_non_negative_integer(acc, trace[key], "execution_trace.#{key}")
        end
      )

    case trace["authority"] do
      %{} = authority ->
        errors
        |> require_equal(
          authority["schema_version"],
          @authority_trace_schema_version,
          "execution_trace.authority.schema_version"
        )
        |> validate_authority_assertions(authority["assertions"])
        |> validate_real_solver_authority(
          authority["initial"],
          "execution_trace.authority.initial"
        )
        |> validate_real_solver_authority(
          authority["next"],
          "execution_trace.authority.next"
        )
        |> validate_authority_chain(authority["chain"])

      _ ->
        ["execution_trace.authority must be an object" | errors]
    end
  end

  defp validate_execution_trace(errors, _trace),
    do: ["execution_trace must be an object" | errors]

  defp validate_authority_assertions(errors, %{} = assertions) do
    Enum.reduce(~w(all_real_solver no_mock_execution no_fallback), errors, fn key, acc ->
      require_equal(acc, assertions[key], true, "execution_trace.authority.assertions.#{key}")
    end)
  end

  defp validate_authority_assertions(errors, _assertions),
    do: ["execution_trace.authority.assertions must be an object" | errors]

  defp validate_authority_chain(errors, chain) when is_list(chain) and chain != [] do
    chain
    |> Enum.with_index()
    |> Enum.reduce(errors, fn {authority, index}, acc ->
      validate_real_solver_authority(
        acc,
        authority,
        "execution_trace.authority.chain[#{index}]"
      )
    end)
  end

  defp validate_authority_chain(errors, _chain),
    do: ["execution_trace.authority.chain must be non-empty" | errors]

  defp validate_real_solver_authority(errors, %{} = authority, field) do
    errors
    |> require_equal(
      authority["schema_version"],
      @execution_authority_schema_version,
      "#{field}.schema_version"
    )
    |> require_equal(authority["execution_class"], "real_solver", "#{field}.execution_class")
    |> require_equal(authority["mock_execution"], false, "#{field}.mock_execution")
    |> require_equal(authority["fallback_used"], false, "#{field}.fallback_used")
    |> require_equal(authority["production_eligible"], true, "#{field}.production_eligible")
    |> require_string(authority["executor_id"], "#{field}.executor_id")
    |> require_string(authority["runtime"], "#{field}.runtime")
    |> require_string(authority["result_origin"], "#{field}.result_origin")
    |> require_string(authority["evidence_statement"], "#{field}.evidence_statement")
  end

  defp validate_real_solver_authority(errors, _authority, field),
    do: ["#{field} must be an object" | errors]

  defp validate_research_evidence(
         errors,
         %{
           "research_evidence" => %{} = evidence,
           "summary" => %{} = summary
         } = bundle
       ) do
    ranked = string_values(evidence["ranked_candidate_ids"])
    focus = string_values(evidence["focus_candidate_ids"])
    metrics = string_values(evidence["primary_metric_ids"])
    winner = evidence["winner_candidate_id"]

    errors
    |> require_equal(
      evidence["schema_version"],
      @research_evidence_schema_version,
      "research_evidence.schema_version"
    )
    |> require_string_list(
      evidence["ranked_candidate_ids"],
      "research_evidence.ranked_candidate_ids",
      false
    )
    |> require_unique(ranked, "research_evidence.ranked_candidate_ids")
    |> require_positive_integer(evidence["candidate_count"], "research_evidence.candidate_count")
    |> add_if(
      is_integer(evidence["candidate_count"]) and evidence["candidate_count"] != length(ranked),
      "research_evidence.candidate_count must match ranked_candidate_ids"
    )
    |> require_string(winner, "research_evidence.winner_candidate_id")
    |> require_equal(
      winner,
      summary["winner_candidate_id"],
      "research_evidence.winner_candidate_id"
    )
    |> add_if(
      is_binary(winner) and winner not in ranked,
      "research_evidence winner must be present in ranked candidates"
    )
    |> require_string_list(
      evidence["focus_candidate_ids"],
      "research_evidence.focus_candidate_ids",
      false
    )
    |> require_unique(focus, "research_evidence.focus_candidate_ids")
    |> add_unknown_candidates(focus, ranked)
    |> require_string_list(
      evidence["primary_metric_ids"],
      "research_evidence.primary_metric_ids",
      false
    )
    |> require_unique(metrics, "research_evidence.primary_metric_ids")
    |> require_positive_integer(
      evidence["metric_objective_count"],
      "research_evidence.metric_objective_count"
    )
    |> add_if(
      is_integer(evidence["metric_objective_count"]) and
        evidence["metric_objective_count"] != length(metrics),
      "research_evidence.metric_objective_count must match primary_metric_ids"
    )
    |> require_string_list(
      evidence["violated_quality_gate_ids"],
      "research_evidence.violated_quality_gate_ids",
      true
    )
    |> require_equal(
      evidence["quality_gate_decision"],
      summary["reliability_decision"],
      "research_evidence.quality_gate_decision"
    )
    |> require_equal(
      evidence["plan_decision"],
      summary["next_round_decision"],
      "research_evidence.plan_decision"
    )
    |> require_non_negative_integer(
      evidence["plan_step_count"],
      "research_evidence.plan_step_count"
    )
    |> require_optional_equal(
      evidence["plan_step_count"],
      summary["runnable_next_step_count"],
      "research_evidence.plan_step_count"
    )
    |> require_positive_integer(
      evidence["chain_round_count"],
      "research_evidence.chain_round_count"
    )
    |> require_optional_equal(
      evidence["chain_round_count"],
      summary["chain_round_count"],
      "research_evidence.chain_round_count"
    )
    |> require_positive_integer(
      evidence["chain_trace_round_count"],
      "research_evidence.chain_trace_round_count"
    )
    |> validate_chain_trace_count(evidence, bundle["chain"])
    |> require_string(
      evidence["final_winner_candidate_id"],
      "research_evidence.final_winner_candidate_id"
    )
    |> require_optional_equal(
      evidence["final_winner_candidate_id"],
      get_in(bundle, ["chain", "final_winner_candidate_id"]),
      "research_evidence.final_winner_candidate_id"
    )
  end

  defp validate_research_evidence(errors, _bundle),
    do: ["research_evidence must be an object" | errors]

  defp validate_validation_evidence(errors, %{
         "validation_evidence" => %{} = validation,
         "research_evidence" => %{} = research
       }) do
    errors
    |> require_equal(
      validation["schema_version"],
      @validation_evidence_schema_version,
      "validation_evidence.schema_version"
    )
    |> require_equal(
      validation["validation_posture"],
      "screening_validation",
      "validation_evidence.validation_posture"
    )
    |> validate_object_list(
      validation["baseline_refs"],
      "validation_evidence.baseline_refs",
      ~w(baseline_id kind status scope)
    )
    |> validate_confidence_counts(
      validation["candidate_confidence_counts"],
      "validation_evidence.candidate_confidence_counts"
    )
    |> validate_sensitivity_summary(validation["sensitivity_summary"], research)
    |> validate_object_list(
      validation["acceptance_criteria"],
      "validation_evidence.acceptance_criteria",
      ~w(criterion_id metric_id operator status)
    )
    |> validate_uncertainty_summary(validation)
    |> validate_validation_readiness(validation)
    |> require_string_list(
      validation["external_validation_plan"],
      "validation_evidence.external_validation_plan",
      false
    )
    |> require_string_list(
      validation["violated_quality_gate_ids"],
      "validation_evidence.violated_quality_gate_ids",
      true
    )
    |> require_equal(
      validation["violated_quality_gate_ids"],
      research["violated_quality_gate_ids"],
      "validation_evidence.violated_quality_gate_ids"
    )
  end

  defp validate_validation_evidence(errors, _bundle),
    do: ["validation_evidence must be an object" | errors]

  defp validate_sensitivity_summary(errors, %{} = summary, research) do
    errors
    |> require_equal(
      summary["schema_version"],
      "kyuubiki.material-sensitivity-summary/v1",
      "validation_evidence.sensitivity_summary.schema_version"
    )
    |> require_string(summary["method"], "validation_evidence.sensitivity_summary.method")
    |> require_string(
      summary["winner_stability_state"],
      "validation_evidence.sensitivity_summary.winner_stability_state"
    )
    |> require_string_list(
      summary["primary_metric_ids"],
      "validation_evidence.sensitivity_summary.primary_metric_ids",
      false
    )
    |> require_equal(
      summary["primary_metric_ids"],
      research["primary_metric_ids"],
      "validation_evidence.sensitivity_summary.primary_metric_ids"
    )
    |> require_string_list(
      summary["focus_candidate_ids"],
      "validation_evidence.sensitivity_summary.focus_candidate_ids",
      false
    )
    |> require_equal(
      summary["focus_candidate_ids"],
      research["focus_candidate_ids"],
      "validation_evidence.sensitivity_summary.focus_candidate_ids"
    )
    |> require_equal(
      summary["chain_trace_round_count"],
      research["chain_trace_round_count"],
      "validation_evidence.sensitivity_summary.chain_trace_round_count"
    )
  end

  defp validate_sensitivity_summary(errors, _summary, _research),
    do: ["validation_evidence.sensitivity_summary must be an object" | errors]

  defp validate_uncertainty_summary(
         errors,
         %{"uncertainty_summary" => %{} = uncertainty} = validation
       ) do
    errors
    |> require_equal(
      uncertainty["schema_version"],
      "kyuubiki.material-uncertainty-summary/v1",
      "validation_evidence.uncertainty_summary.schema_version"
    )
    |> require_string_list(
      uncertainty["known_limitations"],
      "validation_evidence.uncertainty_summary.known_limitations",
      false
    )
    |> require_equal(
      uncertainty["external_validation_required"],
      true,
      "validation_evidence.uncertainty_summary.external_validation_required"
    )
    |> validate_confidence_counts(
      uncertainty["candidate_confidence_counts"],
      "validation_evidence.uncertainty_summary.candidate_confidence_counts"
    )
    |> require_equal(
      uncertainty["candidate_confidence_counts"],
      validation["candidate_confidence_counts"],
      "validation_evidence.candidate_confidence_counts"
    )
  end

  defp validate_uncertainty_summary(errors, _validation),
    do: ["validation_evidence.uncertainty_summary must be an object" | errors]

  defp validate_validation_readiness(
         errors,
         %{"validation_readiness" => %{} = readiness} = validation
       ) do
    reasons = string_values(readiness["blocking_reasons"])
    low_count = get_in(validation, ["candidate_confidence_counts", "low"])

    errors
    |> require_equal(
      readiness["schema_version"],
      "kyuubiki.material-validation-readiness/v1",
      "validation_evidence.validation_readiness.schema_version"
    )
    |> require_equal(
      readiness["decision"],
      "screening_only",
      "validation_evidence.validation_readiness.decision"
    )
    |> require_score(readiness["score"], "validation_evidence.validation_readiness.score")
    |> require_string_list(
      readiness["blocking_reasons"],
      "validation_evidence.validation_readiness.blocking_reasons",
      false
    )
    |> add_if(
      "external_validation_required" not in reasons,
      "validation_evidence.validation_readiness.blocking_reasons must include external_validation_required"
    )
    |> add_if(
      validation["violated_quality_gate_ids"] not in [nil, []] and
        "violated_quality_gates" not in reasons,
      "validation_evidence.validation_readiness.blocking_reasons must include violated_quality_gates when gates are violated"
    )
    |> add_if(
      is_integer(low_count) and low_count > 0 and "low_confidence_material_cards" not in reasons,
      "validation_evidence.validation_readiness.blocking_reasons must include low_confidence_material_cards when low-confidence cards exist"
    )
    |> require_string_list(
      readiness["next_validation_actions"],
      "validation_evidence.validation_readiness.next_validation_actions",
      false
    )
  end

  defp validate_validation_readiness(errors, _validation),
    do: ["validation_evidence.validation_readiness must be an object" | errors]

  defp validate_material_card_refs(errors, %{
         "summary" => %{} = summary,
         "research_evidence" => %{} = research
       }) do
    refs = if is_list(summary["material_card_refs"]), do: summary["material_card_refs"], else: []
    ranked = string_values(research["ranked_candidate_ids"])

    errors =
      errors
      |> validate_object_list(
        summary["material_card_refs"],
        "summary.material_card_refs",
        ~w(material_card_id candidate_id confidence unit_system parameter_scope)
      )
      |> require_positive_integer(
        summary["material_card_ref_count"],
        "summary.material_card_ref_count"
      )
      |> add_if(
        is_integer(summary["material_card_ref_count"]) and
          summary["material_card_ref_count"] != length(refs),
        "summary.material_card_ref_count must match material_card_refs"
      )

    refs
    |> Enum.with_index()
    |> Enum.reduce(errors, fn
      {%{} = ref, index}, acc ->
        acc
        |> require_equal(
          ref["schema_version"],
          "kyuubiki.material-card/v1",
          "summary.material_card_refs[#{index}].schema_version"
        )
        |> add_if(
          is_binary(ref["candidate_id"]) and ref["candidate_id"] not in ranked,
          "summary.material_card_refs[#{index}].candidate_id must be present in ranked candidates"
        )

      {_ref, _index}, acc ->
        acc
    end)
  end

  defp validate_material_card_refs(errors, _bundle), do: errors

  defp validate_summary(errors, %{} = summary) do
    errors
    |> require_string(summary["winner_candidate_id"], "summary.winner_candidate_id")
    |> require_string(summary["reliability_decision"], "summary.reliability_decision")
    |> require_string(summary["next_round_decision"], "summary.next_round_decision")
    |> require_string(summary["chain_stop_reason"], "summary.chain_stop_reason")
  end

  defp validate_summary(errors, _summary), do: ["summary must be an object" | errors]

  defp validate_summary_artifact_consistency(errors, %{"summary" => %{} = summary} = bundle) do
    errors
    |> require_value_equal(
      bundle["next_round_execution_plan"],
      "decision",
      summary["next_round_decision"],
      "next_round_execution_plan.decision"
    )
    |> require_optional_value_equal(
      bundle["next_round_execution_plan"],
      "runnable_step_count",
      summary["runnable_next_step_count"],
      "next_round_execution_plan.runnable_step_count"
    )
    |> require_optional_value_equal(
      bundle["next_round_execution_plan"],
      "iteration",
      summary["next_iteration"],
      "next_round_execution_plan.iteration"
    )
    |> require_optional_value_equal(
      bundle["next_exploration"],
      "iteration",
      summary["next_iteration"],
      "next_exploration.iteration"
    )
    |> require_value_equal(
      bundle["chain"],
      "stop_reason",
      summary["chain_stop_reason"],
      "chain.stop_reason"
    )
  end

  defp validate_summary_artifact_consistency(errors, _bundle), do: errors

  defp validate_chain_trace_count(errors, evidence, %{"optimization_trace" => trace})
       when is_list(trace) do
    add_if(
      errors,
      evidence["chain_trace_round_count"] != length(trace),
      "research_evidence.chain_trace_round_count must match chain.optimization_trace"
    )
  end

  defp validate_chain_trace_count(errors, _evidence, _chain), do: errors

  defp add_unknown_candidates(errors, focus, ranked) do
    case Enum.find(focus, &(&1 not in ranked)) do
      nil ->
        errors

      candidate ->
        [
          "research_evidence.focus_candidate_ids contains unknown candidate #{inspect(candidate)}"
          | errors
        ]
    end
  end

  defp validate_object_list(errors, value, field, required_strings)
       when is_list(value) and value != [] do
    value
    |> Enum.with_index()
    |> Enum.reduce(errors, fn
      {%{} = item, index}, acc ->
        Enum.reduce(required_strings, acc, fn key, nested ->
          require_string(nested, item[key], "#{field}[#{index}].#{key}")
        end)

      {_item, index}, acc ->
        ["#{field}[#{index}] must be an object" | acc]
    end)
  end

  defp validate_object_list(errors, _value, field, _required_strings),
    do: ["#{field} must be a non-empty array" | errors]

  defp validate_confidence_counts(errors, %{} = counts, field) do
    Enum.reduce(~w(low medium high unknown), errors, fn key, acc ->
      require_non_negative_integer(acc, counts[key], "#{field}.#{key}")
    end)
  end

  defp validate_confidence_counts(errors, _counts, field),
    do: ["#{field} must be an object" | errors]

  defp require_string_list(errors, value, field, allow_empty)
       when is_list(value) do
    cond do
      value == [] and not allow_empty -> ["#{field} must be non-empty" | errors]
      Enum.all?(value, &(is_binary(&1) and &1 != "")) -> errors
      true -> ["#{field} must contain only non-empty strings" | errors]
    end
  end

  defp require_string_list(errors, _value, field, _allow_empty),
    do: ["#{field} must be an array" | errors]

  defp string_values(value) when is_list(value),
    do: Enum.filter(value, &(is_binary(&1) and &1 != ""))

  defp string_values(_value), do: []

  defp require_unique(errors, values, field) do
    if length(values) == MapSet.size(MapSet.new(values)) do
      errors
    else
      ["#{field} must not contain duplicates" | errors]
    end
  end

  defp require_non_negative_integer(errors, value, _field)
       when is_integer(value) and value >= 0,
       do: errors

  defp require_non_negative_integer(errors, _value, field),
    do: ["#{field} must be a non-negative integer" | errors]

  defp require_positive_integer(errors, value, _field)
       when is_integer(value) and value > 0,
       do: errors

  defp require_positive_integer(errors, _value, field),
    do: ["#{field} must be positive" | errors]

  defp require_score(errors, value, _field)
       when is_number(value) and value >= 0 and value <= 1,
       do: errors

  defp require_score(errors, _value, field),
    do: ["#{field} must be between 0 and 1" | errors]

  defp require_optional_equal(errors, _actual, nil, _field), do: errors

  defp require_optional_equal(errors, actual, expected, field),
    do: require_equal(errors, actual, expected, field)

  defp add_if(errors, true, message), do: [message | errors]
  defp add_if(errors, false, _message), do: errors

  defp require_artifact_schema(errors, %{} = artifact, expected, field),
    do: require_equal(errors, artifact["schema_version"], expected, "#{field}.schema_version")

  defp require_artifact_schema(errors, _artifact, _expected, field),
    do: ["#{field} must be an object" | errors]

  defp require_equal(errors, actual, expected, field) do
    if actual == expected, do: errors, else: ["#{field} must be #{expected}" | errors]
  end

  defp require_value_equal(errors, %{} = artifact, key, expected, field),
    do: require_equal(errors, artifact[key], expected, field)

  defp require_value_equal(errors, _artifact, _key, _expected, field),
    do: ["#{field} is required" | errors]

  defp require_optional_value_equal(errors, _artifact, _key, nil, _field), do: errors

  defp require_optional_value_equal(errors, artifact, key, expected, field),
    do: require_value_equal(errors, artifact, key, expected, field)

  defp require_string(errors, value, _field) when is_binary(value) and value != "", do: errors
  defp require_string(errors, _value, field), do: ["#{field} must be a non-empty string" | errors]

  defp require_argv(errors, value, field) when is_list(value) and value != [] do
    if Enum.all?(value, &(is_binary(&1) and &1 != "")) do
      errors
    else
      ["#{field} must be a non-empty argv array" | errors]
    end
  end

  defp require_argv(errors, _value, field),
    do: ["#{field} must be a non-empty argv array" | errors]

  defp require_sha256(errors, value, field) when is_binary(value) do
    if String.match?(value, ~r/\A[0-9a-f]{64}\z/) do
      errors
    else
      ["#{field} must be a lowercase SHA-256 hex digest" | errors]
    end
  end

  defp require_sha256(errors, _value, field),
    do: ["#{field} must be a lowercase SHA-256 hex digest" | errors]
end
