import { faGlobe } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Extension, ExtensionContext } from 'shared';
import { z } from 'zod';
import { type FieldDef, insertFieldsAfter } from '@/elements/form-engine/index.ts';
import Text from '@/elements/typography/Text.tsx';
import ConfigurationPage from './pages/admin/ConfigurationPage.tsx';
import CaddyPage from './pages/server/CaddyPage.tsx';
import { getExtTranslations } from './translations.ts';

function CardSummary() {
  return <Text size='sm'>Publish server domains through Caddy Docker Proxy, with Cloudflare DNS for allowed zones.</Text>;
}

class DawidCaddyExtension extends Extension {
  public cardConfigurationPage: React.FC | null = ConfigurationPage;
  public cardComponent: React.FC | null = CardSummary;
  public cardIcon: React.ReactNode = <FontAwesomeIcon icon={faGlobe} />;

  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.enterRoutes((routes) =>
      routes.addServerRoute({
        name: () => getExtTranslations().t('pages.server.title', {}),
        icon: faGlobe,
        path: '/caddy',
        element: CaddyPage,
        permission: 'caddy.read',
      }),
    );

    ctx.extensionRegistry.enterPermissionIcons((icons) =>
      icons.addServerPermissionIcon('caddy', <FontAwesomeIcon icon={faGlobe} />).addAdminPermissionIcon('caddy', <FontAwesomeIcon icon={faGlobe} />),
    );

    ctx.extensionRegistry.enterForms((forms) => {
      for (const formId of ['admin.servers.create', 'admin.servers.update'] as const) {
        forms.extend(formId, {
          zodShape: {
            featureLimits: z.object({
              caddyDomains: z.number().int().min(0),
            }),
          },
          initialValues: {
            featureLimits: {
              caddyDomains: 1,
            },
          },
          transform: (fields) =>
            insertFieldsAfter(fields, 'featureLimits.schedules', {
              type: 'number',
              name: 'featureLimits.caddyDomains',
              label: () => getExtTranslations().t('serverForm.limit', {}),
              description: () => getExtTranslations().t('serverForm.limitDescription', {}),
              required: true,
              props: { placeholder: '1', min: 0 },
            } satisfies FieldDef),
        });
      }
    });
  }
}

export default new DawidCaddyExtension();
