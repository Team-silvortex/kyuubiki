defmodule KyuubikiSdk.ModelPlanApproval do
  @moduledoc "Canonical plan digests and read-only caller approval requests."

  alias KyuubikiSdk.Error
  alias KyuubikiSdk.ModelCollaboration

  @request_schema "kyuubiki.model-plan-approval-request/v1"
  @approval_schema "kyuubiki.model-plan-approval/v2"

  def request_schema_version, do: @request_schema
  def approval_schema_version, do: @approval_schema

  def compute_digest(plan) when is_map(plan) do
    with {:ok, canonical} <- canonical_json(plan) do
      digest = :crypto.hash(:sha256, canonical) |> Base.encode16(case: :lower)
      {:ok, "sha256:" <> digest}
    end
  end

  def compute_digest(_plan), do: validation_error("model plan must be a JSON object")

  def build_request(plan) when is_map(plan) do
    with :ok <- validate_plan(plan),
         {:ok, required_steps} <- required_steps(plan["steps"]),
         {:ok, digest} <- compute_digest(plan) do
      {:ok,
       %{
         "schema_version" => @request_schema,
         "plan_schema_version" => plan["schema_version"],
         "plan_digest" => digest,
         "session_id" => plan["session_id"],
         "workflow_id" => plan["workflow_id"],
         "status" => if(required_steps == [], do: "not_required", else: "approval_required"),
         "execution_authority" => "none_approval_request_only",
         "approval_schema_version" => @approval_schema,
         "required_steps" => required_steps
       }}
    end
  end

  def build_request(_plan), do: validation_error("model plan must be a JSON object")

  defp validate_plan(plan) do
    steps = if is_list(plan["steps"]), do: plan["steps"], else: []

    errors =
      []
      |> add_issue(
        plan["schema_version"] != ModelCollaboration.plan_schema_version(),
        "unsupported model plan schema_version: #{inspect(plan["schema_version"])}"
      )
      |> add_issue(
        plan["ok"] != true or plan["issues"] not in [nil, []],
        "model plan must be valid and issue-free before approval"
      )
      |> add_issue(
        not present_string?(plan["session_id"]) or not present_string?(plan["workflow_id"]),
        "model plan session_id and workflow_id are required"
      )
      |> add_issue(steps == [], "model plan contains no steps")
      |> add_issue(
        not contiguous_steps?(steps),
        "model plan step indexes must be contiguous and one-based"
      )

    if errors == [],
      do: :ok,
      else: {:error, Error.model_research_execution(errors |> Enum.uniq() |> Enum.sort())}
  end

  defp required_steps(steps) do
    steps
    |> Enum.filter(&(&1["requires_confirmation"] == true))
    |> Enum.reduce_while({:ok, []}, fn step, {:ok, acc} ->
      reason = step["confirmation_reason"]
      risk = step["risk"]

      cond do
        not present_string?(reason) ->
          {:halt,
           validation_error(
             "gated model plan step #{step["index"]} requires a confirmation_reason"
           )}

        risk not in ["sensitive", "destructive"] ->
          {:halt, validation_error("gated model plan step #{step["index"]} has invalid risk")}

        true ->
          request_step = %{
            "index" => step["index"],
            "action" => step["action"],
            "risk" => risk,
            "confirmation_reason" => reason
          }

          {:cont, {:ok, [request_step | acc]}}
      end
    end)
    |> then(fn
      {:ok, required} -> {:ok, Enum.reverse(required)}
      error -> error
    end)
  end

  defp canonical_json(value) when is_map(value) do
    if Enum.all?(Map.keys(value), &is_binary/1) do
      value
      |> Map.keys()
      |> Enum.sort()
      |> Enum.reduce_while({:ok, []}, fn key, {:ok, parts} ->
        with {:ok, encoded_value} <- canonical_json(value[key]) do
          {:cont, {:ok, [Jason.encode!(key) <> ":" <> encoded_value | parts]}}
        else
          error -> {:halt, error}
        end
      end)
      |> then(fn
        {:ok, parts} -> {:ok, "{" <> (parts |> Enum.reverse() |> Enum.join(",")) <> "}"}
        error -> error
      end)
    else
      validation_error("model plan JSON object keys must be strings")
    end
  end

  defp canonical_json(value) when is_list(value) do
    value
    |> Enum.reduce_while({:ok, []}, fn item, {:ok, parts} ->
      case canonical_json(item) do
        {:ok, encoded} -> {:cont, {:ok, [encoded | parts]}}
        error -> {:halt, error}
      end
    end)
    |> then(fn
      {:ok, parts} -> {:ok, "[" <> (parts |> Enum.reverse() |> Enum.join(",")) <> "]"}
      error -> error
    end)
  end

  defp canonical_json(value) when is_integer(value), do: {:ok, Integer.to_string(value)}

  defp canonical_json(value) when is_float(value) do
    encoded = value |> :erlang.float_to_binary(decimals: 15) |> String.trim_trailing("0")
    {:ok, if(String.ends_with?(encoded, "."), do: encoded <> "0", else: encoded)}
  end

  defp canonical_json(value) when is_binary(value) or is_boolean(value) or is_nil(value),
    do: {:ok, Jason.encode!(value)}

  defp canonical_json(value),
    do: validation_error("model plan contains non-JSON value: #{inspect(value, limit: 4)}")

  defp present_string?(value), do: is_binary(value) and String.trim(value) != ""

  defp contiguous_steps?(steps) do
    steps
    |> Enum.with_index(1)
    |> Enum.all?(fn {step, index} -> is_map(step) and step["index"] == index end)
  end

  defp add_issue(errors, true, message), do: [message | errors]
  defp add_issue(errors, false, _message), do: errors
  defp validation_error(message), do: {:error, Error.model_research_execution([message])}
end
