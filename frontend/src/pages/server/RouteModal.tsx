import { type ModalProps } from '@mantine/core';
import { useEffect, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import { ServerCan } from '@/elements/Can.tsx';
import NumberInput from '@/elements/input/NumberInput.tsx';
import Select from '@/elements/input/Select.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import Stack from '@/elements/layout/Stack.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import FormModal from '@/elements/modals/FormModal.tsx';
import { ModalFooter } from '@/elements/modals/Modal.tsx';
import Text from '@/elements/typography/Text.tsx';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { type CaddyRoute, createCaddyRoute, type ServerCaddy, updateCaddyPort } from '../../api.ts';
import { useExtTranslations } from '../../translations.ts';

type Kind = 'subdomain' | 'custom';

const LABEL = /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/;

function validSubdomain(input: string): boolean {
  const value = input.trim().toLowerCase().replace(/\.$/, '');
  if (!value || value.length > 253) return false;
  return value.split('.').every((label) => LABEL.test(label));
}

function validHostname(input: string): boolean {
  const value = input.trim().toLowerCase().replace(/\.$/, '');
  if (!value || value.length > 253 || value.includes('..')) return false;
  const labels = value.split('.');
  return labels.length >= 2 && labels.every((label) => LABEL.test(label));
}

function portNumber(value: number | string): number {
  return typeof value === 'number' ? value : Number(value);
}

export default function RouteModal({
  serverUuid,
  data,
  route,
  onSaved,
  ...props
}: Omit<ModalProps, 'children'> & {
  serverUuid: string;
  data: ServerCaddy;
  route?: CaddyRoute;
  onSaved: () => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { t } = useTranslations();
  const { addToast } = useToast();

  const canSubdomain = data.allowedDomains.length > 0;
  const canCustom = data.allowCustomDomains;
  const both = canSubdomain && canCustom;

  const [loading, setLoading] = useState(false);
  const [kind, setKind] = useState<Kind>(canSubdomain ? 'subdomain' : 'custom');
  const [domainUuid, setDomainUuid] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [domain, setDomain] = useState('');
  const [port, setPort] = useState<number | string>('');

  useEffect(() => {
    if (!props.opened) return;
    setKind(canSubdomain ? 'subdomain' : 'custom');
    setDomainUuid(data.allowedDomains[0]?.uuid ?? null);
    setName('');
    setDomain('');
    setPort(route?.port ?? '');
  }, [props.opened]);

  const allowed = data.allowedDomains.find((entry) => entry.uuid === domainUuid) ?? null;
  const normalizedName = name.trim().toLowerCase().replace(/\.$/, '');
  const nameBlank = normalizedName.length === 0;
  const nameValid = nameBlank || validSubdomain(normalizedName);
  const preview =
    allowed && nameValid ? (nameBlank ? allowed.domain : `${normalizedName}.${allowed.domain}`) : '';
  const customValid = validHostname(domain);
  const parsedPort = portNumber(port);
  const portValid = Number.isInteger(parsedPort) && parsedPort >= 1 && parsedPort <= 65535;
  const domainValid = route
    ? true
    : kind === 'custom'
      ? customValid
      : !!allowed && nameValid && data.nodePointSet;

  const doSubmit = async () => {
    setLoading(true);
    try {
      if (route) {
        await updateCaddyPort(serverUuid, route.uuid, parsedPort);
        addToast(tExt('pages.server.toast.updated', {}), 'success');
      } else if (kind === 'subdomain') {
        await createCaddyRoute(serverUuid, {
          kind: 'subdomain',
          domainUuid: domainUuid ?? undefined,
          name: nameBlank ? '' : normalizedName,
          port: parsedPort,
        });
        addToast(tExt('pages.server.toast.created', {}), 'success');
      } else {
        await createCaddyRoute(serverUuid, {
          kind: 'custom',
          domain: domain.trim().toLowerCase().replace(/\.$/, ''),
          port: parsedPort,
        });
        addToast(tExt('pages.server.toast.created', {}), 'success');
      }
      onSaved();
      props.onClose();
    } catch (err) {
      addToast(httpErrorToHuman(err), 'error');
    }
    setLoading(false);
  };

  return (
    <FormModal
      size='lg'
      {...props}
      title={
        route
          ? tExt('pages.server.modal.editTitle', { domain: route.domain })
          : tExt('pages.server.modal.createTitle', {})
      }
      onSubmit={(e) => {
        e.preventDefault();
        doSubmit();
      }}
    >
      <Stack gap='md'>
        {!route && both && (
          <SegmentedControl
            fullWidth
            data={[
              { value: 'subdomain', label: tExt('pages.server.modal.kindSubdomain', {}) },
              { value: 'custom', label: tExt('pages.server.modal.kindCustom', {}) },
            ]}
            value={kind}
            onChange={(value) => setKind(value as Kind)}
          />
        )}

        {route && (
          <TextInput label={tExt('pages.server.modal.preview', {})} value={route.domain} readOnly disabled />
        )}

        {!route && kind === 'subdomain' && !data.nodePointSet && (
          <Alert color='yellow'>{tExt('pages.server.nodePointMissing', {})}</Alert>
        )}

        {!route && kind === 'subdomain' && data.nodePointSet && (
          <>
            <Select
              withAsterisk
              label={tExt('pages.server.modal.allowedDomain', {})}
              data={data.allowedDomains.map((entry) => ({ label: entry.domain, value: entry.uuid }))}
              value={domainUuid}
              onChange={setDomainUuid}
              allowDeselect={false}
            />
            <TextInput
              label={tExt('pages.server.modal.name', {})}
              description={
                preview
                  ? tExt('pages.server.modal.nameDescription', { preview })
                  : tExt('pages.server.modal.nameDescriptionEmpty', {})
              }
              placeholder='play'
              value={name}
              onChange={(e) => setName(e.target.value)}
              error={name.length > 0 && !nameValid}
            />
            {allowed && nameValid && allowed.recordType && allowed.recordContent && (
              <Text size='sm'>
                {allowed.cloudflare
                  ? tExt('pages.server.modal.dnsAuto', {
                      type: allowed.recordType,
                      name: preview,
                      target: allowed.recordContent,
                    })
                  : tExt('pages.server.modal.dnsManual', {
                      type: allowed.recordType,
                      name: preview,
                      target: allowed.recordContent,
                    })}
              </Text>
            )}
          </>
        )}

        {!route && kind === 'custom' && (
          <TextInput
            withAsterisk
            label={tExt('pages.server.modal.customDomain', {})}
            description={tExt('pages.server.modal.customDomainDescription', {})}
            placeholder='play.example.com'
            value={domain}
            onChange={(e) => setDomain(e.target.value)}
            error={domain.length > 0 && !customValid}
          />
        )}

        <NumberInput
          withAsterisk
          label={tExt('pages.server.modal.port', {})}
          description={tExt('pages.server.modal.portDescription', {})}
          placeholder='25565'
          min={1}
          max={65535}
          allowDecimal={false}
          value={port}
          onChange={setPort}
        />

        <ModalFooter>
          <ServerCan action={route ? 'caddy.update' : 'caddy.create'}>
            <Button type='submit' loading={loading} disabled={!domainValid || !portValid}>
              {route ? tExt('pages.server.button.save', {}) : tExt('pages.server.button.create', {})}
            </Button>
          </ServerCan>
          <Button variant='default' onClick={props.onClose}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </Stack>
    </FormModal>
  );
}
