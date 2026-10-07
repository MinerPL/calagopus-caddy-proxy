DROP TABLE IF EXISTS dev_minerpl_caddyproxy_cleanup;
DROP TABLE IF EXISTS dev_minerpl_caddyproxy_routes;
ALTER TABLE servers DROP COLUMN IF EXISTS caddy_domains;
