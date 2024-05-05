SELECT
    id,
    "timestamp",
    logentry,
    "level",
    event_id
FROM
    sentry_log
WHERE
    id < ?
ORDER BY
    "timestamp" DESC
LIMIT
    ?;