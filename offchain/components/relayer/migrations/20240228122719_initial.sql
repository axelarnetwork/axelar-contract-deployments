CREATE TABLE axelar_block (
    id serial PRIMARY KEY,
    latest_block bigint NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    CONSTRAINT singleton CHECK (id = 1)
);

CREATE TABLE solana_transaction (
    id serial PRIMARY KEY,
    latest_signature text NOT NULL,
    updated_at timestamp without time zone NOT NULL,
    CONSTRAINT singleton CHECK (id = 1)
);

CREATE TABLE axelar_messages (
    id serial PRIMARY KEY,
    solana_transaction text NOT NULL,
    source_address text NOT NULL,
    destination_address text NOT NULL,
    destination_chain text NOT NULL,
    payload bytea NOT NULL,
    payload_hash bytea NOT NULL CHECK (length(payload_hash) = 32),
    ccid text NULL,
    status text NOT NULL CHECK (status IN ('pending', 'submitted')) DEFAULT 'pending'
);

CREATE INDEX idx_payload_hash ON axelar_messages (payload_hash)
CREATE INDEX idx_solana_transaction ON axelar_messages (solana_transaction)
