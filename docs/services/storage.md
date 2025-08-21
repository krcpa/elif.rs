# Storage

Abstract file operations behind a storage service. Support both local and cloud (S3-like) backends, presigned URLs, and streaming.

Patterns
- Store uploads under private paths; expose via presigned URLs.
- Stream large files to avoid memory spikes.
- Sanitize file names and enforce content-type checks.
