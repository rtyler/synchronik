CREATE TABLE runs (
    uuid TEXT NOT NULL PRIMARY KEY,
    num INTEGER NOT NULL,
    status INTEGER NOT NULL,
    log_url TEXT NOT NULL,
    definition TEXT NOT NULL,
    scm_info TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT (DATETIME('now')),
    FOREIGN KEY(scm_info) REFERENCES scm_info(uuid),
    FOREIGN KEY(definition) REFERENCES run_definition(uuid)
);

CREATE TABLE scm_info (
    uuid TEXT NOT NULL PRIMARY KEY,
    git_url TEXT NOT NULL,
    ref TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT (DATETIME('now'))
);

CREATE TABLE run_definition (
    uuid TEXT NOT NULL PRIMARY KEY,
    definition TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT (DATETIME('now'))
);
