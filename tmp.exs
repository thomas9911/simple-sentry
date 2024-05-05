Application.put_env(:sentry, :dsn, "http://test@localhost:8080/0")

Mix.install([:jason, :hackney, :sentry])
