CREATE TABLE sentry_log(
   id INTEGER PRIMARY KEY,
   timestamp INTEGER,
   logentry TEXT,
   level TEXT,
   environment TEXT,
   event_id TEXT,
   platform TEXT,
   server_name TEXT,
   sdk TEXT,
   user TEXT,
   tags TEXT,
   contexts TEXT,
   extra TEXT,
   breadcrumbs TEXT
);
