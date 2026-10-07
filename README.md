# Caddy

Calagopus Panel extension (`dev.dawid.caddy`) that publishes a server through [caddy-docker-proxy](https://github.com/lucaslorentz/caddy-docker-proxy).

A server owner opens the **Caddy** tab and adds either a subdomain of a domain an admin allowed, or a custom domain they already control. Allowed domains carry a public DNS record (an **A** address or a **CNAME** hostname). When Cloudflare is turned on for that domain, creating the subdomain writes the DNS record, and deleting the server deletes it.

Every domain on the server is written as container labels. One domain, or several that share a port, uses:

```text
caddy: play.example.com
caddy.reverse_proxy: {{upstreams 25565}}
```

The number is the port the process listens on inside the container, which the user types in. Domains that need different ports use `caddy_0`, `caddy_1`, and a matching `caddy_N.reverse_proxy`. Labels that this extension does not own are left alone. Restart the server after a change so Wings recreates the container.

Requires Calagopus Panel `>=1.2.3`.

## Install

This directory is the extension. It is not a standalone app.

On a Panel checkout, put it at `backend-extensions/dev_dawid_caddy` (a symlink is enough) and apply it:

```bash
panel-rs extensions apply --profile balanced
```

On the `:heavy` image, export a `.c7s.zip` from a checkout that already contains the extension and upload that archive:

```bash
panel-rs extensions export dev.dawid.caddy
```

The archive is written to `./exported-extensions/dev_dawid_caddy.c7s.zip`.

## Admin setup

Open the extension configuration page.

1. Add each domain users may create subdomains under.
2. Set the public record: **A** and the Caddy host's IPv4 address, or **CNAME** and a hostname.
3. Optional: enable Cloudflare, paste the zone id and an API token with **Zone.DNS:Edit**, then test it. Proxied (orange-cloud) defaults to off so visitors reach Caddy directly.
4. Custom domains are on by default. They only add labels. DNS stays with the user, and a name under an allowed domain has to be created as a subdomain.
5. New servers start with a domain limit of 1. Change the default here, or set **Caddy domains** on a server (0 disables the tab's create button).

A saved public record is copied onto each subdomain when it is created. Editing the record later does not rewrite DNS that already exists. Remove the subdomain and add it again to point it somewhere else. An allowed domain that still has subdomains cannot be removed or renamed.

## What the server owner does

On the server, open **Caddy** and add a domain. Pick an allowed domain and a name such as `play` or `mc.play`, or type a full custom hostname. Enter the container port.

The page shows the labels that will be applied. Restart the server once they look right. Deleting a domain here also deletes its Cloudflare record. Deleting the server does the same: the route rows go with the server, and each Cloudflare record is deleted. If Cloudflare is down, the token is stored encrypted and a background task retries the delete about once a minute, backing off between attempts. That task runs on the primary Panel instance.

## Caddy on the node

Run caddy-docker-proxy on the same Docker network Wings uses for server containers, so `{{upstreams}}` can see them. `CADDY_INGRESS_NETWORKS` can name that network when the proxy should not watch every one.

## Checks

`cargo test` in the extension crate needs the Panel workspace. The hostname and label rules are covered on their own:

```bash
cargo test --manifest-path checks/Cargo.toml
```

Inside a Panel checkout, the same tests run with the extension:

```bash
cargo test -p dev_dawid_caddy
```
