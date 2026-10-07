import Tabs from '@/elements/layout/Tabs.tsx';
import { useExtTranslations } from '../../translations.ts';
import RoutesTab from './RoutesTab.tsx';
import SettingsTab from './SettingsTab.tsx';

export default function ConfigurationPage() {
  const { t: tExt } = useExtTranslations();

  return (
    <Tabs defaultValue='settings' keepMounted={false}>
      <Tabs.List>
        <Tabs.Tab value='settings'>{tExt('pages.admin.tabs.settings', {})}</Tabs.Tab>
        <Tabs.Tab value='routes'>{tExt('pages.admin.tabs.routes', {})}</Tabs.Tab>
      </Tabs.List>

      <Tabs.Panel value='settings' pt='md'>
        <SettingsTab />
      </Tabs.Panel>
      <Tabs.Panel value='routes' pt='md'>
        <RoutesTab />
      </Tabs.Panel>
    </Tabs>
  );
}
