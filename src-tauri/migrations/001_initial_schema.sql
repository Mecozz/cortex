-- Migration 001: Initial schema -- all 9 BRAINDB tables

-- Projects: namespace scope for facts and tasks
CREATE TABLE IF NOT EXISTS proj (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Semantic Facts: extracted knowledge with temporal validity
CREATE TABLE IF NOT EXISTS facts (
    id TEXT PRIMARY KEY,
    proj_id TEXT REFERENCES proj(id),
    content TEXT NOT NULL,
    category TEXT,
    is_current INTEGER NOT NULL DEFAULT 1,
    valid_from INTEGER NOT NULL,
    valid_until INTEGER,
    last_confirmed_at INTEGER,
    source_conversation TEXT,
    importance_score REAL NOT NULL DEFAULT 0.5,
    confidence_score REAL NOT NULL DEFAULT 0.5,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_facts_current ON facts(is_current, confidence_score);
CREATE INDEX IF NOT EXISTS idx_facts_proj ON facts(proj_id);

-- Episodic Memory: every message stored as a diary entry
CREATE TABLE IF NOT EXISTS episodic (
    id TEXT PRIMARY KEY,
    proj_id TEXT REFERENCES proj(id),
    conversation_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    context_summary TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_episodic_convo ON episodic(conversation_id);
CREATE INDEX IF NOT EXISTS idx_episodic_time ON episodic(timestamp);

-- Vault: encrypted credential storage
CREATE TABLE IF NOT EXISTS vault (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    value_encrypted TEXT NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Conversations: archived summaries
CREATE TABLE IF NOT EXISTS convo (
    id TEXT PRIMARY KEY,
    proj_id TEXT REFERENCES proj(id),
    title TEXT,
    summary TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_convo_proj ON convo(proj_id);

-- Relationships: semantic graph extracted by Librarian
CREATE TABLE IF NOT EXISTS rel (
    id TEXT PRIMARY KEY,
    subject TEXT NOT NULL,
    predicate TEXT NOT NULL,
    object TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.5,
    source_conversation TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rel_subject ON rel(subject);

-- Scores: rolling capture buffer with decay
CREATE TABLE IF NOT EXISTS scores (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    score REAL NOT NULL,
    conversation_id TEXT NOT NULL,
    buffered_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_scores_expires ON scores(expires_at);

-- Tools: locally forged and registry-installed tool catalog
CREATE TABLE IF NOT EXISTS tools (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    code TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '0.1.0',
    is_active INTEGER NOT NULL DEFAULT 1,
    published INTEGER NOT NULL DEFAULT 0,
    registry_url TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Tasks: extracted goals and todos
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    proj_id TEXT REFERENCES proj(id),
    content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    source_conversation TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_proj ON tasks(proj_id);