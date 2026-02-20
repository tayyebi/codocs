-- Initial schema for Codocs federated sticky-note platform

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Users (local accounts)
CREATE TABLE users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username    TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    display_name TEXT,
    -- ActivityPub keys (RSA)
    public_key  TEXT NOT NULL,
    private_key TEXT NOT NULL,
    -- ActivityPub actor URL
    actor_url   TEXT NOT NULL UNIQUE,
    -- admin flag
    is_admin    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Remote actors cached from federation
CREATE TABLE remote_actors (
    actor_url   TEXT PRIMARY KEY,
    username    TEXT NOT NULL,
    host        TEXT NOT NULL,
    public_key  TEXT NOT NULL,
    inbox_url   TEXT NOT NULL,
    fetched_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Follow relationships (local follower -> remote or local followee)
CREATE TABLE follows (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    follower_actor_url TEXT NOT NULL,
    followee_actor_url TEXT NOT NULL,
    accepted    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (follower_actor_url, followee_actor_url)
);

-- Sticky notes (local + received via federation)
CREATE TABLE notes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- ActivityPub activity/object id
    ap_id       TEXT UNIQUE,
    -- author: local user id or NULL if remote
    author_id   UUID REFERENCES users(id) ON DELETE CASCADE,
    -- remote author actor URL if federated
    remote_author_url TEXT,
    -- the web page URL this note is anchored to
    page_url    TEXT NOT NULL,
    -- CSS selector for the anchored element
    selector    TEXT,
    -- note text
    body        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX notes_page_url_idx ON notes (page_url);

-- Delivered activities log (to avoid re-processing)
CREATE TABLE processed_activities (
    activity_id TEXT PRIMARY KEY,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
