# Migrations

Create migrations with the CLI and apply/rollback safely. Use idempotent, forward-only patterns when possible.

CLI
- `elifrs migrate create add_posts_table`
- `elifrs migrate run`
- `elifrs migrate rollback`
- `elifrs migrate status`

Patterns
- Use explicit up/down SQL where supported.
- Backfill with care; wrap in transactions when possible.
- Make rollbacks safe and test in CI.
