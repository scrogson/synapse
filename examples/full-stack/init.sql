-- Synapse Blog Example Schema

CREATE TABLE IF NOT EXISTS users (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    bio TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS posts (
    id BIGSERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    published BOOLEAN NOT NULL DEFAULT false,
    author_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_posts_author_id ON posts(author_id);
CREATE INDEX IF NOT EXISTS idx_posts_published ON posts(published);
CREATE INDEX IF NOT EXISTS idx_posts_created_at ON posts(created_at);
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at);

-- Seed data
INSERT INTO users (email, name, bio) VALUES
    ('alice@example.com', 'Alice', 'Software engineer and blogger'),
    ('bob@example.com', 'Bob', 'Tech enthusiast'),
    ('charlie@example.com', 'Charlie', NULL)
ON CONFLICT DO NOTHING;

INSERT INTO posts (title, content, published, author_id) VALUES
    ('Hello World', 'This is my first post!', true, 1),
    ('Getting Started with Rust', 'Rust is a systems programming language...', true, 1),
    ('Draft Post', 'This is still a draft.', false, 1),
    ('GraphQL Tips', 'Here are some tips for GraphQL...', true, 2)
ON CONFLICT DO NOTHING;
