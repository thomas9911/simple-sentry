INSERT INTO
    sentry_log (
        project_id,
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
        breadcrumbs,
        exception
    )
VALUES
    (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)