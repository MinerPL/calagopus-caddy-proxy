import { faPen, faTrash } from '@fortawesome/free-solid-svg-icons';
import { useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import CopyOnClick from '@/elements/CopyOnClick.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import ConfirmationModal from '@/elements/modals/ConfirmationModal.tsx';
import ContextMenu, { ContextMenuToggle } from '@/elements/overlays/ContextMenu.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import Code from '@/elements/typography/Code.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { type CaddyRoute, deleteCaddyRoute, type ServerCaddy } from '../../api.ts';
import { useExtTranslations } from '../../translations.ts';
import RouteModal from './RouteModal.tsx';

export default function RouteRow({
  route,
  serverUuid,
  data,
  onChanged,
}: {
  route: CaddyRoute;
  serverUuid: string;
  data: ServerCaddy;
  onChanged: () => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const [openModal, setOpenModal] = useState<'edit' | 'delete' | null>(null);
  const canUpdate = useServerCan('caddy.update');
  const canDelete = useServerCan('caddy.delete');

  const doDelete = async () => {
    try {
      await deleteCaddyRoute(serverUuid, route.uuid);
      addToast(tExt('pages.server.toast.deleted', {}), 'success');
      setOpenModal(null);
      onChanged();
    } catch (err) {
      addToast(httpErrorToHuman(err), 'error');
    }
  };

  return (
    <>
      <RouteModal
        opened={openModal === 'edit'}
        onClose={() => setOpenModal(null)}
        serverUuid={serverUuid}
        data={data}
        route={route}
        onSaved={onChanged}
      />
      <ConfirmationModal
        opened={openModal === 'delete'}
        onClose={() => setOpenModal(null)}
        title={tExt('pages.server.deleteModal.title', {})}
        confirm={tExt('pages.server.button.delete', {})}
        onConfirmed={doDelete}
      >
        {tExt('pages.server.deleteModal.content', { domain: route.domain }).md()}
      </ConfirmationModal>

      <ContextMenu
        items={[
          {
            type: 'action',
            icon: faPen,
            label: tExt('pages.server.button.edit', {}),
            onClick: () => setOpenModal('edit'),
            color: 'gray',
            canAccess: canUpdate,
          },
          {
            type: 'action',
            icon: faTrash,
            label: tExt('pages.server.button.delete', {}),
            onClick: () => setOpenModal('delete'),
            color: 'red',
            canAccess: canDelete,
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
              <CopyOnClick content={route.domain}>
                <Code>{route.domain}</Code>
              </CopyOnClick>
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
              {route.recordType && route.recordContent ? (
                <Code>
                  {route.recordType} {route.recordContent}
                </Code>
              ) : (
                '—'
              )}
            </TableData>
            <TableData>
              <FormattedTimestamp timestamp={route.created} />
            </TableData>
            <ContextMenuToggle items={items} openMenu={openMenu} />
          </TableRow>
        )}
      </ContextMenu>
    </>
  );
}
