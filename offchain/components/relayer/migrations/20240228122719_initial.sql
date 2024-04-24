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
