ALTER TABLE servers ADD COLUMN caddy_domains INTEGER NOT NULL DEFAULT 1;

CREATE TABLE dev_dawid_caddy_routes (
    uuid uuid NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    server_uuid uuid NOT NULL REFERENCES servers(uuid) ON DELETE CASCADE,
    kind varchar(16) NOT NULL,
    domain varchar(253) NOT NULL UNIQUE,
    parent_domain varchar(253),
    allowed_domain_uuid uuid,
    port integer NOT NULL,
    record_type varchar(8),
    record_content varchar(253),
    cloudflare_zone_id varchar(64),
    cloudflare_record_id varchar(64),
    created timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT dev_dawid_caddy_routes_kind_check CHECK (kind IN ('subdomain', 'custom')),
    CONSTRAINT dev_dawid_caddy_routes_port_check CHECK (port >= 1 AND port <= 65535)
);

CREATE INDEX dev_dawid_caddy_routes_server_uuid_idx
    ON dev_dawid_caddy_routes (server_uuid);
CREATE INDEX dev_dawid_caddy_routes_allowed_domain_idx
    ON dev_dawid_caddy_routes (allowed_domain_uuid);

CREATE TABLE dev_dawid_caddy_cleanup (
    uuid uuid NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    zone_id varchar(64) NOT NULL,
    record_id varchar(64) NOT NULL,
    token_encrypted text NOT NULL,
    attempts integer NOT NULL DEFAULT 0,
    last_error text,
    next_attempt timestamptz NOT NULL DEFAULT now(),
    created timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX dev_dawid_caddy_cleanup_next_attempt_idx
    ON dev_dawid_caddy_cleanup (next_attempt);
