defmodule KyuubikiSdk.ModelResearchBootstrap do
  @moduledoc "Fail-closed document-to-research planning preflight."

  alias KyuubikiSdk.Error

  @bootstrap_schema_version "kyuubiki.model-research-bootstrap/v1"
  @report_schema_version "kyuubiki.model-research-readiness-report/v1"
  @sdks ~w(rust python elixir)

  def bootstrap_schema_version, do: @bootstrap_schema_version
  def report_schema_version, do: @report_schema_version

  def inspect(bootstrap, sdk, resource_exists) when is_function(resource_exists, 1) do
    with {:ok, sdk} <- normalize_sdk(sdk) do
      {:ok, build_report(bootstrap, sdk, resource_exists)}
    end
  end

  def inspect(_bootstrap, _sdk, _resource_exists),
    do: {:error, Error.model_research_bootstrap(["resource_exists must be callable"])}

  defp build_report(bootstrap, sdk, _resolver) when not is_map(bootstrap),
    do: empty_report(sdk, "bootstrap must be a JSON object")

  defp build_report(bootstrap, sdk, resolver) do
    blockers =
      []
      |> require(
        bootstrap["schema_version"] == @bootstrap_schema_version,
        "schema_version must be #{@bootstrap_schema_version}"
      )

    version_line = text(bootstrap, "version_line") || "unknown"
    entrypoint = text(bootstrap, "entrypoint") || "unknown"
    first = object(bootstrap["first_research"])

    blockers =
      blockers
      |> require(not is_nil(first), "first_research must be a JSON object")
      |> require(
        text(first || %{}, "reliability_posture") == "screening_only",
        "first_research.reliability_posture must be screening_only"
      )

    workflow_id = text(first || %{}, "workflow_id") || "unknown"
    hard_rules = string_list(bootstrap["hard_rules"])
    stop_conditions = string_list(bootstrap["stop_conditions"])
    {completion, blockers} = completion_contract(bootstrap, blockers)
    protocol = bootstrap["research_protocol"]

    blockers =
      blockers
      |> require(length(hard_rules) >= 8, "hard_rules must contain at least 8 non-empty rules")
      |> require(
        length(stop_conditions) >= 4,
        "stop_conditions must contain at least 4 non-empty rules"
      )
      |> require(
        is_list(protocol) and length(protocol) >= 6,
        "research_protocol must contain at least 6 stages"
      )

    {resources, blockers} = add_path(MapSet.new(), blockers, entrypoint, "entrypoint")
    {resources, blockers} = add_document_paths(bootstrap, resources, blockers)

    {surface, resources, blockers} =
      selected_surface(bootstrap, sdk, resources, blockers)

    {resources, blockers} = add_execution_resources(bootstrap, resources, blockers)
    {resources, blockers} = add_first_resources(first, resources, blockers)
    {resources, blockers} = add_preflight_resources(bootstrap, resources, blockers)
    required_resources = resources |> MapSet.to_list() |> Enum.sort()
    missing_resources = Enum.reject(required_resources, &resolver_exists?(resolver, &1))

    blockers =
      blockers
      |> Kernel.++(Enum.map(missing_resources, &"missing required resource: #{&1}"))
      |> Enum.uniq()
      |> Enum.sort()

    %{
      "schema_version" => @report_schema_version,
      "selected_sdk" => sdk,
      "ready_for_planning" => blockers == [] and not is_nil(surface),
      "execution_authority" => "none_preflight_only",
      "version_line" => version_line,
      "entrypoint" => entrypoint,
      "workflow_id" => workflow_id,
      "selected_surface" => surface,
      "required_resources" => required_resources,
      "missing_resources" => missing_resources,
      "blockers" => blockers,
      "hard_rules" => hard_rules,
      "stop_conditions" => stop_conditions,
      "completion_contract" => completion
    }
  end

  defp selected_surface(bootstrap, sdk, resources, blockers) do
    collaboration = nested_object(bootstrap, ["sdk_surfaces", sdk])
    preflight = nested_object(bootstrap, ["preflight", "surfaces", sdk])
    execution_root = object(bootstrap["execution_contract"])
    execution = nested_object(execution_root || %{}, ["surfaces", sdk])

    blockers =
      require(
        blockers,
        text(execution_root || %{}, "approval_authority") == "caller_only",
        "execution_contract.approval_authority must be caller_only"
      )

    if is_nil(collaboration) or is_nil(preflight) or is_nil(execution) do
      {nil, resources, ["selected SDK surface is missing: #{sdk}" | blockers]}
    else
      definitions = [
        {"collaboration_path", collaboration, "path", "sdk_surfaces.#{sdk}.path", true},
        {"preflight_path", preflight, "path", "preflight.surfaces.#{sdk}.path", true},
        {"execution_path", execution, "path", "execution_contract.surfaces.#{sdk}.path", true},
        {"frontier_path", execution, "frontier_path",
         "execution_contract.surfaces.#{sdk}.frontier_path", true},
        {"validation_path", execution, "validation_path",
         "execution_contract.surfaces.#{sdk}.validation_path", true},
        {"request", collaboration, "request", "sdk_surfaces.#{sdk}.request", false},
        {"inspect", preflight, "inspect", "preflight.surfaces.#{sdk}.inspect", false},
        {"normalize", collaboration, "normalize", "sdk_surfaces.#{sdk}.normalize", false},
        {"plan", collaboration, "plan", "sdk_surfaces.#{sdk}.plan", false},
        {"executor", execution, "executor", "execution_contract.surfaces.#{sdk}.executor", false},
        {"dispatcher", execution, "dispatcher", "execution_contract.surfaces.#{sdk}.dispatcher",
         false},
        {"approval_verifier", execution, "approval_verifier",
         "execution_contract.surfaces.#{sdk}.approval_verifier", false},
        {"frontier_start", execution, "frontier_start",
         "execution_contract.surfaces.#{sdk}.frontier_start", false},
        {"frontier_advance", execution, "frontier_advance",
         "execution_contract.surfaces.#{sdk}.frontier_advance", false},
        {"result_validator", execution, "result_validator",
         "execution_contract.surfaces.#{sdk}.result_validator", false},
        {"receipt_verifier", execution, "receipt_verifier",
         "execution_contract.surfaces.#{sdk}.receipt_verifier", false},
        {"frontier_verifier", execution, "frontier_verifier",
         "execution_contract.surfaces.#{sdk}.frontier_verifier", false}
      ]

      {values, resources, blockers, complete?} =
        Enum.reduce(definitions, {%{}, resources, blockers, true}, fn
          {output, source, key, label, path?}, {values, resources, blockers, complete?} ->
            case text(source, key) do
              nil ->
                {values, resources, ["#{label} must be a non-empty string" | blockers], false}

              value when path? ->
                {resources, blockers} = add_path(resources, blockers, value, label)
                {Map.put(values, output, value), resources, blockers, complete?}

              value ->
                {Map.put(values, output, value), resources, blockers, complete?}
            end
        end)

      {if(complete?, do: values, else: nil), resources, blockers}
    end
  end

  defp add_document_paths(bootstrap, resources, blockers) do
    case bootstrap["required_documents"] do
      documents when is_list(documents) ->
        blockers =
          require(
            blockers,
            length(documents) >= 4,
            "required_documents must contain at least 4 entries"
          )

        documents
        |> Enum.with_index()
        |> Enum.reduce({resources, blockers}, fn {document, index}, {resources, blockers} ->
          add_path(
            resources,
            blockers,
            text(object(document) || %{}, "path") || "",
            "required_documents[#{index}].path"
          )
        end)

      _ ->
        {resources, ["required_documents must be an array" | blockers]}
    end
  end

  defp add_execution_resources(bootstrap, resources, blockers) do
    case object(bootstrap["execution_contract"]) do
      nil ->
        {resources, ["execution_contract must be a JSON object" | blockers]}

      execution ->
        Enum.reduce(
          ~w(approval_schema approval_fixture receipt_schema frontier_schema frontier_fixture validation_report_schema validation_report_fixture),
          {resources, blockers},
          fn key, {resources, blockers} ->
            add_path(resources, blockers, text(execution, key) || "", "execution_contract.#{key}")
          end
        )
    end
  end

  defp add_first_resources(nil, resources, blockers), do: {resources, blockers}

  defp add_first_resources(first, resources, blockers) do
    Enum.reduce(
      ~w(session_fixture proposal_fixture catalog_request_fixture),
      {resources, blockers},
      fn key, {resources, blockers} ->
        add_path(resources, blockers, text(first, key) || "", "first_research.#{key}")
      end
    )
  end

  defp add_preflight_resources(bootstrap, resources, blockers) do
    case object(bootstrap["preflight"]) do
      nil ->
        {resources, ["preflight must be a JSON object" | blockers]}

      preflight ->
        blockers =
          require(
            blockers,
            text(preflight, "execution_authority") == "none_preflight_only",
            "preflight.execution_authority must be none_preflight_only"
          )

        Enum.reduce(~w(report_schema report_fixture), {resources, blockers}, fn key,
                                                                                {resources,
                                                                                 blockers} ->
          add_path(resources, blockers, text(preflight, key) || "", "preflight.#{key}")
        end)
    end
  end

  defp add_path(resources, blockers, path, label) do
    if safe_repo_path?(path),
      do: {MapSet.put(resources, path), blockers},
      else: {resources, ["#{label} must be a safe project-relative path" | blockers]}
  end

  defp safe_repo_path?(path) when is_binary(path) and path != "" do
    not String.starts_with?(path, "/") and not String.contains?(path, "\\") and
      Enum.all?(String.split(path, "/"), &(&1 not in ["", ".."]))
  end

  defp safe_repo_path?(_path), do: false

  defp resolver_exists?(resolver, path) do
    try do
      resolver.(path) == true
    rescue
      _ -> false
    catch
      _, _ -> false
    end
  end

  defp normalize_sdk(sdk) when is_atom(sdk), do: normalize_sdk(Atom.to_string(sdk))

  defp normalize_sdk(sdk) when sdk in @sdks, do: {:ok, sdk}

  defp normalize_sdk(_sdk),
    do: {:error, Error.model_research_bootstrap(["selected_sdk must be rust, python, or elixir"])}

  defp require(blockers, true, _message), do: blockers
  defp require(blockers, false, message), do: [message | blockers]
  defp object(value) when is_map(value), do: value
  defp object(_value), do: nil

  defp nested_object(value, []), do: object(value)

  defp nested_object(value, [key | rest]) when is_map(value),
    do: nested_object(value[key], rest)

  defp nested_object(_value, _keys), do: nil

  defp text(root, key) when is_map(root) do
    case root[key] do
      value when is_binary(value) ->
        if(String.trim(value) == "", do: nil, else: String.trim(value))

      _ ->
        nil
    end
  end

  defp string_list(value) when is_list(value) do
    value
    |> Enum.filter(&is_binary/1)
    |> Enum.map(&String.trim/1)
    |> Enum.reject(&(&1 == ""))
  end

  defp string_list(_value), do: []

  defp completion_contract(bootstrap, blockers) do
    case object(bootstrap["completion_contract"]) do
      nil ->
        {nil, ["completion_contract must be a JSON object" | blockers]}

      completion ->
        blockers =
          [
            {"required_artifacts", 3},
            {"required_claims", 3},
            {"forbidden_claims", 2}
          ]
          |> Enum.reduce(blockers, fn {key, minimum}, blockers ->
            require(
              blockers,
              length(string_list(completion[key])) >= minimum,
              "completion_contract.#{key} must contain at least #{minimum} entries"
            )
          end)

        {completion, blockers}
    end
  end

  defp empty_report(sdk, blocker) do
    %{
      "schema_version" => @report_schema_version,
      "selected_sdk" => sdk,
      "ready_for_planning" => false,
      "execution_authority" => "none_preflight_only",
      "version_line" => "unknown",
      "entrypoint" => "unknown",
      "workflow_id" => "unknown",
      "selected_surface" => nil,
      "required_resources" => [],
      "missing_resources" => [],
      "blockers" => [blocker],
      "hard_rules" => [],
      "stop_conditions" => [],
      "completion_contract" => nil
    }
  end
end
