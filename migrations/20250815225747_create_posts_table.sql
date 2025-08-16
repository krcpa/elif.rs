-- Migration: 20250815225747_create_posts_table.sql
-- Created at: 2025-08-15 22:57:47 UTC

-- Up
CREATE TABLE posts (
    id UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    content VARCHAR(255) NOT NULL,
    user_id INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
);



-- Down


DROP TABLE IF EXISTS posts;
