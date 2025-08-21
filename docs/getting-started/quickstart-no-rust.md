# Quickstart (No Rust)

Build a CRUD API end-to-end using the CLI — no prior Rust required. We’ll scaffold a resource, run DB migrations, start the dev server with hot reload, and generate OpenAPI docs.

You will:
- Create a new project
- Scaffold a `Post` resource (model + migration + controller + routes + tests)
- Run migrations and start the server
- Create API v1 and generate OpenAPI docs
- Run basic HTTP tests

1) Create a new project

- `elif new blog` — scaffolds a new application in `./blog`
- `cd blog`

2) Generate a resource

- `elif make resource Post --fields title:string,content:text,published:boolean --api --tests --policy --requests`  
  Generates model, controller, migration, tests, policies, request validators, and API endpoints.

3) Database setup and migrations

- Set `DATABASE_URL` in `.env` (e.g., Postgres DSN)
- `elif migrate create init` — creates base migration if needed
- `elif migrate run` — applies pending migrations
- `elif db seed --env development` — optional seed data

4) Start the development server

- `elif serve --hot-reload --port 3000`  
  Visit `http://127.0.0.1:3000` and hit your REST endpoints (e.g., `/posts`).

5) API versioning (optional but recommended)

- `elif version create v1 --description "Public API v1"`
- Route your generated API under `/api/v1` (see Routing).  
- `elif openapi generate --format yaml --output openapi/api_v1.yml`
- `elif openapi serve --port 8080` — open Swagger UI at `http://127.0.0.1:8080`

6) Run HTTP tests

- `elif test` — runs project tests  
- Focus a resource: `elif test --focus Post`

That’s it — you’ve shipped a versioned CRUD API with docs and tests.
