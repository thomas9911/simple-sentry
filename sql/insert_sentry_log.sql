INSERT INTO
    sentry_log (
        timestamp,
        logentry,
        level,
        environment,
        event_id,
        platform,
        server_name,
        sdk,
        user,
        tags,
        contexts,
        extra,
        breadcrumbs
    )
VALUES
    (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)