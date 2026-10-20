/** Build the ``session.create`` params for the ChatSidebar sidecar session.
 *
 * Extracted from ``components/ChatSidebar.tsx`` so the invariant —
 * ``close_on_disconnect`` is set, source is ``"tool"``, and the profile is
 * forwarded when present — can be tested without loading the component's UI
 * dependencies (``@nous-research/ui`` is not vendored into this repo).
 * ``ChatSidebar`` re-exports it, so component-side callers are unchanged.
 * See ``chat-sidebar-session-params.test.ts``.
 */
export function sidecarSessionCreateParams(profile?: string): Record<string, unknown> {
  return {
    close_on_disconnect: true,
    source: "tool",
    ...(profile ? { profile } : {}),
  };
}
