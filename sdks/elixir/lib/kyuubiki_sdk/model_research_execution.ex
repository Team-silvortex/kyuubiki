defmodule KyuubikiSdk.ModelResearchExecution do
  @moduledoc "Caller-verified execution of validated model Headless plans."

  alias KyuubikiSdk.ControlPlaneClient
  alias KyuubikiSdk.Error
  alias KyuubikiSdk.ModelCollaboration
  alias KyuubikiSdk.ModelPlanApproval
  alias KyuubikiSdk.Session

  @approval_schema "kyuubiki.model-plan-approval/v2"
  @receipt_schema "kyuubiki.model-research-execution-receipt/v2"

  def approval_schema_version, do: @approval_schema
  def receipt_schema_version, do: @receipt_schema

  def session_dispatcher(session, opts \\ []) do
    fn action, payload -> dispatch_session_action(session, action, payload, opts) end
  end

  def execute(dispatcher, plan, approval, approval_verifier)
      when is_function(dispatcher, 2) and is_function(approval_verifier, 2) do
    with {:ok, plan_digest} <- validate_execution_request(plan, approval),
         :ok <- verify_approval(plan, approval, approval_verifier),
         {:ok, verified_digest} <- ModelPlanApproval.compute_digest(plan),
         :ok <- ensure_digest_unchanged(plan_digest, verified_digest) do
      execute_steps(dispatcher, plan, plan_digest, approval)
    end
  end

  def execute(_dispatcher, _plan, _approval, _approval_verifier),
    do: validation_error("dispatcher and approval verifier must be two-argument functions")

  def dispatch_session_action(session, action, payload, opts \\ []) do
    poll_interval = Keyword.get(opts, :poll_interval, 500)
    timeout = Keyword.get(opts, :timeout, 300_000)

    with :ok <- validate_wait_bounds(poll_interval, timeout) do
      do_dispatch_session_action(session, action, payload, poll_interval, timeout)
    end
  end

  defp do_dispatch_session_action(session, "direct_solver_rpc", payload, _poll, _timeout) do
    with {:ok, solve_kind} <- required_string(payload, "solve_kind"),
         {:ok, model_payload} <- required_map(payload, "payload"),
         {:ok, output} <- Session.solve_direct(session, solve_kind, model_payload) do
      dispatch_result("solver_rpc", output)
    end
  end

  defp do_dispatch_session_action(session, action, payload, poll_interval, timeout) do
    with {:ok, client} <- control_plane(session) do
      dispatch_control_plane(session, client, action, payload, poll_interval, timeout)
    end
  end

  defp dispatch_control_plane(_session, client, "service_health", _payload, _poll, _timeout),
    do: wrap_dispatch("control_plane", ControlPlaneClient.health(client))

  defp dispatch_control_plane(_session, client, "protocol_describe", _payload, _poll, _timeout),
    do: wrap_dispatch("control_plane", ControlPlaneClient.protocol(client))

  defp dispatch_control_plane(_session, client, "agents_describe", _payload, _poll, _timeout),
    do: wrap_dispatch("control_plane", ControlPlaneClient.agents(client))

  defp dispatch_control_plane(
         _session,
         client,
         "workflow_catalog_list",
         _payload,
         _poll,
         _timeout
       ),
       do: wrap_dispatch("control_plane", ControlPlaneClient.list_workflow_catalog(client))

  defp dispatch_control_plane(
         _session,
         client,
         "operator_catalog_list",
         _payload,
         _poll,
         _timeout
       ),
       do: wrap_dispatch("control_plane", ControlPlaneClient.list_workflow_operators(client))

  defp dispatch_control_plane(_session, client, "fem_submit", payload, _poll, _timeout) do
    with {:ok, solve_kind} <- required_string(payload, "solve_kind"),
         {:ok, model_payload} <- required_map(payload, "payload") do
      wrap_dispatch(
        "control_plane",
        ControlPlaneClient.submit_fem_job(client, solve_kind, model_payload)
      )
    end
  end

  defp dispatch_control_plane(
         _session,
         client,
         "workflow_submit_catalog",
         payload,
         _poll,
         _timeout
       ) do
    with {:ok, workflow_id} <- required_string(payload, "workflow_id"),
         {:ok, input_artifacts} <- required_map(payload, "input_artifacts") do
      wrap_dispatch(
        "control_plane",
        ControlPlaneClient.submit_workflow_catalog_job(client, workflow_id, input_artifacts)
      )
    end
  end

  defp dispatch_control_plane(_session, client, "workflow_submit_graph", payload, _poll, _timeout) do
    with {:ok, graph} <- required_map(payload, "graph"),
         {:ok, input_artifacts} <- required_map(payload, "input_artifacts") do
      wrap_dispatch(
        "control_plane",
        ControlPlaneClient.submit_workflow_graph_job(client, graph, input_artifacts)
      )
    end
  end

  defp dispatch_control_plane(_session, client, "operator_task_prepare", payload, _poll, _timeout) do
    with {:ok, task} <- required_map(payload, "task") do
      wrap_dispatch("control_plane", ControlPlaneClient.prepare_operator_task(client, task))
    end
  end

  defp dispatch_control_plane(_session, client, "operator_task_execute", payload, _poll, _timeout) do
    with {:ok, task} <- required_map(payload, "task") do
      wrap_dispatch("control_plane", ControlPlaneClient.execute_operator_task(client, task))
    end
  end

  defp dispatch_control_plane(
         _session,
         client,
         "operator_task_batch_prepare",
         payload,
         _poll,
         _timeout
       ) do
    with {:ok, batch} <- required_map(payload, "batch") do
      wrap_dispatch(
        "control_plane",
        ControlPlaneClient.prepare_operator_task_batch(client, batch)
      )
    end
  end

  defp dispatch_control_plane(
         _session,
         client,
         "operator_task_batch_execute",
         payload,
         _poll,
         _timeout
       ) do
    with {:ok, batch} <- required_map(payload, "batch") do
      wrap_dispatch(
        "control_plane",
        ControlPlaneClient.execute_operator_task_batch(client, batch)
      )
    end
  end

  defp dispatch_control_plane(session, _client, "job_wait", payload, poll_interval, timeout) do
    with {:ok, job_id} <- required_string(payload, "job_id") do
      wrap_dispatch(
        "control_plane",
        Session.wait_for_job(session, job_id, poll_interval: poll_interval, timeout: timeout)
      )
    end
  end

  defp dispatch_control_plane(_session, client, "result_fetch", payload, _poll, _timeout) do
    with {:ok, job_id} <- required_string(payload, "job_id") do
      wrap_dispatch("control_plane", ControlPlaneClient.fetch_result(client, job_id))
    end
  end

  defp dispatch_control_plane(_session, client, "result_chunk_fetch", payload, _poll, _timeout) do
    with {:ok, job_id} <- required_string(payload, "job_id"),
         {:ok, kind} <- required_string(payload, "kind"),
         {:ok, offset} <- optional_unsigned(payload, "offset"),
         {:ok, limit} <- optional_unsigned(payload, "limit") do
      wrap_dispatch(
        "control_plane",
        ControlPlaneClient.fetch_result_chunk(client, job_id, kind,
          offset: offset,
          limit: limit
        )
      )
    end
  end

  defp dispatch_control_plane(_session, client, "job_cancel", payload, _poll, _timeout) do
    with {:ok, job_id} <- required_string(payload, "job_id") do
      wrap_dispatch("control_plane", ControlPlaneClient.cancel_job(client, job_id))
    end
  end

  defp dispatch_control_plane(_session, _client, action, _payload, _poll, _timeout),
    do: validation_error("unsupported model action: #{action}")

  defp execute_steps(dispatcher, plan, plan_digest, approval) do
    {status, failed_step, records} =
      Enum.reduce_while(plan["steps"], {:completed, nil, []}, fn step,
                                                                 {_status, _failed, records} ->
        case dispatcher.(step["action"], step["payload"]) do
          {:ok, dispatched} ->
            record = %{
              "index" => step["index"],
              "action" => step["action"],
              "job_id" => action_job_id(step["action"], step["payload"]),
              "authority" => dispatched["authority"],
              "output" => dispatched["output"],
              "error" => nil
            }

            {:cont, {:completed, nil, [record | records]}}

          {:error, reason} ->
            record = %{
              "index" => step["index"],
              "action" => step["action"],
              "job_id" => action_job_id(step["action"], step["payload"]),
              "authority" => nil,
              "output" => nil,
              "error" => bounded_error(reason)
            }

            {:halt, {:failed, step["index"], [record | records]}}

          other ->
            record = %{
              "index" => step["index"],
              "action" => step["action"],
              "job_id" => action_job_id(step["action"], step["payload"]),
              "authority" => nil,
              "output" => nil,
              "error" => bounded_error({:invalid_dispatch_result, other})
            }

            {:halt, {:failed, step["index"], [record | records]}}
        end
      end)

    records = Enum.reverse(records)

    {:ok,
     %{
       "schema_version" => @receipt_schema,
       "plan_schema_version" => plan["schema_version"],
       "session_id" => plan["session_id"],
       "workflow_id" => plan["workflow_id"],
       "plan_digest" => plan_digest,
       "status" => Atom.to_string(status),
       "execution_authority" => "kyuubiki-headless-sdk",
       "approval_id" => if(approval, do: approval["approval_id"], else: nil),
       "completed_steps" => Enum.count(records, &is_nil(&1["error"])),
       "failed_step" => failed_step,
       "records" => records
     }}
  end

  defp action_job_id(action, payload)
       when action in ["job_wait", "result_fetch", "result_chunk_fetch", "job_cancel"] and
              is_map(payload),
       do: payload["job_id"]

  defp action_job_id(_action, _payload), do: nil

  defp validate_execution_request(plan, approval) when is_map(plan) do
    steps = if is_list(plan["steps"]), do: plan["steps"], else: []

    errors =
      []
      |> add_issue(
        plan["schema_version"] != ModelCollaboration.plan_schema_version(),
        "unsupported model plan schema_version: #{inspect(plan["schema_version"])}"
      )
      |> add_issue(
        plan["ok"] != true or plan["issues"] not in [nil, []],
        "model plan must be valid and issue-free before dispatch"
      )
      |> add_issue(steps == [], "model plan contains no steps")
      |> add_issue(
        not contiguous_steps?(steps),
        "model plan step indexes must be contiguous and one-based"
      )

    with {:ok, plan_digest} <- ModelPlanApproval.compute_digest(plan) do
      gated =
        steps
        |> Enum.filter(&(&1["requires_confirmation"] == true))
        |> MapSet.new(&{&1["index"], &1["action"]})

      {approved, errors} = validate_approval(plan, plan_digest, approval, gated, errors)

      errors =
        Enum.reduce(MapSet.difference(gated, approved), errors, fn {index, action}, current ->
          ["step #{index} (#{action}) requires an exact caller-issued approval" | current]
        end)

      if errors == [],
        do: {:ok, plan_digest},
        else: {:error, Error.model_research_execution(errors |> Enum.uniq() |> Enum.sort())}
    end
  end

  defp validate_execution_request(_plan, _approval),
    do: validation_error("model plan must be a JSON object")

  defp validate_approval(_plan, _plan_digest, nil, _gated, errors),
    do: {MapSet.new(), errors}

  defp validate_approval(plan, plan_digest, approval, gated, errors) when is_map(approval) do
    errors =
      errors
      |> add_issue(
        approval["schema_version"] != @approval_schema,
        "unsupported model approval schema_version: #{inspect(approval["schema_version"])}"
      )
      |> add_issue(
        approval["session_id"] != plan["session_id"] or
          approval["workflow_id"] != plan["workflow_id"],
        "model approval does not match plan session and workflow"
      )
      |> add_issue(
        approval["plan_digest"] != plan_digest,
        "model approval plan_digest does not match the complete plan"
      )
      |> require_approval_string(approval, "approval_id")
      |> require_approval_string(approval, "authority")
      |> require_approval_string(approval, "issued_at")

    case approval["approved_steps"] do
      steps when is_list(steps) ->
        Enum.reduce(steps, {MapSet.new(), errors}, fn step, {approved, current} ->
          validate_approved_step(step, gated, approved, current)
        end)

      _ ->
        {MapSet.new(), ["model approval approved_steps must be an array" | errors]}
    end
  end

  defp validate_approval(_plan, _plan_digest, _approval, _gated, errors),
    do: {MapSet.new(), ["model approval must be a JSON object" | errors]}

  defp validate_approved_step(step, gated, approved, errors) when is_map(step) do
    key = {step["index"], step["action"]}

    cond do
      not is_integer(elem(key, 0)) or not is_binary(elem(key, 1)) ->
        {approved, ["model approval step requires integer index and string action" | errors]}

      MapSet.member?(approved, key) ->
        {approved, ["model approval repeats step #{elem(key, 0)} (#{elem(key, 1)})" | errors]}

      not MapSet.member?(gated, key) ->
        {MapSet.put(approved, key),
         [
           "model approval references a non-gated or mismatched step #{elem(key, 0)} (#{elem(key, 1)})"
           | errors
         ]}

      true ->
        {MapSet.put(approved, key), errors}
    end
  end

  defp validate_approved_step(_step, _gated, approved, errors),
    do: {approved, ["model approval steps must be JSON objects" | errors]}

  defp verify_approval(_plan, nil, _verifier), do: :ok

  defp verify_approval(plan, approval, verifier) do
    case verifier.(plan, approval) do
      true ->
        :ok

      :ok ->
        :ok

      {:ok, _evidence} ->
        :ok

      {:error, reason} ->
        validation_error("caller approval verifier rejected approval: #{bounded_error(reason)}")

      _ ->
        validation_error("caller approval verifier rejected approval")
    end
  end

  defp ensure_digest_unchanged(digest, digest), do: :ok

  defp ensure_digest_unchanged(_expected, _actual),
    do: validation_error("model plan changed after approval verification")

  defp control_plane(%Session{control_plane: nil}),
    do: {:error, Error.transport("control plane client is not configured")}

  defp control_plane(%Session{control_plane: client}), do: {:ok, client}

  defp control_plane(_session), do: {:error, Error.transport("invalid Headless session")}

  defp wrap_dispatch(authority, {:ok, output}), do: dispatch_result(authority, output)
  defp wrap_dispatch(_authority, {:error, reason}), do: {:error, reason}

  defp dispatch_result(authority, output),
    do: {:ok, %{"authority" => authority, "output" => output}}

  defp required_string(payload, key) when is_map(payload) do
    case payload[key] do
      value when is_binary(value) and value != "" -> {:ok, value}
      _ -> validation_error("model action payload requires non-empty string #{key}")
    end
  end

  defp required_string(_payload, key),
    do: validation_error("model action payload requires non-empty string #{key}")

  defp required_map(payload, key) when is_map(payload) do
    case payload[key] do
      value when is_map(value) -> {:ok, value}
      _ -> validation_error("model action payload #{key} must be a JSON object")
    end
  end

  defp required_map(_payload, key),
    do: validation_error("model action payload #{key} must be a JSON object")

  defp optional_unsigned(payload, key) when is_map(payload) do
    case payload[key] do
      nil -> {:ok, nil}
      value when is_integer(value) and value >= 0 -> {:ok, value}
      _ -> validation_error("model action payload #{key} must be an unsigned integer")
    end
  end

  defp require_approval_string(errors, approval, key) do
    add_issue(errors, not present_string?(approval[key]), "model approval #{key} is required")
  end

  defp present_string?(value), do: is_binary(value) and String.trim(value) != ""

  defp contiguous_steps?(steps) do
    steps
    |> Enum.with_index(1)
    |> Enum.all?(fn {step, index} -> is_map(step) and step["index"] == index end)
  end

  defp validate_wait_bounds(poll_interval, timeout) do
    cond do
      not is_integer(poll_interval) or not is_integer(timeout) or poll_interval <= 0 or
        timeout <= 0 or poll_interval > timeout ->
        validation_error("model dispatcher wait bounds require 0 < poll_interval <= timeout")

      timeout > 86_400_000 ->
        validation_error("model dispatcher timeout cannot exceed 24 hours")

      true ->
        :ok
    end
  end

  defp bounded_error(%{message: message}) when is_binary(message), do: truncate_error(message)
  defp bounded_error(reason), do: reason |> inspect(limit: 20) |> truncate_error()
  defp truncate_error(message) when byte_size(message) <= 2_048, do: message
  defp truncate_error(message), do: valid_prefix(message, 2_048) <> "..."

  defp valid_prefix(message, size) do
    prefix = binary_part(message, 0, size)
    if String.valid?(prefix), do: prefix, else: valid_prefix(message, size - 1)
  end

  defp add_issue(errors, true, message), do: [message | errors]
  defp add_issue(errors, false, _message), do: errors
  defp validation_error(message), do: {:error, Error.model_research_execution([message])}
end
