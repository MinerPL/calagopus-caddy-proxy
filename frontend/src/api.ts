import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi, parsePaginationFromApi, serializeForApi } from '@/lib/serialization/api-transform.ts';

const routeSchema = z.object({
  uuid: z.string(),
  kind: z.enum(['subdomain', 'custom']),
  domain: z.string(),
  port: z.number(),
  recordType: z.enum(['A', 'CNAME']).nullable(),
  recordContent: z.string().nullable(),
  dns: z.enum(['cloudflare', 'manual']),
  created: z.coerce.date(),
});

const labelSchema = z.object({
  key: z.string(),
  value: z.string(),
});

const publicDomainSchema = z.object({
  uuid: z.string(),
  domain: z.string(),
  recordType: z.enum(['A', 'CNAME']),
  recordContent: z.string(),
  cloudflare: z.boolean(),
});

const serverCaddySchema = z.object({
  routes: z.array(routeSchema),
  labels: z.array(labelSchema),
  limit: z.number(),
  allowCustomDomains: z.boolean(),
  allowedDomains: z.array(publicDomainSchema),
});

const adminDomainSchema = z.object({
  uuid: z.string(),
  domain: z.string(),
  recordType: z.enum(['A', 'CNAME']),
  recordContent: z.string(),
  cloudflareEnabled: z.boolean(),
  cloudflareZoneId: z.string(),
  cloudflareTokenSet: z.boolean(),
  cloudflareProxied: z.boolean(),
});

const settingsSchema = z.object({
  allowCustomDomains: z.boolean(),
  defaultLimit: z.number(),
  domains: z.array(adminDomainSchema),
  pendingCleanup: z.number(),
});

const adminRouteSchema = z.object({
  uuid: z.string(),
  serverUuid: z.string(),
  serverName: z.string(),
  kind: z.enum(['subdomain', 'custom']),
  domain: z.string(),
  port: z.number(),
  dns: z.enum(['cloudflare', 'manual']),
  created: z.coerce.date(),
});

const createSchema = z.object({
  kind: z.enum(['subdomain', 'custom']),
  domainUuid: z.string().optional(),
  name: z.string().optional(),
  domain: z.string().optional(),
  port: z.number(),
});

const portSchema = z.object({
  port: z.number(),
});

const settingsWriteSchema = z.object({
  allowCustomDomains: z.boolean(),
  defaultLimit: z.number(),
  domains: z.array(
    z.object({
      uuid: z.string(),
      domain: z.string(),
      recordType: z.enum(['A', 'CNAME']),
      recordContent: z.string(),
      cloudflareEnabled: z.boolean(),
      cloudflareZoneId: z.string(),
      cloudflareApiToken: z.string().optional(),
      cloudflareProxied: z.boolean(),
    }),
  ),
});

const testSchema = z.object({
  uuid: z.string().optional(),
  domain: z.string(),
  cloudflareZoneId: z.string(),
  cloudflareApiToken: z.string().optional(),
});

const testResponseSchema = z.object({
  zoneName: z.string(),
});

export type CaddyRoute = z.infer<typeof routeSchema>;
export type ServerCaddy = z.infer<typeof serverCaddySchema>;
export type CaddySettings = z.infer<typeof settingsSchema>;
export type AdminCaddyRoute = z.infer<typeof adminRouteSchema>;
export type DomainDraft = z.infer<typeof settingsWriteSchema>['domains'][number] & {
  cloudflareTokenSet: boolean;
};

export const getServerCaddy = async (serverUuid: string): Promise<ServerCaddy> => {
  const { data } = await axiosInstance.get(`/api/client/servers/${serverUuid}/caddy`);
  return parseFromApi(serverCaddySchema, data);
};

export const createCaddyRoute = async (
  serverUuid: string,
  body: z.infer<typeof createSchema>,
): Promise<CaddyRoute> => {
  const { data } = await axiosInstance.post(
    `/api/client/servers/${serverUuid}/caddy`,
    serializeForApi(createSchema, body),
  );
  return parseFromApi(routeSchema, data.route);
};

export const updateCaddyPort = async (serverUuid: string, routeUuid: string, port: number): Promise<CaddyRoute> => {
  const { data } = await axiosInstance.patch(
    `/api/client/servers/${serverUuid}/caddy/${routeUuid}`,
    serializeForApi(portSchema, { port }),
  );
  return parseFromApi(routeSchema, data.route);
};

export const deleteCaddyRoute = async (serverUuid: string, routeUuid: string): Promise<void> => {
  await axiosInstance.delete(`/api/client/servers/${serverUuid}/caddy/${routeUuid}`);
};

export const getSettings = async (): Promise<CaddySettings> => {
  const { data } = await axiosInstance.get('/api/admin/extensions/dev.dawid.caddy/settings');
  return parseFromApi(settingsSchema, data);
};

export const updateSettings = async (body: z.infer<typeof settingsWriteSchema>): Promise<CaddySettings> => {
  const { data } = await axiosInstance.put(
    '/api/admin/extensions/dev.dawid.caddy/settings',
    serializeForApi(settingsWriteSchema, body),
  );
  return parseFromApi(settingsSchema, data);
};

export const testCloudflare = async (body: z.infer<typeof testSchema>): Promise<string> => {
  const { data } = await axiosInstance.post(
    '/api/admin/extensions/dev.dawid.caddy/domains/test',
    serializeForApi(testSchema, body),
  );
  return parseFromApi(testResponseSchema, data).zoneName;
};

export const getAdminRoutes = async (page: number, search: string) => {
  const { data } = await axiosInstance.get('/api/admin/extensions/dev.dawid.caddy/routes', {
    params: { page, per_page: 25, search: search || undefined },
  });
  return parsePaginationFromApi(adminRouteSchema, data);
};

export const deleteAdminRoute = async (routeUuid: string): Promise<void> => {
  await axiosInstance.delete(`/api/admin/extensions/dev.dawid.caddy/routes/${routeUuid}`);
};
