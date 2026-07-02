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
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  post "/execute" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.execute(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  post "/prepare-batch" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.prepare_batch(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  post "/execute-batch" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.execute_batch(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  post "/checkpoint-batch" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.checkpoint_batch(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  post "/verify-checkpoint-batch" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.verify_checkpoint_batch(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  post "/resume-plan-batch" do
    with_auth(conn, :write, fn conn ->
      case OperatorTaskEnvelope.resume_plan_batch(conn.body_params) do
        {:ok, payload} -> respond_json(conn, 200, payload)
        {:error, reason} -> unprocessable(conn, reason)
      end
    end)
  end

  match _ do
    Plug.Conn.send_resp(conn, 404, "not found")
  end
end
