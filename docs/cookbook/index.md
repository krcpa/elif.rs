# Cookbook

Practical recipes you can drop into your app.

- File uploads with presigned URLs
  - Generate a short-lived upload URL; verify content-type and size.
- JWT auth for mobile clients
  - Issue token pairs; refresh flow; revoke and rotate secrets.
- Background job for image processing
  - Enqueue uploads; resize and store variants; update DB; retries.
- Caching paginated queries
  - Cache list responses by (page, per_page, filters); invalidate on writes.
- Real-time notifications via WebSockets
  - Broadcast domain events to user-specific channels; handle reconnects.
- Multi-tenant routing and data separation
  - Route groups by subdomain; enforce tenant scoping on queries.
