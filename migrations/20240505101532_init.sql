CREATE TABLE sentry_log(
   id INTEGER PRIMARY KEY,
   project_id INTEGER NOT NULL,
   timestamp INTEGER NOT NULL,
   logentry TEXT NOT NULL,
   level TEXT NOT NULL,
   event_id TEXT NOT NULL,
   environment TEXT,
   platform TEXT,
   server_name TEXT,
   sdk TEXT,
   user TEXT,
   tags TEXT,
   contexts TEXT,
   extra TEXT,
   breadcrumbs TEXT,
   exception TEXT
);
