CREATE SCHEMA IF NOT EXISTS posts;

CREATE TABLE IF NOT EXISTS posts.boards (
    id SERIAL PRIMARY KEY,
    name VARCHAR(16) UNIQUE NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE
);

CREATE TABLE IF NOT EXISTS posts.posts (
    id SERIAL PRIMARY KEY,
    board_id INTEGER NOT NULL REFERENCES posts.boards(id) ON DELETE CASCADE,
    title TEXT,
    text TEXT NOT NULL,
    hash_ip TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS posts_board_id_idx ON posts.posts (board_id);

CREATE TABLE IF NOT EXISTS posts.comments (
    id SERIAL PRIMARY KEY,
    post_id INTEGER NOT NULL REFERENCES posts.posts(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    hash_ip TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS comments_post_id_idx ON posts.comments (post_id);
