defmodule KyuubikiWeb.CryptoErrorRedactionTest do
  use ExUnit.Case, async: true

  @secret String.duplicate("test-secret-not-for-production-", 3)
  @salt "test-redaction-salt"

  test "wrong-typed request tokens are rejected without raising secret-bearing errors" do
    for operation <- [:verify, :decrypt], token <- [[], ["x"], 1, %{}, {:token, "x"}] do
      assert {:error, :invalid} = apply(Plug.Crypto, operation, [@secret, @salt, token])
    end
  end

  test "invalid crypto arguments cannot expose secrets in formatted exception traces" do
    calls = [
      {:mask, [@secret, nil]},
      {:masked_compare, [nil, @secret, @salt]},
      {:secure_compare, [nil, @secret]}
    ]

    calls =
      calls ++
        for operation <- [:sign, :encrypt, :verify, :decrypt],
            args <- [[@secret, nil, "token", []], [@secret, @salt, "token", %{}]] do
          {operation, args}
        end

    for {operation, args} <- calls do
      formatted =
        try do
          apply(Plug.Crypto, operation, args)
          flunk("expected invalid #{operation} arguments to raise")
        rescue
          error in ArgumentError -> Exception.format(:error, error, __STACKTRACE__)
        end

      refute formatted =~ @secret
      refute formatted =~ @salt
    end
  end
end
