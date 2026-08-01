defmodule KyuubikiSdk.ModelProviderAdapters do
  @moduledoc false

  alias KyuubikiSdk.Error

  @proposal_schema "kyuubiki.model-workflow-proposal/v1"
  @providers ~w(openai openai_chat anthropic gemini canonical)

  def normalize_provider(provider) when is_atom(provider),
    do: provider |> Atom.to_string() |> normalize_provider()

  def normalize_provider(provider) when provider in @providers, do: {:ok, provider}

  def normalize_provider(provider),
    do: validation_error("unsupported model provider: #{inspect(provider)}")

  def project_tools(provider, available_tools) when provider in @providers do
    definitions =
      Enum.map(available_tools, fn tool ->
        parameters = tool_parameters(tool["required_payload_keys"])

        case provider do
          "openai" ->
            %{
              "type" => "function",
              "name" => tool["action"],
              "description" => tool["description"],
              "parameters" => parameters,
              "strict" => false
            }

          "openai_chat" ->
            %{
              "type" => "function",
              "function" => %{
                "name" => tool["action"],
                "description" => tool["description"],
                "parameters" => parameters,
                "strict" => false
              }
            }

          "anthropic" ->
            %{
              "name" => tool["action"],
              "description" => tool["description"],
              "input_schema" => parameters
            }

          "gemini" ->
            %{
              "name" => tool["action"],
              "description" => tool["description"],
              "parameters" => parameters
            }

          "canonical" ->
            tool
        end
      end)

    if provider == "gemini", do: [%{"functionDeclarations" => definitions}], else: definitions
  end

  def normalize_response(provider, session_id, response) when is_map(response) do
    with {:ok, provider} <- normalize_provider(provider) do
      case provider do
        "canonical" -> normalize_canonical(session_id, response)
        "openai" -> normalize_provider_calls(session_id, collect_openai(response))
        "openai_chat" -> normalize_provider_calls(session_id, collect_openai(response))
        "anthropic" -> normalize_provider_calls(session_id, collect_anthropic(response))
        "gemini" -> normalize_provider_calls(session_id, collect_gemini(response))
      end
    end
  end

  def normalize_response(_provider, _session_id, _response),
    do: validation_error("provider response must be a JSON object")

  def sanitize_context(context), do: sanitize_value(context, "", [])

  defp normalize_provider_calls(session_id, {raw_calls, summaries}) do
    with {:ok, calls} <- parse_calls(raw_calls) do
      if calls == [] do
        validation_error("provider response contains no supported tool calls")
      else
        {:ok,
         %{
           "schema_version" => @proposal_schema,
           "session_id" => session_id,
           "summary" => Enum.join(summaries, "\n"),
           "calls" => calls
         }}
      end
    end
  end

  defp normalize_canonical(session_id, response) do
    cond do
      response["session_id"] != session_id ->
        validation_error("canonical proposal session_id does not match requested session")

      response["schema_version"] != @proposal_schema ->
        validation_error("canonical proposal has unsupported schema_version")

      not is_list(response["calls"]) ->
        validation_error("canonical proposal calls must be an array")

      true ->
        result =
          Enum.reduce_while(response["calls"], {:ok, []}, fn call, {:ok, calls} ->
            if is_map(call) do
              case parse_arguments(Map.get(call, "payload", %{})) do
                {:ok, payload} -> {:cont, {:ok, [Map.put(call, "payload", payload) | calls]}}
                {:error, error} -> {:halt, {:error, error}}
              end
            else
              {:halt, validation_error("canonical proposal calls must be JSON objects")}
            end
          end)

        case result do
          {:ok, calls} -> {:ok, Map.put(response, "calls", Enum.reverse(calls))}
          error -> error
        end
    end
  end

  defp collect_openai(response) do
    output = response |> Map.get("output") |> map_list()
    choices = response |> Map.get("choices") |> map_list()

    output_calls =
      for item <- output, item["type"] == "function_call" do
        {item["call_id"] || item["id"], item["name"], item["arguments"]}
      end

    output_summaries =
      for item <- output,
          item["type"] == "message",
          part <- item |> Map.get("content") |> map_list(),
          part["type"] in ["output_text", "text"],
          text = clean_text(part["text"]),
          text != nil,
          do: text

    choice_calls =
      for choice <- choices,
          message = choice["message"],
          is_map(message),
          call <- message["tool_calls"] |> map_list(),
          function = call["function"] || %{} do
        {call["id"], function["name"], function["arguments"]}
      end

    choice_summaries =
      choices
      |> Enum.map(fn choice ->
        message = choice["message"]
        clean_text(if(is_map(message), do: message["content"], else: nil))
      end)
      |> Enum.reject(&is_nil/1)

    {output_calls ++ choice_calls, output_summaries ++ choice_summaries}
  end

  defp collect_anthropic(response) do
    blocks = response |> Map.get("content") |> map_list()

    calls =
      for block <- blocks, block["type"] == "tool_use" do
        {block["id"], block["name"], block["input"]}
      end

    summaries =
      for block <- blocks,
          block["type"] == "text",
          text = clean_text(block["text"]),
          text != nil,
          do: text

    {calls, summaries}
  end

  defp collect_gemini(response) do
    parts =
      for candidate <- response |> Map.get("candidates") |> map_list(),
          content = candidate["content"],
          is_map(content),
          part <- content["parts"] |> map_list(),
          do: part

    candidate_calls =
      for part <- parts, function = part["functionCall"], is_map(function) do
        {function["id"], function["name"], function["args"]}
      end

    step_calls =
      for step <- response |> Map.get("steps") |> map_list(),
          step["type"] == "function_call" do
        {step["id"], step["name"], step["arguments"]}
      end

    summaries = parts |> Enum.map(&clean_text(&1["text"])) |> Enum.reject(&is_nil/1)
    {candidate_calls ++ step_calls, summaries}
  end

  defp parse_calls(raw_calls) do
    raw_calls
    |> Enum.reduce_while({:ok, []}, fn {id, name, arguments}, {:ok, calls} ->
      case parse_call(id, name, arguments) do
        {:ok, call} -> {:cont, {:ok, [call | calls]}}
        {:error, error} -> {:halt, {:error, error}}
      end
    end)
    |> then(fn
      {:ok, calls} -> {:ok, Enum.reverse(calls)}
      error -> error
    end)
  end

  defp parse_call(id, name, arguments) when is_binary(name) and name != "" do
    if is_nil(arguments) do
      validation_error("provider tool call #{name} is missing arguments")
    else
      with {:ok, payload} <- parse_arguments(arguments) do
        {:ok,
         %{
           "id" => if(is_binary(id), do: id, else: nil),
           "action" => name,
           "payload" => payload,
           "reason" => nil
         }}
      end
    end
  end

  defp parse_call(_id, _name, _arguments),
    do: validation_error("provider tool call is missing a string name")

  defp parse_arguments(arguments) when is_binary(arguments) do
    case Jason.decode(arguments) do
      {:ok, decoded} -> parse_arguments(decoded)
      {:error, _} -> validation_error("provider tool arguments are invalid JSON")
    end
  end

  defp parse_arguments(arguments) when is_map(arguments), do: {:ok, arguments}

  defp parse_arguments(_arguments),
    do: validation_error("provider tool arguments must decode to a JSON object")

  defp sanitize_value(value, path, paths) when is_map(value) do
    Enum.reduce(value, {%{}, paths}, fn {key, child}, {result, current_paths} ->
      key = to_string(key)
      next_path = path <> "/" <> escape_pointer(key)

      if sensitive_key?(key) do
        {Map.put(result, key, "[REDACTED]"), [next_path | current_paths]}
      else
        {sanitized, next_paths} = sanitize_value(child, next_path, current_paths)
        {Map.put(result, key, sanitized), next_paths}
      end
    end)
    |> then(fn {result, current_paths} -> {result, Enum.reverse(current_paths)} end)
  end

  defp sanitize_value(value, path, paths) when is_list(value) do
    value
    |> Enum.with_index()
    |> Enum.map_reduce(paths, fn {child, index}, current_paths ->
      sanitize_value(child, path <> "/" <> Integer.to_string(index), current_paths)
    end)
  end

  defp sanitize_value(value, path, paths) when is_binary(value) do
    if value |> String.trim_leading() |> String.downcase() |> String.starts_with?("bearer ") do
      {"[REDACTED]", [if(path == "", do: "/", else: path) | paths]}
    else
      {value, paths}
    end
  end

  defp sanitize_value(value, _path, paths), do: {value, paths}

  defp tool_parameters(required_keys) do
    %{
      "type" => "object",
      "properties" =>
        Map.new(required_keys, &{&1, %{"description" => "Required `#{&1}` payload"}}),
      "required" => required_keys,
      "additionalProperties" => true
    }
  end

  defp clean_text(value) when is_binary(value) do
    case String.trim(value) do
      "" -> nil
      text -> text
    end
  end

  defp clean_text(_value), do: nil

  defp map_list(value) when is_list(value), do: Enum.filter(value, &is_map/1)
  defp map_list(_value), do: []

  defp sensitive_key?(key) do
    normalized = key |> String.downcase() |> String.replace(["-", "."], "_")

    Enum.any?(
      ~w(token secret password api_key apikey authorization credential private_key),
      &String.contains?(normalized, &1)
    )
  end

  defp escape_pointer(value), do: value |> String.replace("~", "~0") |> String.replace("/", "~1")

  defp validation_error(message),
    do: {:error, Error.model_collaboration_validation([message])}
end
