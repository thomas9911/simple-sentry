SELECT
    logentry,
    timestamp,
    level,
    environment,
    tags,
    breadcrumbs,
    exception
FROM
    sentry_log
WHERE
    id = ?;