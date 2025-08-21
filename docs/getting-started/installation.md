# Installation

This guide covers installing the CLI, creating a new project, and running the dev server.

CLI installation
- Using Cargo: `cargo install elifrs` (installs the `elifrs` binary globally)
- From source (monorepo): `cargo build -p elifrs --release` (then add `target/release` to PATH)

Create a new project
- `elifrs new myapp`
- `cd myapp`

Run the development server
- `elifrs serve --hot-reload --port 3000`
- Open `http://127.0.0.1:3000`.

Project checklist
- Ensure `.env` contains `DATABASE_URL` for Postgres
- Run `elifrs migrate run` to apply migrations
- Run `elifrs test` to verify the setup
