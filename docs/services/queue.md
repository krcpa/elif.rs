# Queue

Process background jobs via the queue worker. Configure concurrency, timeouts, and schedules.

CLI
- Start worker: `elifrs queue work --queue default --workers 1 --timeout 60 --sleep 1000`
- Status: `elifrs queue status [--queue <name>] [--detailed]`
- Scheduler: `elifrs queue schedule [--job <name>] [--daemon] [--check-interval 60]`

Patterns
- Make jobs idempotent; record progress/checkpoints.
- Use dead-letter queues for repeated failures; add metrics for visibility.
