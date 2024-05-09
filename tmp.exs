Application.put_env(:sentry, :dsn, "http://test@localhost:8080/0")

Mix.install([:jason, :hackney, :sentry])


Sentry.capture_message("hallo") |> IO.inspect


try do
  2 / 0
rescue
  e ->
    Sentry.capture_exception(e)
end
