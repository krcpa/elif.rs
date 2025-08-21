# CLI Commands

Reference for `elifrs` commands and common options. See `elifrs --help` for the full list.

Project
- `elifrs new <name> [--path <dir>]` — scaffold a new application.
- `elifrs serve [--port 3000] [--host 127.0.0.1] [--hot-reload] [--watch <paths>...] [--exclude <glob>...] [--env development]` — start dev server.
- `elifrs check` — lint/check project.
- `elifrs test [--focus <Resource>]` — run tests.
- `elifrs map [--json]` — output project map.

Generation
- `elifrs make resource <Name> --fields name:type[,..] [--relationships name:type] [--api] [--tests] [--policy] [--requests] [--resources]` — full resource scaffold.
- `elifrs generate middleware <Name> [--debug] [--conditional] [--tests]` — middleware scaffold.
- `elifrs route add <METHOD> <path> <controller>` — add route entry. `elifrs route list` to list routes.
- `elifrs model add <Name> <fields>` — add a model with fields.
- `elifrs resource new <name> --route <path> --fields <...>` — create a resource spec.

Database
- `elifrs migrate create <name>` — create a migration file.
- `elifrs migrate run` — run pending migrations.
- `elifrs migrate rollback` — rollback last migration.
- `elifrs migrate status` — show status.
- `elifrs db seed [--env <environment>] [--force] [--verbose]` — run seeders.
- `elifrs db factory status|test [--count 3]` — factory diagnostics.

API & Docs
- `elifrs version create <v>` — create API version (e.g., `v1`).
- `elifrs version deprecate <v> [--message <msg>] [--sunset-date <iso8601>]` — deprecate a version.
- `elifrs version list` — list versions.
- `elifrs version migrate --from <v1> --to <v2>` — generate migration guide.
- `elifrs version validate` — check config.
- `elifrs openapi generate [--output <path>] [--format json|yaml]` — generate OpenAPI.
- `elifrs openapi export --format postman|insomnia --output <path>` — export.
- `elifrs openapi serve [--port 8080]` — serve Swagger UI.

Queues
- `elifrs queue work [--queue default] [--max-jobs N] [--timeout 60] [--sleep 1000] [--workers 1] [--stop-when-empty] [--verbose]`
- `elifrs queue status [--queue <name>] [--detailed] [--refresh 0]`
- `elifrs queue schedule [--time <ts>] [--frequency <unit>] [--job <name>] [--dry-run] [--force] [--verbose] [--daemon] [--check-interval 60]`

Email
- `elifrs email send <to> --subject <s> [--template <name>] [--body <text>] [--html] [--context <json>]`
- `elifrs email template list|validate <template>|render <template> [--context <json>] [--format html|text|both]`
- `elifrs email provider test [--provider <name>]|configure <provider> [--interactive]|switch <provider>`
- `elifrs email queue status|process [--limit N] [--timeout 30]|clear [--failed] [--completed]`
- `elifrs email track analytics [--range today|week|month] [--filter <id>]|stats [--group-by day|hour|provider|template]`
- `elifrs email setup [--provider <name>] [--non-interactive]`
- `elifrs email test capture [--enable|--disable] [--dir <path>]|list [--detailed] [--to <email>] [--subject <s>] [--limit 10]|show <id> [--raw] [--part headers|text|html|attachments]|clear --all|--older-than <days>] | export [--format json|csv|mbox] [--output <path>] [--include-body]`
