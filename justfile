default:
  @just --choose

fmt:
    cargo +nightly fmt

dev-db:
    sqlx database create
    sqlx migrate run
    cargo sqlx prepare

send-event-seed:
    @just send-event-seed-0
    @just send-event-seed-0
    @just send-event-seed-0
    @just send-event-seed-0
    @just send-event-seed-0

[private]
send-event-seed-0: 
    @just send-event-all
    @just send-event-all
    @just send-event-all
    @just send-event-all
    @just send-event-all

send-event-all: send-event send-event-file send-event-extra

send-event:
    #!/usr/bin/env sh
    ./sentry-cli send-event --no-environ -m "Hello from Sentry"

send-event-file:
    #!/usr/bin/env sh
    echo "$(date +%c) This is a log record" >> output.log
    echo "$(date +%c) This is another record" >> output.log
    ./sentry-cli send-event --no-environ -m "Demo Event" -t tag1:value --logfile output.log

send-event-extra:
    #!/usr/bin/env sh
    ./sentry-cli send-event -m "a failure" -e task:create-user -e object:42
