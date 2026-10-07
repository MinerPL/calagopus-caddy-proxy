DROP TABLE IF EXISTS dev_dawid_caddy_cleanup;
DROP TABLE IF EXISTS dev_dawid_caddy_routes;
ALTER TABLE servers DROP COLUMN IF EXISTS caddy_domains;
