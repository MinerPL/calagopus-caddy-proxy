import { faSearch, faTrash } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Center } from '@mantine/core';
import { useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Badge from '@/elements/data-display/Badge.tsx';
import Table, { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import TableLink from '@/elements/data-display/TableLink.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Group from '@/elements/layout/Group.tsx';
import ConfirmationModal from '@/elements/modals/ConfirmationModal.tsx';
import ContextMenu, { ContextMenuToggle } from '@/elements/overlays/ContextMenu.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Code from '@/elements/typography/Code.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useSearchablePaginatedTable } from '@/plugins/resource/useSearchablePaginatedTable.ts';
import { useAdminCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { type AdminCaddyRoute, deleteAdminRoute, getAdminRoutes } from '../../api.ts';
import { useExtTranslations } from '../../translations.ts';

export default function RoutesTab() {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const canManage = useAdminCan('caddy.manage');
  const [pending, setPending] = useState<AdminCaddyRoute | null>(null);

  const { data, loading, error, search, setSearch, setPage, refetch } = useSearchablePaginatedTable({
    queryKey: ['extensions', 'dev.minerpl.caddyproxy', 'admin', 'routes'],
    fetcher: (page, search) => getAdminRoutes(page, search),
    modifyParams: false,
  });

  const doDelete = async () => {
    if (!pending) return;
    try {
      await deleteAdminRoute(pending.uuid);
      addToast(tExt('pages.admin.routes.deleted', {}), 'success');
      setPending(null);
      refetch();
    } catch (err) {
      addToast(httpErrorToHuman(err), 'error');
    }
  };

  return (
    <>
      <ConfirmationModal
        opened={pending !== null}
        onClose={() => setPending(null)}
        title={tExt('pages.admin.routes.deleteTitle', {})}
        confirm={tExt('pages.server.button.delete', {})}
        onConfirmed={doDelete}
      >
        {pending
          ? tExt('pages.admin.routes.deleteContent', { domain: pending.domain, server: pending.serverName }).md()
          : null}
      </ConfirmationModal>

      <Group justify='flex-end' mb='md'>
        <TextInput
          placeholder={tExt('pages.admin.routes.search', {})}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          leftSection={<FontAwesomeIcon icon={faSearch} />}
          w={260}
        />
      </Group>

      <Table
        columns={[
          tExt('pages.admin.routes.server', {}),
          tExt('pages.admin.routes.domain', {}),
          tExt('pages.admin.routes.port', {}),
          tExt('pages.admin.routes.dns', {}),
          tExt('pages.admin.routes.created', {}),
          '',
        ]}
        loading={loading}
        error={error}
        pagination={data}
        onPageSelect={setPage}
        empty={
          <Center py='lg'>
            <Text c='dimmed'>{tExt('pages.admin.routes.empty', {})}</Text>
          </Center>
        }
      >
        {data?.data.map((route) => (
          <ContextMenu
            key={route.uuid}
            items={[
              {
                type: 'action',
                icon: faTrash,
                label: tExt('pages.server.button.delete', {}),
                onClick: () => setPending(route),
                color: 'red',
                canAccess: canManage,
              },
            ]}
          >
            {({ items, openMenu }) => (
              <TableRow
                onContextMenu={(e) => {
                  e.preventDefault();
                  openMenu(e.clientX, e.clientY);
                }}
              >
                <TableData>
                  <TableLink to={`/admin/servers/${route.serverUuid}`}>{route.serverName}</TableLink>
                </TableData>
                <TableData>
                  <Code>{route.domain}</Code>
                </TableData>
                <TableData>
                  <Code>{route.port}</Code>
                </TableData>
                <TableData>
                  <Badge color={route.dns === 'cloudflare' ? 'orange' : 'gray'}>
                    {tExt(`pages.server.dns.${route.dns}`, {})}
                  </Badge>
                </TableData>
                <TableData>
                  <FormattedTimestamp timestamp={route.created} />
                </TableData>
                <ContextMenuToggle items={items} openMenu={openMenu} />
              </TableRow>
            )}
          </ContextMenu>
        ))}
      </Table>
    </>
  );
}
