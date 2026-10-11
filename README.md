# Caddy

> [!CAUTION]
> **This extension was written by AI. It is not recommended to run it in production.**
>
> It has not been reviewed or audited by a person. It talks to Cloudflare, writes DNS records, and changes server container labels. A mistake can take a domain offline, leave stale records behind, or expose a server you did not mean to publish. Use it only on a panel you can afford to break.

Calagopus Panel extension (`dev.minerpl.caddyproxy`) that publishes a server through [caddy-docker-proxy](https://github.com/lucaslorentz/caddy-docker-proxy).

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

On a Panel checkout, put it at `backend-extensions/dev_minerpl_caddyproxy` (a symlink is enough) and apply it:

```bash
panel-rs extensions apply --profile balanced
```

On the `:heavy` image, download `dev_minerpl_caddyproxy.c7s.zip` from the latest successful **Package** workflow run (Actions, open the run, then Artifacts). The download is that file itself. Upload it in Admin → Extensions, or copy it into `./build/extensions` and restart. That archive is the extension source in the layout `panel-rs extensions export` writes. The panel compiles it on install. The workflow uploads the archive as an artifact and does not publish a release.

To pack the same archive locally:

```bash
python3 scripts/package.py
```

The file is written to `dist/dev_minerpl_caddyproxy.c7s.zip` and is not committed.

## Make it work

The extension only writes container labels and, when Cloudflare is enabled, DNS records. Traffic reaches the server only after [caddy-docker-proxy](https://github.com/lucaslorentz/caddy-docker-proxy) is running on the node and can see those containers.

Do this on every node that will host a published server.

1. Install caddy-docker-proxy from that repository. The container below is the usual way to run it. Publish 80 and 443, and mount the Docker socket so it can read labels.

2. Set `CADDY_INGRESS_NETWORKS` to `calagopus_nw`. That is the Docker network Wings creates for server containers (`docker.network.name` in the Wings config; `calagopus_nw` is the default). If that setting was renamed, use the name that is actually there, in both places below.

3. When Caddy itself runs in Docker, attach that container to `calagopus_nw` as well. Wings already created the network, so mark it external. Do not let Compose create a second network with the same name.

```yaml
services:
  caddy:
    image: lucaslorentz/caddy-docker-proxy:ci-alpine
    ports:
      - 80:80
      - 443:443/tcp
      - 443:443/udp
    environment:
      - CADDY_INGRESS_NETWORKS=calagopus_nw
    networks:
      - calagopus_nw
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - caddy_data:/data
    restart: unless-stopped

networks:
  calagopus_nw:
    external: true

volumes:
  caddy_data: {}
```

Start it with `docker compose up -d` from the directory that holds this file. If Caddy is already a container, add `calagopus_nw` to its networks, set the same environment variable, and recreate the container. A one-off `docker network connect calagopus_nw <container>` joins the network until the container is recreated.

Then, in the panel:

1. Finish [Admin setup](#admin-setup): allowed domains, a public record on every node, and Cloudflare if you want subdomains created for you.
2. On the server, open **Caddy**, add the domain, and type the port the process listens on inside the container.
3. Restart that server. Wings recreates the container with the new labels, and caddy-docker-proxy picks them up.

Open 80 and 443 through the host firewall to the Caddy container. The node's public record (the CNAME or A address from node points) must be what visitors resolve to, so those ports land on this Caddy.

## Updating

Download the new artifact, which is `dev_minerpl_caddyproxy.c7s.zip` itself, and replace that file in `./build/extensions`, then rebuild. This is the same extension, so saved settings and existing domains stay. Do not add it as a second package, and do not leave two archives with that package name in the folder.

This package replaces `dev.dawid.caddy`. Delete that older archive from `./build/extensions` before adding this one. Two archives for different package names compile as two extensions, and a second archive of the same package fails the build as already installed. If the old extension already ran, its tables (`dev_dawid_caddy_routes`, `dev_dawid_caddy_cleanup`) are left behind; this package uses its own tables. The `servers.caddy_domains` column is shared and is added only when it is missing.

To rebuild the archive from a Panel checkout instead:

```bash
panel-rs extensions export dev.minerpl.caddyproxy
```

## Admin setup

Open the extension configuration page.

1. Add each domain users may create subdomains under.
2. Under **Node points**, every node needs a public record. It starts as a **CNAME** to that node's own domain (its public URL, otherwise the panel URL). Change it to an **A** record and an IPv4 address when the node should be addressed directly.
3. Optional: enable Cloudflare, paste the zone id and an API token with **Zone.DNS:Edit**, then test it. Proxied (orange-cloud) defaults to off so visitors reach Caddy directly.
4. Custom domains are on by default. They only add labels. DNS stays with the user, and a name under an allowed domain has to be created as a subdomain.
5. New servers start with a domain limit of 1. Change the default here, or set **Caddy domains** on a server (0 disables the tab's create button).

A saved public record is copied onto each subdomain when it is created. Editing the record later does not rewrite DNS that already exists. Remove the subdomain and add it again to point it somewhere else. An allowed domain that still has subdomains cannot be removed or renamed.

## What the server owner does

On the server, open **Caddy** and add a domain. Pick an allowed domain and a name such as `play` or `mc.play`, or type a full custom hostname. Enter the container port.

The page shows the labels that will be applied. Restart the server once they look right. Deleting a domain here also deletes its Cloudflare record. Deleting the server does the same: the route rows go with the server, and each Cloudflare record is deleted. If Cloudflare is down, the token is stored encrypted and a background task retries the delete about once a minute, backing off between attempts. That task runs on the primary Panel instance.

## Checks

`cargo test` in the extension crate needs the Panel workspace. The hostname and label rules are covered on their own:

```bash
cargo test --manifest-path checks/Cargo.toml
```

Inside a Panel checkout, the same tests run with the extension:

```bash
cargo test -p dev_minerpl_caddyproxy
```
