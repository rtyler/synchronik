CREATE TABLE agents (
	id INTEGER PRIMARY KEY,
	uuid TEXT NOT NULL UNIQUE,
	name TEXT NOT NULL,
	capabilities TEXT,
	url TEXT NOT NUll,
	created_at TEXT NOT NULL
);
CREATE UNIQUE INDEX uuid_idx ON agents(uuid);
