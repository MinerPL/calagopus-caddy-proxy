import { faPlus } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Center } from '@mantine/core';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { ServerCan } from '@/elements/Can.tsx';
import CopyOnClick from '@/elements/CopyOnClick.tsx';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Table from '@/elements/data-display/Table.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import ConditionalTooltip from '@/elements/overlays/ConditionalTooltip.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useServerStore } from '@/stores/server.ts';
import { getServerCaddy } from '../../api.ts';
import { useExtTranslations } from '../../translations.ts';
import RouteModal from './RouteModal.tsx';
import RouteRow from './RouteRow.tsx';

export const serverCaddyQueryKey = (serverUuid: string) =>
  ['extensions', 'dev.dawid.caddy', 'server', serverUuid] as const;

export default function CaddyPage() {
  const { t: tExt } = useExtTranslations();
  const { server } = useServerStore();
  const [createOpen, setCreateOpen] = useState(false);

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: serverCaddyQueryKey(server.uuid),
    queryFn: () => getServerCaddy(server.uuid),
  });

  const count = data?.routes.length ?? 0;
  const limit = data?.limit ?? 0;
  const configured = !!data && (data.allowedDomains.length > 0 || data.allowCustomDomains);
  const atLimit = !!data && count >= limit;
  const createDisabled = !configured || atLimit;
  const labelText = data?.labels.map((label) => `${label.key}: ${label.value}`).join('\n') ?? '';

  return (
    <ServerContentContainer
      title={tExt('pages.server.title', {})}
      subtitle={tExt('pages.server.subtitle', { count, limit })}
      contentRight={
        <ServerCan action='caddy.create'>
          <ConditionalTooltip
            enabled={!!data && createDisabled}
            label={
              configured
                ? tExt('pages.server.tooltip.limitReached', { limit })
                : tExt('pages.server.notConfigured', {})
            }
          >
            <Button
              disabled={createDisabled}
              onClick={() => setCreateOpen(true)}
              color='blue'
              leftSection={<FontAwesomeIcon icon={faPlus} />}
            >
              {tExt('pages.server.button.create', {})}
            </Button>
          </ConditionalTooltip>
        </ServerCan>
      }
    >
      <Stack gap='md'>
        {data && !configured && <Alert color='yellow'>{tExt('pages.server.notConfigured', {})}</Alert>}
        {count > 0 && <Alert color='blue'>{tExt('pages.server.restart', {})}</Alert>}

        {data && data.labels.length > 0 && (
          <Stack gap={4}>
            <Text size='sm' fw={600}>
              {tExt('pages.server.labels', {})}
            </Text>
            <Text size='xs' c='dimmed'>
              {tExt('pages.server.labelsHint', {})}
            </Text>
            <CopyOnClick content={labelText}>
              <Stack gap={2}>
                {data.labels.map((label) => (
                  <Code key={label.key}>
                    {label.key}: {label.value}
                  </Code>
                ))}
              </Stack>
            </CopyOnClick>
          </Stack>
        )}

        {data && (
          <RouteModal
            opened={createOpen}
            onClose={() => setCreateOpen(false)}
            serverUuid={server.uuid}
            data={data}
            onSaved={refetch}
          />
        )}

        <Table
          columns={[
            tExt('pages.server.table.domain', {}),
            tExt('pages.server.table.port', {}),
            tExt('pages.server.table.dns', {}),
            tExt('pages.server.table.target', {}),
            tExt('pages.server.table.created', {}),
            '',
          ]}
          loading={isLoading}
          error={error ? httpErrorToHuman(error) : null}
          pagination={{ total: count, perPage: Math.max(count, 1), page: 1, data: data?.routes ?? [] }}
          empty={
            <Center py='lg'>
              <Text c='dimmed'>{tExt('pages.server.empty', {})}</Text>
            </Center>
          }
        >
          {data?.routes.map((route) => (
            <RouteRow
              key={route.uuid}
              route={route}
              serverUuid={server.uuid}
              data={data}
              onChanged={refetch}
            />
          ))}
        </Table>
      </Stack>
    </ServerContentContainer>
  );
}
