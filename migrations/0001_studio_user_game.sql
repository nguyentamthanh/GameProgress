BEGIN;

CREATE TABLE studios (
    id uuid PRIMARY KEY,
    name text NOT NULL,
    plan text NOT NULL DEFAULT 'free',
    version bigint NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT now(),
    deleted_at timestamptz NULL
);

CREATE TABLE users (
    id uuid PRIMARY KEY,
    studio_id uuid NOT NULL REFERENCES studios (id),
    email text NOT NULL,
    name text NOT NULL,
    studio_role text NOT NULL CHECK (studio_role IN ('admin', 'producer', 'member', 'viewer')),
    version bigint NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT now(),
    deleted_at timestamptz NULL,
    UNIQUE (studio_id, email)
);

CREATE INDEX users_studio_id_idx ON users (studio_id);

CREATE TABLE games (
    id uuid PRIMARY KEY,
    studio_id uuid NOT NULL REFERENCES studios (id),
    name text NOT NULL,
    genre text NULL,
    platforms text[] NOT NULL DEFAULT '{}',
    stage text NOT NULL CHECK (stage IN ('idea', 'prototype', 'vertical_slice', 'soft_launch', 'live', 'stopped')),
    owner_id uuid NOT NULL REFERENCES users (id),
    store_url text NULL,
    version bigint NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT now(),
    deleted_at timestamptz NULL
);

CREATE INDEX games_studio_id_idx ON games (studio_id);
CREATE INDEX games_studio_id_stage_idx ON games (studio_id, stage);

COMMIT;
