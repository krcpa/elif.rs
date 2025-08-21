# CLI Commands

Reference for `elif` commands and common options. See `elif --help` for the full list.

Project
- `elif new <name> [--path <dir>]` — scaffold a new application.
- `elif serve [--port 3000] [--host 127.0.0.1] [--hot-reload] [--watch <paths>...] [--exclude <glob>...] [--env development]` — start dev server.
- `elif check` — lint/check project.
- `elif test [--focus <Resource>]` — run tests.
- `elif map [--json]` — output project map.

Generation
- `elif make resource <Name> --fields name:type[,..] [--relationships name:type] [--api] [--tests] [--policy] [--requests] [--resources]` — full resource scaffold.
- `elif generate middleware <Name> [--debug] [--conditional] [--tests]` — middleware scaffold.
- `elif route add <METHOD> <path> <controller>` — add route entry. `elif route list` to list routes.
- `elif model add <Name> <fields>` — add a model with fields.
- `elif resource new <name> --route <path> --fields <...>` — create a resource spec.

Database
- `elif migrate create <name>` — create a migration file.
- `elif migrate run` — run pending migrations.
- `elif migrate rollback` — rollback last migration.
- `elif migrate status` — show status.
- `elif db seed [--env <environment>] [--force] [--verbose]` — run seeders.
- `elif db factory status|test [--count 3]` — factory diagnostics.

API & Docs
- `elif version create <v>` — create API version (e.g., `v1`).
- `elif version deprecate <v> [--message <msg>] [--sunset-date <iso8601>]` — deprecate a version.
- `elif version list` — list versions.
- `elif version migrate --from <v1> --to <v2>` — generate migration guide.
- `elif version validate` — check config.
- `elif openapi generate [--output <path>] [--format json|yaml]` — generate OpenAPI.
- `elif openapi export --format postman|insomnia --output <path>` — export.
- `elif openapi serve [--port 8080]` — serve Swagger UI.

Queues
- `elif queue work [--queue default] [--max-jobs N] [--timeout 60] [--sleep 1000] [--workers 1] [--stop-when-empty] [--verbose]`
- `elif queue status [--queue <name>] [--detailed] [--refresh 0]`
- `elif queue schedule [--time <ts>] [--frequency <unit>] [--job <name>] [--dry-run] [--force] [--verbose] [--daemon] [--check-interval 60]`

Email
- `elif email send <to> --subject <s> [--template <name>] [--body <text>] [--html] [--context <json>]`
- `elif email template list|validate <template>|render <template> [--context <json>] [--format html|text|both]`
- `elif email provider test [--provider <name>]|configure <provider> [--interactive]|switch <provider>`
- `elif email queue status|process [--limit N] [--timeout 30]|clear [--failed] [--completed]`
- `elif email track analytics [--range today|week|month] [--filter <id>]|stats [--group-by day|hour|provider|template]`
- `elif email setup [--provider <name>] [--non-interactive]`
- `elif email test capture [--enable|--disable] [--dir <path>]|list [--detailed] [--to <email>] [--subject <s>] [--limit 10]|show <id> [--raw] [--part headers|text|html|attachments]|clear --all|--older-than <days>|export [--format json|csv|mbox] [--output <path>] [--include-body]`
