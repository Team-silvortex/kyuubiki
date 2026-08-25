defmodule KyuubikiWeb.Playground.AgentRpcTransport do
  @moduledoc """
  Bounded TCP transport for Agent RPC frames.

  Transport, JSON, and local progress-consumer failures stay distinct so the
  control plane never penalizes a healthy Agent for a local callback defect.
  """

  alias KyuubikiWeb.Playground.AgentClient

  @rpc_version 1
  @default_connect_timeout_ms 1_500
  @default_recv_timeout_ms 15_000
  @default_request_timeout_ms 120_000
  @default_max_rpc_frame_bytes 512 * 1024 * 1024

  @spec request(map(), String.t(), map(), (map() -> any()), keyword()) ::
          {:ok, map()} | {:error, term()}
  def request(endpoint, request_id, request, on_progress, opts)
      when is_map(endpoint) and is_binary(request_id) and is_map(request) and
             is_function(on_progress, 1) and is_list(opts) do
    with :ok <- validate_request(request) do
      case connect(endpoint) do
        {:ok, socket} ->
          try do
            request_over_socket(socket, request_id, request, on_progress, opts)
          after
            :gen_tcp.close(socket)
          end

        {:error, reason} ->
          {:error, {:agent_transport_failure, :connect, reason}}
      end
    end
  end

  @spec validate_request(map()) :: :ok | {:error, term()}
  def validate_request(request) when is_map(request) do
    case encode_request(request) do
      {:ok, _payload} -> :ok
      {:error, _reason} = error -> error
    end
  end

  @spec emit_progress((map() -> any()), map()) :: :ok | {:error, term()}
  def emit_progress(on_progress, progress)
      when is_function(on_progress, 1) and is_map(progress) do
    case invoke_progress(on_progress, progress) do
      {:ok, {:error, reason}} -> progress_callback_error(:returned_error, reason)
      {:ok, _result} -> :ok
      {:error, kind, reason} -> progress_callback_error(kind, reason)
    end
  end

  def emit_progress(_on_progress, _progress),
    do: progress_callback_error(:invalid_callback, :invalid_progress_consumer)

  @spec local_failure?(term()) :: boolean()
  def local_failure?({:progress_callback_failed, _kind, _detail}), do: true
  def local_failure?({:request_encoding_failed, _detail}), do: true
  def local_failure?(_reason), do: false

  @spec request_timeout_ms(keyword()) :: pos_integer()
  def request_timeout_ms(opts) when is_list(opts) do
    case Keyword.get(opts, :request_timeout_ms) do
      value when is_integer(value) and value > 0 -> value
      _ -> configured_value(:request_timeout_ms, @default_request_timeout_ms)
    end
  end

  defp request_over_socket(socket, request_id, request, on_progress, opts) do
    with :ok <- tag_transport_error(send_request(socket, request), :send),
         {:ok, response_payload} <-
           tag_transport_error(
             recv_response(socket, request_id, on_progress, request_deadline_ms(opts)),
             :receive
           ) do
      case decode_response(response_payload, request_id) do
        {:error, {:invalid_response, _reason} = reason} ->
          {:error, {:agent_transport_failure, :protocol, reason}}

        result ->
          result
      end
    end
  end

  defp tag_transport_error({:error, {:request_encoding_failed, _detail}} = error, _stage),
    do: error

  defp tag_transport_error(
         {:error, {:progress_callback_failed, _kind, _detail}} = error,
         _stage
       ),
       do: error

  defp tag_transport_error({:error, {:invalid_response, _reason} = reason}, _stage),
    do: {:error, {:agent_transport_failure, :protocol, reason}}

  defp tag_transport_error({:error, reason}, stage),
    do: {:error, {:agent_transport_failure, stage, reason}}

  defp tag_transport_error(result, _stage), do: result

  defp connect(%{host: host, port: port})
       when is_binary(host) and is_integer(port) and port > 0 and port <= 65_535 do
    :gen_tcp.connect(
      String.to_charlist(host),
      port,
      [
        :binary,
        packet: 4,
        packet_size: max_rpc_frame_bytes(),
        active: false
      ],
      configured_value(:connect_timeout_ms, @default_connect_timeout_ms)
    )
  end

  defp connect(_endpoint), do: {:error, :invalid_endpoint}

  defp send_request(socket, request) do
    with {:ok, payload} <- encode_request(request) do
      :gen_tcp.send(socket, payload)
    end
  end

  defp encode_request(request) do
    case Jason.encode(request) do
      {:ok, payload} -> {:ok, payload}
      {:error, error} -> {:error, {:request_encoding_failed, Exception.message(error)}}
    end
  rescue
    error -> {:error, {:request_encoding_failed, Exception.message(error)}}
  end

  defp recv_response(socket, request_id, on_progress, deadline_ms) do
    case remaining_recv_timeout_ms(deadline_ms) do
      0 -> {:error, :request_timeout}
      timeout_ms -> recv_response_frame(socket, request_id, on_progress, deadline_ms, timeout_ms)
    end
  end

  defp recv_response_frame(socket, request_id, on_progress, deadline_ms, timeout_ms) do
    case :gen_tcp.recv(socket, 0, timeout_ms) do
      {:ok, payload} -> decode_frame(socket, payload, request_id, on_progress, deadline_ms)
      {:error, reason} -> {:error, reason}
    end
  end

  defp decode_frame(socket, payload, request_id, on_progress, deadline_ms) do
    case Jason.decode(payload) do
      {:ok, %{"event" => event, "rpc_version" => @rpc_version, "id" => ^request_id} = frame}
      when event in ["progress", "heartbeat"] ->
        with %{} = progress <- frame["progress"],
             :ok <- emit_progress(on_progress, progress) do
          recv_response(socket, request_id, on_progress, deadline_ms)
        else
          nil -> {:error, {:invalid_response, :missing_progress_payload}}
          {:error, _reason} = error -> error
          _invalid -> {:error, {:invalid_response, :invalid_progress_payload}}
        end

      {:ok, %{"rpc_version" => @rpc_version, "id" => ^request_id}} ->
        {:ok, payload}

      {:ok, _frame} ->
        {:error, {:invalid_response, :unexpected_rpc_frame}}

      {:error, reason} ->
        {:error, {:invalid_response, reason}}
    end
  end

  defp decode_response(raw_response, request_id) do
    case Jason.decode(raw_response) do
      {:ok, %{"rpc_version" => @rpc_version, "id" => ^request_id, "ok" => true} = decoded} ->
        {:ok, decoded["result"]}

      {:ok, %{"rpc_version" => @rpc_version, "id" => ^request_id, "ok" => false} = decoded} ->
        error = Map.get(decoded, "error", %{})

        if is_map(error),
          do: {:error, {:rpc_error, error["code"], error["message"]}},
          else: {:error, {:invalid_response, :malformed_rpc_error}}

      {:ok, _decoded} ->
        {:error, {:invalid_response, :malformed_rpc_response}}

      {:error, reason} ->
        {:error, {:invalid_response, reason}}
    end
  end

  defp invoke_progress(on_progress, progress) do
    {:ok, on_progress.(progress)}
  rescue
    error -> {:error, :error, Exception.message(error)}
  catch
    kind, reason -> {:error, kind, inspect(reason)}
  end

  defp progress_callback_error(kind, reason),
    do: {:error, {:progress_callback_failed, kind, callback_failure_detail(reason)}}

  defp callback_failure_detail(reason) when is_binary(reason), do: reason
  defp callback_failure_detail(reason), do: inspect(reason)

  defp request_deadline_ms(opts) do
    System.monotonic_time(:millisecond) + request_timeout_ms(opts)
  end

  defp remaining_recv_timeout_ms(deadline_ms) do
    remaining_ms = deadline_ms - System.monotonic_time(:millisecond)

    if remaining_ms <= 0,
      do: 0,
      else: min(configured_value(:recv_timeout_ms, @default_recv_timeout_ms), remaining_ms)
  end

  defp max_rpc_frame_bytes do
    configured_value(:max_rpc_frame_bytes, @default_max_rpc_frame_bytes)
  end

  defp configured_value(key, default) do
    case Application.get_env(:kyuubiki_web, AgentClient, []) |> Keyword.get(key, default) do
      value when is_integer(value) and value > 0 -> value
      _ -> default
    end
  end
end
