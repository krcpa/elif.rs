# Introduction

elif.rs is a batteries-included web framework for Rust with a Laravel-style developer experience. It balances approachability and power:

- Expressive routing macros and groups with compile-time validation
- A rich `ElifRequest`/`ElifResponse` API and a Laravel-like `response()` builder
- An opinionated ORM and Query Builder with migrations, relations, pagination
- First-class API versioning and OpenAPI generation
- Built-in middleware v2 pipeline, logging, timeouts, CORS, and error handling
- A capable CLI (`elifrs`) that scaffolds projects, resources, migrations, tests

You can build apps without prior Rust knowledge by following conventions and using generators. When you’re ready, the framework’s types and modules are there so you can grow into Rust’s capabilities safely.

Core crates (selected):
- `elif-http`: server, routing, request/response, middleware, WebSocket
- `elif-http-derive`: macros like `#[routes]`, `#[group]`, `#[resource]`, `#[get]`
- `elif-orm`: models, query builder, relationships, transactions
- `elif-validation`: rules and request-validation primitives
- `elif-openapi`: OpenAPI spec generation and export
- `elifrs` (CLI): project scaffolding, codegen, migrations, versioning, docs
