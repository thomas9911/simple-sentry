SELECT
    id,
    project_id,
    "timestamp",
    logentry,
    "level",
    event_id
FROM
    sentry_log
WHERE
    id < ?
ORDER BY
    id DESC
LIMIT
    ?;
