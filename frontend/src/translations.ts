import { defineEnglishItem, defineTranslations } from 'shared';

const translations = defineTranslations({
  items: {
    domain: defineEnglishItem('Domain', 'Domains'),
  },
  translations: {
    serverForm: {
      limit: 'Caddy domains',
      limitDescription: 'Maximum number of Caddy domains this server may publish.',
    },
    pages: {
      server: {
        title: 'Caddy',
        subtitle: '{count} of {limit} domains used',
        empty: 'No domains yet. Add a subdomain or point a domain you already own at this server.',
        notConfigured: 'An administrator still needs to add an allowed domain, or enable custom domains.',
        nodePointMissing: 'This server’s node has no public record yet. An administrator sets that under the Caddy extension.',
        restart: 'Restart the server after adding or removing a domain. Caddy Docker Proxy reads the labels the next time the container is created.',
        labels: 'Container labels',
        labelsHint: 'Click to copy. These are written onto the server container.',
        button: {
          create: 'Add domain',
          edit: 'Change port',
          delete: 'Delete',
          save: 'Save',
        },
        tooltip: {
          limitReached: 'This server has reached its limit of {limit} domains.',
        },
        table: {
          domain: 'Domain',
          port: 'Container port',
          dns: 'DNS',
          target: 'Public record',
          created: 'Created',
        },
        dns: {
          cloudflare: 'Cloudflare',
          manual: 'Manual',
        },
        toast: {
          created: 'Domain added. Restart the server so Caddy picks it up.',
          updated: 'Port updated. Restart the server so Caddy picks it up.',
          deleted: 'Domain removed.',
        },
        modal: {
          createTitle: 'Add domain',
          editTitle: 'Change port for {domain}',
          kind: 'Domain type',
          kindSubdomain: 'Subdomain',
          kindCustom: 'Custom domain',
          allowedDomain: 'Domain',
          name: 'Subdomain',
          nameDescription: 'Optional. Leave it empty to use the domain itself, or this becomes {preview}.',
          nameDescriptionEmpty: 'Optional. Leave it empty to use the domain itself, or enter a name such as play or mc.play.',
          preview: 'Address',
          dnsAuto: 'Cloudflare will create a {type} record for {name} pointing at {target}.',
          dnsManual: 'Create a {type} record for {name} pointing at {target}. Nothing is created in DNS for you.',
          customDomain: 'Domain',
          customDomainDescription: 'The full name, such as play.example.com. Point it at your Caddy host yourself.',
          port: 'Container port',
          portDescription:
            'The port the process listens on inside the container. The reverse proxy label uses the upstreams template with this port.',
        },
        deleteModal: {
          title: 'Remove domain',
          content: 'Remove **{domain}**? Its Cloudflare record is deleted too, and the Caddy label comes off this server.',
        },
      },
      admin: {
        tabs: {
          settings: 'Domains',
          routes: 'Server domains',
        },
        settings: {
          title: 'Allowed domains',
          description:
            'Users can publish a domain listed here, or a subdomain of it. The address comes from the node that server runs on. Turn on Cloudflare to create the record automatically.',
          allowCustom: 'Allow custom domains',
          allowCustomDescription: 'Lets a user attach a domain they manage themselves. No DNS record is created.',
          defaultLimit: 'Default domain limit',
          defaultLimitDescription: 'Applied to newly created servers. Change it per server under feature limits.',
          add: 'Add domain',
          domain: 'Domain',
          recordType: 'Record',
          recordContent: 'Points at',
          nodePointsTitle: 'Node points',
          nodePointsDescription:
            'Every node needs a record. It starts as a CNAME to that node’s domain, and a subdomain uses the record of the node its server runs on.',
          nodePointRequired: 'Required',
          nodePointsEmpty: 'This panel has no nodes yet.',
          nodeName: 'Node',
          cloudflare: 'Create records in Cloudflare',
          cloudflareDescription: 'Uses the zone id and token below when a user adds this domain or a subdomain of it.',
          zoneId: 'Zone ID',
          token: 'API token',
          tokenDescription: 'Zone.DNS:Edit on this zone. Stored encrypted. Leave blank to keep the saved token.',
          tokenStored: 'A token is saved.',
          proxied: 'Proxy through Cloudflare',
          proxiedDescription: 'Orange-cloud the record. Leave this off when Caddy itself should see the visitor.',
          snapshot: 'Domains already created keep the record they were given. Remove and add them again to point them somewhere else.',
          test: 'Test token',
          testOk: 'Cloudflare zone {zone} is reachable.',
          remove: 'Remove',
          save: 'Save',
          saved: 'Caddy settings saved.',
          cleanup: '{count} Cloudflare deletes are still retrying after a server was removed.',
          empty: 'No allowed domains yet. Users can still add a custom domain if that option is on.',
        },
        routes: {
          search: 'Search domains',
          empty: 'No server is using a Caddy domain yet.',
          server: 'Server',
          domain: 'Domain',
          port: 'Port',
          dns: 'DNS',
          created: 'Created',
          deleted: 'Domain removed.',
          deleteTitle: 'Remove domain',
          deleteContent: 'Remove **{domain}** from {server}? Its Cloudflare record is deleted too, and the Caddy label comes off that server.',
        },
      },
    },
  },
});

export const useExtTranslations = translations.useTranslations.bind(translations);
export const getExtTranslations = translations.getTranslations.bind(translations);

export default translations;
