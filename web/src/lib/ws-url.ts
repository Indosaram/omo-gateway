/**
 * Build an absolute ``ws(s)://`` URL for a dashboard WebSocket endpoint.
 *
 * This used to come from ``@hermes/shared``. That package is published from
 * the hermes monorepo and is not vendored into omon-gateway (it is absent
 * from both ``bun.lock`` and ``package-lock.json`` and does not exist on the
 * npm registry), so importing it left ``@/lib/api`` unloadable — the SPA
 * build never reaches that module, but ``bun test``/``vitest`` do. The
 * contract callers rely on is preserved here: the host comes from
 * ``window.location``, the dashboard base-path prefix is applied, extra
 * query params are merged, and the auth pair is appended last.
 */

/** ``[name, value]`` auth query pair, e.g. ``["ticket", id]`` or ``["token", tok]``. */
export type WebSocketAuthParam = readonly [string, string];

export interface WebSocketUrlOptions {
  /** Auth query pair for the active mode (fresh ticket when gated, token in loopback). */
  authParam: WebSocketAuthParam;
  /** Dashboard URL prefix: ``""`` at the host root, ``"/hermes"`` behind a proxy. */
  basePath?: string;
  /** Extra query params, merged before the auth param. */
  params?: Record<string, string>;
  /** Dashboard-relative path, e.g. ``"/api/ws"`` or ``"/api/plugins/kanban/events"``. */
  path: string;
}

export function buildHermesWebSocketUrl(opts: WebSocketUrlOptions): string {
  const { authParam, basePath = "", params, path } = opts;
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const base = basePath.replace(/\/+$/, "");
  const route = path.startsWith("/") ? path : `/${path}`;

  const query = new URLSearchParams();
  for (const [key, value] of Object.entries(params ?? {})) query.append(key, value);
  const [authName, authValue] = authParam;
  // An absent token (loopback without a session token) must not produce a
  // dangling ``?token=``; gated mode always carries a ticket here.
  if (authName && authValue) query.append(authName, authValue);
  const search = query.toString();

  const url = `${protocol}//${window.location.host}${base}${route}`;
  if (!search) return url;
  return `${url}${url.includes("?") ? "&" : "?"}${search}`;
}
