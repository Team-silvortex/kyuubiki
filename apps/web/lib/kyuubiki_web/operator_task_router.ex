defmodule KyuubikiWeb.OperatorTaskRouter do
  @moduledoc false

  use Plug.Router

  alias KyuubikiWeb.Orchestra.OperatorTaskEnvelope
  import KyuubikiWeb.RouterSupport

  plug(:match)
  plug(:dispatch)

  post "/prepare" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.prepare(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable_operator_task(conn, reason)
      end
    end)
  end

  post "/execute" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.execute(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable_operator_task(conn, reason)
      end
    end)
  end

  post "/prepare-batch" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.prepare_batch(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable_operator_task(conn, reason)
      end
    end)
  end

  post "/execute-batch" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.execute_batch(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable_operator_task(conn, reason)
      end
    end)
  end

  post "/checkpoint-batch" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.checkpoint_batch(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable_operator_task(conn, reason)
      end
    end)
  end

  post "/verify-checkpoint-batch" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.verify_checkpoint_batch(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable_operator_task(conn, reason)
      end
    end)
  end

  post "/resume-plan-batch" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.resume_plan_batch(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable_operator_task(conn, reason)
      end
    end)
  end

  match _ do
    Plug.Conn.send_resp(conn, 404, "not found")
  end

  defp unprocessable_operator_task(conn, reason) do
    respond_json(conn, 422, %{
      "error" => inspect(reason),
      "error_code" => operator_task_error_code(reason)
    })
  end

  defp operator_task_error_code({:operator_task_digest_mismatch, _mismatch}),
    do: "operator_task_digest_mismatch"

  defp operator_task_error_code({:operator_task_mirror_mismatch, _mismatch}),
    do: "operator_task_mirror_mismatch"

  defp operator_task_error_code(:missing_operator_task_digest),
    do: "operator_task_digest_missing"

  defp operator_task_error_code(:missing_operator_task), do: "operator_task_missing"
  defp operator_task_error_code(:missing_operator_task_batch), do: "operator_task_batch_missing"

  defp operator_task_error_code(:operator_task_execution_abi_mismatch),
    do: "operator_task_execution_abi_mismatch"

  defp operator_task_error_code(:operator_task_program_mismatch),
    do: "operator_task_program_mismatch"

  defp operator_task_error_code(:operator_task_entrypoint_mismatch),
    do: "operator_task_entrypoint_mismatch"

  defp operator_task_error_code(:invalid_operator_task_ir), do: "operator_task_invalid"
  defp operator_task_error_code(:invalid_operator_task_batch), do: "operator_task_batch_invalid"
  defp operator_task_error_code(reason) when is_atom(reason), do: Atom.to_string(reason)
  defp operator_task_error_code(_reason), do: "operator_task_error"
end
