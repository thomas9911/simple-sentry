#! /bin/bash
export SENTRY_DSN='http://test@localhost:8080/0'

# ./sentry-cli send-event -m "Hello from Sentry"
# ./sentry-cli send-event -m "a failure" -e task:create-user -e object:42

echo "$(date +%c) This is a log record" >> output.log
echo "$(date +%c) This is another record" >> output.log
./sentry-cli send-event --no-environ -m "Demo Event" -t tag1:value --logfile output.log
