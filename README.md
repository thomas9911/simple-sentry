Super simple sentry app

## why

Normal sentry is opensource and self-hostable. However you need quite some resources to run. For some local testing it could be a bit overkill.

This project provides a very basic setup for running sentry-like service. It's not meant to be used in production, but rather for simple local development.

## optionally create projects from env variable on startup

```
SIMPLE_SENTRY_PROJECTS='1=project1;2=project2;3=project3' 
```

project names can even have '=' signs in their name like:

```
SIMPLE_SENTRY_PROJECTS='1=project1;2=project=2' 
```

## dev

It uses `just` cli to run some common tasks.

### fmt

```sh
just fmt
```

alias for `cargo +nightly format`

### dev-db

```sh
just dev-db
```

creates a local sqlite db. This is used by sqlx library to validate macros.

## send test events

```sh
just send-event
# send multiple
just send-event-all
```
