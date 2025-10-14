create extension if not exists "pgcrypto";
create extension if not exists "vector";

create table if not exists conversations (
    id uuid primary key default gen_random_uuid(),
    conv_id text not null unique,
    title text,
    created_at timestamptz not null,
    markers text[] not null default array[]::text[]
);

create table if not exists messages (
    id uuid primary key,
    conversation_id uuid not null references conversations(id) on delete cascade,
    idx int not null,
    role text not null,
    timestamp timestamptz not null,
    content text not null,
    project text,
    meeting text,
    markers text[] not null default array[]::text[]
);

create table if not exists embeddings (
    message_id uuid primary key references messages(id) on delete cascade,
    model text not null,
    dim int not null,
    vector vector(1536) not null
);

create index if not exists messages_project_idx on messages(project);
create index if not exists messages_timestamp_idx on messages(timestamp);
-- Note: IVFFlat index with optimal lists parameter is created dynamically based on data size
-- See floatctl-embed/src/lib.rs for dynamic index creation logic
-- For reference: optimal lists = max(10, row_count / 1000)
