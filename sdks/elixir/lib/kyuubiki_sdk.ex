defmodule KyuubikiSdk do
  @moduledoc "Protocol-first headless SDK entry point."

  alias KyuubikiSdk.Session

  def new_session(opts \\ []), do: Session.new(opts)
end
