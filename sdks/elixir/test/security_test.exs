defmodule KyuubikiSdk.SecurityTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.Auth
  alias KyuubikiSdk.ControlPlaneClient

  test "auth inspection redacts and rejects header injection" do
    token = "private-elixir-sdk-token"
    auth = Auth.access_token(token)

    refute inspect(auth) =~ token
    assert :ok = Auth.validate(auth)

    assert {:error, "invalid authentication header value"} =
             Auth.validate(Auth.access_token("token\r\nX-Injected: yes"))

    client = ControlPlaneClient.new("http://127.0.0.1:9", auth: Auth.access_token("bad\r\ntoken"))
    assert {:error, error} = ControlPlaneClient.health(client)
    assert error.type == :transport
    refute error.message =~ "bad"
  end

  test "auth rejects unsafe custom header names and empty values" do
    assert {:error, "invalid authentication header name"} =
             Auth.validate(%Auth{header_name: "X-Test\r\nInjected", header_value: "token"})

    assert {:error, "invalid authentication header value"} =
             Auth.validate(Auth.access_token(""))
  end
end
