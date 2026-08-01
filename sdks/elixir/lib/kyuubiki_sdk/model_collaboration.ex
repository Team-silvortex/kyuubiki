defmodule KyuubikiSdk.ModelCollaboration do
  @moduledoc "Provider-neutral model proposals over constrained Headless actions."

  alias KyuubikiSdk.Error
  alias KyuubikiSdk.ModelProviderAdapters

  @session_schema "kyuubiki.model-collaboration/v1"
  @proposal_schema "kyuubiki.model-workflow-proposal/v1"
  @plan_schema "kyuubiki.model-headless-plan/v1"

  def session_schema_version, do: @session_schema
  def proposal_schema_version, do: @proposal_schema
  def plan_schema_version, do: @plan_schema

  def default_policy do
    %{
      "allowed_actions" => [],
      "allowed_categories" => [],
      "max_steps" => 12,
      "max_context_bytes" => 64 * 1024,
      "service_only" => true,
      "allow_sensitive" => false,
      "allow_destructive" => false
    }
  end

  def tools(policy \\ %{}) when is_map(policy) do
    policy = Map.merge(default_policy(), policy)
    Enum.filter(base_tools(), &policy_allows_tool?(policy, &1))
  end

  def build_request(provider, session, context) do
    with {:ok, provider} <- ModelProviderAdapters.normalize_provider(provider),
         {:ok, session} <- validate_session(session) do
      {context, redacted_paths} = ModelProviderAdapters.sanitize_context(context)
      context_bytes = context |> Jason.encode!() |> byte_size()
      policy = session["policy"]
      available_tools = tools(policy)

      cond do
        context_bytes > policy["max_context_bytes"] ->
          validation_error(
            "sanitized model context uses #{context_bytes} bytes; policy allows #{policy["max_context_bytes"]}"
          )

        available_tools == [] ->
          validation_error("model collaboration policy exposes no Headless tools")

        true ->
          {:ok,
           %{
             "schema_version" => @session_schema,
             "provider" => provider,
             "session" => session,
             "instructions" => [
               "Plan only for this objective: #{session["objective"]}",
               "Use only supplied Headless tools and never invent action names.",
               "Return no more than #{policy["max_steps"]} tool calls.",
               "Return tool calls as an untrusted proposal; never claim that execution occurred."
             ],
             "context" => context,
             "redacted_paths" => redacted_paths,
             "tools" => ModelProviderAdapters.project_tools(provider, available_tools),
             "output_contract" => @proposal_schema
           }}
      end
    end
  end

  def build_plan(session, proposal) when is_map(proposal) do
    with {:ok, session} <- validate_session(session) do
      policy = session["policy"]
      available = Map.new(tools(policy), &{&1["action"], &1})
      {calls, initial_issues} = proposal_calls(proposal)

      issues =
        initial_issues
        |> add_issue(
          proposal["schema_version"] != @proposal_schema,
          "unsupported proposal schema_version: #{inspect(proposal["schema_version"])}"
        )
        |> add_issue(
          proposal["session_id"] != session["session_id"],
          "proposal session_id does not match collaboration session"
        )
        |> add_issue(calls == [], "model proposal contains no tool calls")
        |> add_issue(
          length(calls) > policy["max_steps"],
          "model proposal contains #{length(calls)} calls; policy allows #{policy["max_steps"]}"
        )

      {steps, issues} =
        calls
        |> Enum.with_index(1)
        |> Enum.map_reduce(issues, fn {call, index}, current_issues ->
          plan_step(index, call, available, current_issues)
        end)

      issues = issues |> Enum.uniq() |> Enum.sort()

      {:ok,
       %{
         "schema_version" => @plan_schema,
         "session_id" => session["session_id"],
         "workflow_id" => session["workflow_id"],
         "ok" => issues == [],
         "ready_without_confirmation" =>
           issues == [] and Enum.all?(steps, &(not &1["requires_confirmation"])),
         "issues" => issues,
         "steps" => steps
       }}
    end
  end

  def build_plan(_session, _proposal),
    do: validation_error("model proposal must be a JSON object")

  defdelegate project_tools(provider, available_tools), to: ModelProviderAdapters
  defdelegate normalize_response(provider, session_id, response), to: ModelProviderAdapters
  defdelegate sanitize_context(context), to: ModelProviderAdapters

  defp validate_session(session) when is_map(session) do
    raw_policy = session["policy"]
    policy = normalize_policy(raw_policy)

    errors =
      []
      |> add_issue(
        session["schema_version"] != @session_schema,
        "unsupported session schema_version: #{inspect(session["schema_version"])}"
      )
      |> require_session_string(session, "session_id")
      |> require_session_string(session, "workflow_id")
      |> require_session_string(session, "objective")
      |> require_session_string(session, "created_at")
      |> require_positive_integer(policy, "max_steps")
      |> require_positive_integer(policy, "max_context_bytes")
      |> require_list(policy, "allowed_actions")
      |> require_list(policy, "allowed_categories")
      |> add_policy_issues(raw_policy)

    if errors == [] do
      {:ok,
       session
       |> Map.put_new("language", "en")
       |> Map.put("policy", policy)}
    else
      {:error, Error.model_collaboration_validation(errors)}
    end
  end

  defp validate_session(_session),
    do: validation_error("model collaboration session must be a JSON object")

  defp normalize_policy(nil), do: default_policy()

  defp normalize_policy(policy) when is_map(policy) do
    Enum.reduce(default_policy(), default_policy(), fn {key, default}, normalized ->
      value = Map.get(policy, key, default)
      Map.put(normalized, key, if(valid_policy_value?(value, default), do: value, else: default))
    end)
  end

  defp normalize_policy(_policy), do: default_policy()

  defp plan_step(index, call, available, issues) when is_map(call) do
    action = call["action"]
    tool = if is_binary(action), do: available[action], else: nil
    payload = if is_map(call["payload"]), do: call["payload"], else: %{}

    issues =
      issues
      |> add_issue(not is_binary(action) or action == "", "step #{index} action is required")
      |> add_issue(
        is_nil(tool),
        "step #{index} action #{action || ""} is unknown or blocked by policy"
      )
      |> add_issue(
        not is_map(call["payload"]),
        "step #{index} (#{action || ""}) payload must be a JSON object"
      )
      |> add_missing_payload_issues(index, action, payload, tool)
      |> add_payload_shape_issues(index, action, payload, tool)

    risk = if tool, do: tool["risk"], else: "normal"

    {%{
       "index" => index,
       "action" => action || "",
       "category" => if(tool, do: tool["category"], else: nil),
       "risk" => risk,
       "payload" => payload,
       "requires_confirmation" => risk != "normal",
       "confirmation_reason" => confirmation_reason(risk),
       "output_keys" => if(tool, do: tool["output_keys"], else: [])
     }, issues}
  end

  defp plan_step(index, _call, _available, issues) do
    plan_step(index, %{}, %{}, ["step #{index} must be a JSON object" | issues])
  end

  defp proposal_calls(proposal) do
    case Map.get(proposal, "calls", []) do
      calls when is_list(calls) -> {calls, []}
      _ -> {[], ["model proposal calls must be an array"]}
    end
  end

  defp add_missing_payload_issues(issues, _index, _action, _payload, nil), do: issues

  defp add_missing_payload_issues(issues, index, action, payload, tool) do
    Enum.reduce(tool["required_payload_keys"], issues, fn key, current ->
      add_issue(
        current,
        not present_value?(payload[key]),
        "step #{index} (#{action}) is missing required payload key #{key}"
      )
    end)
  end

  defp add_payload_shape_issues(issues, _index, _action, _payload, nil), do: issues

  defp add_payload_shape_issues(issues, index, action, payload, _tool) do
    {string_keys, object_keys} =
      case action do
        action when action in ["fem_submit", "direct_solver_rpc"] ->
          {["solve_kind"], ["payload"]}

        "workflow_submit_catalog" ->
          {["workflow_id"], ["input_artifacts"]}

        "workflow_submit_graph" ->
          {[], ["graph", "input_artifacts"]}

        action when action in ["operator_task_prepare", "operator_task_execute"] ->
          {[], ["task"]}

        action when action in ["operator_task_batch_prepare", "operator_task_batch_execute"] ->
          {[], ["batch"]}

        action when action in ["job_wait", "result_fetch", "job_cancel"] ->
          {["job_id"], []}

        "result_chunk_fetch" ->
          {["job_id", "kind"], []}

        _ ->
          {[], []}
      end

    issues =
      Enum.reduce(string_keys, issues, fn key, current ->
        add_issue(
          current,
          Map.has_key?(payload, key) and
            (not is_binary(payload[key]) or String.trim(payload[key]) == ""),
          "step #{index} (#{action}) payload key #{key} must be a non-empty string"
        )
      end)

    issues =
      Enum.reduce(object_keys, issues, fn key, current ->
        add_issue(
          current,
          Map.has_key?(payload, key) and not is_map(payload[key]),
          "step #{index} (#{action}) payload key #{key} must be a JSON object"
        )
      end)

    if action == "result_chunk_fetch" do
      Enum.reduce(["offset", "limit"], issues, fn key, current ->
        value = payload[key]

        add_issue(
          current,
          not is_nil(value) and (not is_integer(value) or value < 0),
          "step #{index} (#{action}) payload key #{key} must be an unsigned integer"
        )
      end)
    else
      issues
    end
  end

  defp policy_allows_tool?(policy, tool) do
    (policy["allowed_actions"] == [] or tool["action"] in policy["allowed_actions"]) and
      (policy["allowed_categories"] == [] or tool["category"] in policy["allowed_categories"]) and
      (policy["allow_sensitive"] or tool["risk"] != "sensitive") and
      (policy["allow_destructive"] or tool["risk"] != "destructive") and
      (not policy["service_only"] or tool["runtime"] == "service")
  end

  defp require_session_string(errors, session, key) do
    add_issue(errors, not present_value?(session[key]), "#{key} is required")
  end

  defp require_positive_integer(errors, policy, key) do
    value = policy[key]
    add_issue(errors, not is_integer(value) or value <= 0, "#{key} must be a positive integer")
  end

  defp require_list(errors, policy, key) do
    add_issue(errors, not is_list(policy[key]), "#{key} must be an array")
  end

  defp add_policy_issues(errors, nil), do: errors

  defp add_policy_issues(errors, policy) when is_map(policy) do
    Enum.reduce(default_policy(), errors, fn {key, default}, current ->
      value = Map.get(policy, key, default)
      add_issue(current, not valid_policy_value?(value, default), policy_issue(key, default))
    end)
  end

  defp add_policy_issues(errors, _policy),
    do: ["model collaboration policy must be a JSON object" | errors]

  defp valid_policy_value?(value, default) when is_list(default),
    do: is_list(value) and Enum.all?(value, &is_binary/1)

  defp valid_policy_value?(value, default) when is_boolean(default), do: is_boolean(value)

  defp valid_policy_value?(value, default) when is_integer(default),
    do: is_integer(value) and value > 0

  defp policy_issue(key, default) when is_list(default), do: "#{key} must be an array of strings"
  defp policy_issue(key, default) when is_boolean(default), do: "#{key} must be a boolean"
  defp policy_issue(key, _default), do: "#{key} must be a positive integer"

  defp add_issue(errors, true, message), do: [message | errors]
  defp add_issue(errors, false, _message), do: errors

  defp present_value?(value) when is_binary(value), do: String.trim(value) != ""
  defp present_value?(value), do: not is_nil(value)

  defp confirmation_reason("sensitive"),
    do: "sensitive Headless action requires explicit approval before dispatch"

  defp confirmation_reason("destructive"),
    do: "destructive Headless action requires explicit approval before dispatch"

  defp confirmation_reason(_risk), do: nil

  defp base_tools do
    service = "service"

    [
      tool("service_health", "discovery", "Check control-plane health.", "normal", service, [], [
        "health"
      ]),
      tool(
        "protocol_describe",
        "discovery",
        "Read protocol compatibility and service endpoints.",
        "normal",
        service,
        [],
        ["protocol"]
      ),
      tool(
        "agents_describe",
        "discovery",
        "List reachable agents and capabilities.",
        "normal",
        service,
        [],
        ["agents"]
      ),
      tool(
        "workflow_catalog_list",
        "discovery",
        "List centrally owned workflow templates.",
        "normal",
        service,
        [],
        ["workflows"]
      ),
      tool(
        "operator_catalog_list",
        "discovery",
        "List workflow operator descriptors.",
        "normal",
        service,
        [],
        ["operators"]
      ),
      tool(
        "fem_submit",
        "solve",
        "Submit a FEM solve kind and model payload.",
        "sensitive",
        service,
        ["solve_kind", "payload"],
        ["job"]
      ),
      tool(
        "direct_solver_rpc",
        "solve",
        "Call a configured solver agent without Orchestra.",
        "sensitive",
        "direct",
        ["solve_kind", "payload"],
        ["result"]
      ),
      tool(
        "workflow_submit_catalog",
        "workflow",
        "Submit a catalog workflow job.",
        "sensitive",
        service,
        ["workflow_id", "input_artifacts"],
        ["job"]
      ),
      tool(
        "workflow_submit_graph",
        "workflow",
        "Submit a validated inline workflow graph.",
        "sensitive",
        service,
        ["graph", "input_artifacts"],
        ["job"]
      ),
      tool(
        "operator_task_prepare",
        "task_ir",
        "Preflight one language-neutral Operator TaskIR envelope.",
        "normal",
        service,
        ["task"],
        ["preparation"]
      ),
      tool(
        "operator_task_execute",
        "task_ir",
        "Execute one prepared Operator TaskIR envelope.",
        "sensitive",
        service,
        ["task"],
        ["execution"]
      ),
      tool(
        "operator_task_batch_prepare",
        "task_ir",
        "Preflight an Operator TaskIR batch.",
        "normal",
        service,
        ["batch"],
        ["preparation"]
      ),
      tool(
        "operator_task_batch_execute",
        "task_ir",
        "Execute an Operator TaskIR batch.",
        "sensitive",
        service,
        ["batch"],
        ["execution"]
      ),
      tool(
        "job_wait",
        "observation",
        "Poll a job until it reaches a terminal state.",
        "normal",
        service,
        ["job_id"],
        ["job"]
      ),
      tool(
        "result_fetch",
        "observation",
        "Fetch the retained result bundle for a job.",
        "normal",
        service,
        ["job_id"],
        ["result"]
      ),
      tool(
        "result_chunk_fetch",
        "observation",
        "Fetch one bounded result chunk.",
        "normal",
        service,
        ["job_id", "kind"],
        ["chunk"]
      ),
      tool(
        "job_cancel",
        "lifecycle",
        "Cancel a running job after explicit approval.",
        "destructive",
        service,
        ["job_id"],
        ["job"]
      )
    ]
  end

  defp tool(action, category, description, risk, runtime, required, outputs) do
    %{
      "action" => action,
      "category" => category,
      "description" => description,
      "risk" => risk,
      "runtime" => runtime,
      "required_payload_keys" => required,
      "output_keys" => outputs
    }
  end

  defp validation_error(message),
    do: {:error, Error.model_collaboration_validation([message])}
end
