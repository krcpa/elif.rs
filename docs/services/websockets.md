# WebSockets

Build real-time features with channels, broadcasting, and per-connection state.

Example snippets (see `crates/elif-http/examples/websocket_channel_demo.rs`)
- Sending text messages to all connections
- Broadcasting to a specific channel/topic
- Enforcing auth on connect and rejecting unauthorized clients

Tips
- Keep messages small and typed (JSON schemas) to simplify clients.
- Add backpressure and rate limiting to avoid abuse.
