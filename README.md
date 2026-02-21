# Codocs — Federated Sticky Notes 📌

A federated sticky-note platform for the web, powered by **Rust**, **ActivityPub**, and **PostgreSQL**. Anyone can run their own Codocs node; notes are shared across instances via ActivityPub federation. The browser extension is the only front-end — including the admin area.

## Architecture

```
┌──────────────────────────────┐     ActivityPub      ┌──────────────────┐
│  Browser Extension           │ ←──────────────────→ │  Remote Codocs   │
│  (popup / content / admin)   │                       │  Node            │
└──────────┬───────────────────┘                       └──────────────────┘
           │ REST API (JWT)
           ▼
┌──────────────────────────────┐
│  Codocs Server (Rust binary) │
│  actix-web + sqlx            │
└──────────┬───────────────────┘
           │
     ┌─────▼─────┐
     │ PostgreSQL │
     └───────────┘
```

## Quick start (Docker)

```bash
cp .env.example .env   # edit POSTGRES_PASSWORD, JWT_SECRET, INSTANCE_DOMAIN
docker compose up -d
```

The server listens on port **8080** by default.

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | — | PostgreSQL connection string |
| `JWT_SECRET` | `change-me-in-production` | Secret for signing JWTs |
| `INSTANCE_DOMAIN` | `localhost` | Public domain of this node |
| `INSTANCE_URL` | `http://localhost:8080` | Full public URL of this node |
| `PORT` | `8080` | HTTP listen port |

## API Endpoints

### Auth
- `POST /api/auth/register` – register a new account
- `POST /api/auth/login` – get a JWT token

### Notes
- `GET  /api/notes?url=<page-url>` – list notes for a page (local + federated)
- `POST /api/notes` – create a sticky note `{ page_url, body, selector? }`
- `DELETE /api/notes/:id` – delete a note (author or admin)

### Admin (requires admin JWT)
- `GET    /api/admin/users` – list all users
- `DELETE /api/admin/users/:id` – delete a user
- `POST   /api/admin/users/:id/promote` – promote user to admin

### ActivityPub
- `GET  /.well-known/webfinger?resource=acct:user@domain` – WebFinger discovery
- `GET  /users/:username` – Actor document
- `POST /users/:username/inbox` – receive Follow / Create / Undo / Delete
- `GET  /users/:username/outbox` – user's recent notes

## Extension

Load the `extension/` directory as an **unpacked extension** in Chrome / Edge / Firefox:

1. Open `chrome://extensions` (or `about:debugging` in Firefox)
2. Enable **Developer mode**
3. **Load unpacked** → select `extension/`

The popup lets you register, sign in, pick elements, and post notes.  
Open **Options** (right-click extension icon → Options) for the admin area.

## Federation

To follow a remote user and receive their notes, a remote server sends a `Follow` activity to your inbox. Your server auto-accepts follows and begins delivering `Create` activities when notes are posted.

## Server development

```bash
cd server
cargo build            # or `cargo run` (requires DATABASE_URL env var at runtime)
```

