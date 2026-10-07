import { Center, SimpleGrid } from '@mantine/core';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { AdminCan } from '@/elements/Can.tsx';
import TitleCard from '@/elements/data-display/TitleCard.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import NumberInput from '@/elements/input/NumberInput.tsx';
import PasswordInput from '@/elements/input/PasswordInput.tsx';
import Select from '@/elements/input/Select.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useAdminCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { type DomainDraft, getSettings, testCloudflare, updateSettings } from '../../api.ts';
import { useExtTranslations } from '../../translations.ts';

export const adminSettingsQueryKey = ['extensions', 'dev.minerpl.caddyproxy', 'admin', 'settings'] as const;

function blankDomain(): DomainDraft {
  return {
    uuid: crypto.randomUUID(),
    domain: '',
    recordType: 'A',
    recordContent: '',
    cloudflareEnabled: false,
    cloudflareZoneId: '',
    cloudflareApiToken: '',
    cloudflareProxied: false,
    cloudflareTokenSet: false,
  };
}

export default function SettingsTab() {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const queryClient = useQueryClient();
  const canManage = useAdminCan('caddy.manage');

  const { data, isLoading, error } = useQuery({
    queryKey: adminSettingsQueryKey,
    queryFn: () => getSettings(),
  });

  const [allowCustomDomains, setAllowCustomDomains] = useState(true);
  const [defaultLimit, setDefaultLimit] = useState<number | string>(1);
  const [domains, setDomains] = useState<DomainDraft[]>([]);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState<string | null>(null);

  useEffect(() => {
    if (!data) return;
    setAllowCustomDomains(data.allowCustomDomains);
    setDefaultLimit(data.defaultLimit);
    setDomains(
      data.domains.map((domain) => ({
        uuid: domain.uuid,
        domain: domain.domain,
        recordType: domain.recordType,
        recordContent: domain.recordContent,
        cloudflareEnabled: domain.cloudflareEnabled,
        cloudflareZoneId: domain.cloudflareZoneId,
        cloudflareApiToken: '',
        cloudflareProxied: domain.cloudflareProxied,
        cloudflareTokenSet: domain.cloudflareTokenSet,
      })),
    );
  }, [data]);

  if (isLoading) {
    return (
      <Center py='lg'>
        <Spinner />
      </Center>
    );
  }

  if (error || !data) {
    return <Alert color='red'>{error ? httpErrorToHuman(error) : null}</Alert>;
  }

  const patch = (uuid: string, next: Partial<DomainDraft>) => {
    setDomains((current) => current.map((domain) => (domain.uuid === uuid ? { ...domain, ...next } : domain)));
  };

  const limit = typeof defaultLimit === 'number' ? defaultLimit : Number(defaultLimit);
  const incomplete = domains.some((domain) => {
    if (!domain.domain.trim() || !domain.recordContent.trim()) return true;
    if (!domain.cloudflareEnabled) return false;
    return !domain.cloudflareZoneId.trim() || (!domain.cloudflareApiToken.trim() && !domain.cloudflareTokenSet);
  });

  const doSave = async () => {
    setSaving(true);
    try {
      await updateSettings({
        allowCustomDomains,
        defaultLimit: Number.isInteger(limit) ? limit : 1,
        domains: domains.map((domain) => ({
          uuid: domain.uuid,
          domain: domain.domain,
          recordType: domain.recordType,
          recordContent: domain.recordContent,
          cloudflareEnabled: domain.cloudflareEnabled,
          cloudflareZoneId: domain.cloudflareZoneId,
          cloudflareApiToken: domain.cloudflareApiToken.trim() || undefined,
          cloudflareProxied: domain.cloudflareProxied,
        })),
      });
      addToast(tExt('pages.admin.settings.saved', {}), 'success');
      queryClient.invalidateQueries({ queryKey: adminSettingsQueryKey });
    } catch (err) {
      addToast(httpErrorToHuman(err), 'error');
    }
    setSaving(false);
  };

  const doTest = async (domain: DomainDraft) => {
    setTesting(domain.uuid);
    try {
      const zone = await testCloudflare({
        uuid: domain.uuid,
        domain: domain.domain,
        cloudflareZoneId: domain.cloudflareZoneId,
        cloudflareApiToken: domain.cloudflareApiToken.trim() || undefined,
      });
      addToast(tExt('pages.admin.settings.testOk', { zone }), 'success');
    } catch (err) {
      addToast(httpErrorToHuman(err), 'error');
    }
    setTesting(null);
  };

  return (
    <Stack gap='lg'>
      {data.pendingCleanup > 0 && (
        <Alert color='yellow'>{tExt('pages.admin.settings.cleanup', { count: data.pendingCleanup })}</Alert>
      )}

      <TitleCard title={tExt('pages.admin.settings.title', {})}>
        <Stack gap='md'>
          <Text size='sm' c='dimmed'>
            {tExt('pages.admin.settings.description', {})}
          </Text>
          <Switch
            label={tExt('pages.admin.settings.allowCustom', {})}
            description={tExt('pages.admin.settings.allowCustomDescription', {})}
            checked={allowCustomDomains}
            onChange={(e) => setAllowCustomDomains(e.target.checked)}
            disabled={!canManage}
          />
          <NumberInput
            withAsterisk
            label={tExt('pages.admin.settings.defaultLimit', {})}
            description={tExt('pages.admin.settings.defaultLimitDescription', {})}
            min={0}
            max={1000}
            allowDecimal={false}
            value={defaultLimit}
            onChange={setDefaultLimit}
            disabled={!canManage}
          />
        </Stack>
      </TitleCard>

      {domains.length === 0 && <Alert color='blue'>{tExt('pages.admin.settings.empty', {})}</Alert>}

      {domains.map((domain) => (
        <TitleCard key={domain.uuid} title={domain.domain || tExt('pages.admin.settings.domain', {})}>
          <Stack gap='md'>
            <SimpleGrid cols={{ base: 1, sm: 2 }}>
              <TextInput
                withAsterisk
                label={tExt('pages.admin.settings.domain', {})}
                placeholder='example.com'
                value={domain.domain}
                onChange={(e) => patch(domain.uuid, { domain: e.target.value })}
                disabled={!canManage}
              />
              <Select
                withAsterisk
                label={tExt('pages.admin.settings.recordType', {})}
                data={[
                  { value: 'A', label: 'A' },
                  { value: 'CNAME', label: 'CNAME' },
                ]}
                value={domain.recordType}
                onChange={(value) => value && patch(domain.uuid, { recordType: value as 'A' | 'CNAME' })}
                allowDeselect={false}
                disabled={!canManage}
              />
            </SimpleGrid>
            <TextInput
              withAsterisk
              label={tExt('pages.admin.settings.recordContent', {})}
              description={
                domain.recordType === 'A'
                  ? tExt('pages.admin.settings.recordContentA', {})
                  : tExt('pages.admin.settings.recordContentCname', {})
              }
              placeholder={domain.recordType === 'A' ? '203.0.113.10' : 'edge.example.net'}
              value={domain.recordContent}
              onChange={(e) => patch(domain.uuid, { recordContent: e.target.value })}
              disabled={!canManage}
            />
            <Text size='xs' c='dimmed'>
              {tExt('pages.admin.settings.snapshot', {})}
            </Text>
            <Switch
              label={tExt('pages.admin.settings.cloudflare', {})}
              description={tExt('pages.admin.settings.cloudflareDescription', {})}
              checked={domain.cloudflareEnabled}
              onChange={(e) => patch(domain.uuid, { cloudflareEnabled: e.target.checked })}
              disabled={!canManage}
            />
            {domain.cloudflareEnabled && (
              <>
                <TextInput
                  withAsterisk
                  label={tExt('pages.admin.settings.zoneId', {})}
                  value={domain.cloudflareZoneId}
                  onChange={(e) => patch(domain.uuid, { cloudflareZoneId: e.target.value })}
                  disabled={!canManage}
                />
                <PasswordInput
                  withAsterisk={!domain.cloudflareTokenSet}
                  label={tExt('pages.admin.settings.token', {})}
                  description={
                    domain.cloudflareTokenSet
                      ? `${tExt('pages.admin.settings.tokenStored', {})} ${tExt('pages.admin.settings.tokenDescription', {})}`
                      : tExt('pages.admin.settings.tokenDescription', {})
                  }
                  value={domain.cloudflareApiToken}
                  onChange={(e) => patch(domain.uuid, { cloudflareApiToken: e.target.value })}
                  disabled={!canManage}
                />
                <Switch
                  label={tExt('pages.admin.settings.proxied', {})}
                  description={tExt('pages.admin.settings.proxiedDescription', {})}
                  checked={domain.cloudflareProxied}
                  onChange={(e) => patch(domain.uuid, { cloudflareProxied: e.target.checked })}
                  disabled={!canManage}
                />
                <Group>
                  <Button
                    variant='default'
                    loading={testing === domain.uuid}
                    disabled={!canManage || !domain.domain.trim() || !domain.cloudflareZoneId.trim()}
                    onClick={() => doTest(domain)}
                  >
                    {tExt('pages.admin.settings.test', {})}
                  </Button>
                </Group>
              </>
            )}
            <Group justify='flex-end'>
              <Button
                color='red'
                variant='light'
                disabled={!canManage}
                onClick={() => setDomains((current) => current.filter((item) => item.uuid !== domain.uuid))}
              >
                {tExt('pages.admin.settings.remove', {})}
              </Button>
            </Group>
          </Stack>
        </TitleCard>
      ))}

      <Group justify='space-between'>
        <Button variant='default' disabled={!canManage} onClick={() => setDomains((current) => [...current, blankDomain()])}>
          {tExt('pages.admin.settings.add', {})}
        </Button>
        <AdminCan action='caddy.manage' cantSave>
          <Button onClick={doSave} loading={saving} disabled={incomplete || !Number.isInteger(limit)}>
            {tExt('pages.admin.settings.save', {})}
          </Button>
        </AdminCan>
      </Group>
    </Stack>
  );
}
