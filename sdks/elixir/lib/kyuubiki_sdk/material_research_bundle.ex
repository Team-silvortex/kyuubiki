defmodule KyuubikiSdk.MaterialResearchBundle do
  @moduledoc "Validator for retained automated material research bundle artifacts."

  alias KyuubikiSdk.Error

  @schema_version "kyuubiki.material-research-bundle/v1"
  @posture "screening_research_bundle"
  @exploration_schema_version "kyuubiki.material-exploration-run/v1"
  @next_round_execution_schema_version "kyuubiki.material-exploration-next-round-execution/v1"
  @chain_schema_version "kyuubiki.material-exploration-chain/v1"

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
      |> validate_summary(bundle["summary"])

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

  defp validate_summary(errors, %{} = summary) do
    errors
    |> require_string(summary["winner_candidate_id"], "summary.winner_candidate_id")
    |> require_string(summary["reliability_decision"], "summary.reliability_decision")
    |> require_string(summary["next_round_decision"], "summary.next_round_decision")
    |> require_string(summary["chain_stop_reason"], "summary.chain_stop_reason")
  end

  defp validate_summary(errors, _summary), do: ["summary must be an object" | errors]

  defp require_artifact_schema(errors, %{} = artifact, expected, field),
    do: require_equal(errors, artifact["schema_version"], expected, "#{field}.schema_version")

  defp require_artifact_schema(errors, _artifact, _expected, field),
    do: ["#{field} must be an object" | errors]

  defp require_equal(errors, actual, expected, field) do
    if actual == expected, do: errors, else: ["#{field} must be #{expected}" | errors]
  end

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
