CREATE TABLE agents (
    id SERIAL PRIMARY KEY,
    uuid UUID NOT NULL UNIQUE,
    name TEXT NOT NULL,
    capabilities TEXT,
    url TEXT NOT NUll,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX uuid_idx ON agents(uuid);
