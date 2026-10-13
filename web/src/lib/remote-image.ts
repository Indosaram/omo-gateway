/**
 * Chat content is model/user supplied, so an image URL in it must not be fetched
 * automatically: loading a remote image leaks the viewer's IP and lets the sender
 * track when the message is read. Only inline (data:/blob:) and same-origin images
 * load by themselves; everything else needs an explicit click.
 */
export function isAutoLoadableImageSrc(src: string | undefined | null, origin: string): boolean {
  if (!src) return false;
  const value = src.trim();
  if (!value) return false;
  if (/^(data:image\/|blob:)/i.test(value)) return true;
  // Browsers strip tabs/newlines and treat a backslash like `/`, so `/<backslash>host` or `/<TAB>/host` is
  // protocol-relative; none of these belong in a legitimate src.
  if (/[\\\t\r\n]/.test(value)) return false;
  // Protocol-relative URLs point at another host.
  if (value.startsWith("//")) return false;
  // No scheme: a same-origin relative path.
  if (!/^[a-z][a-z0-9+.-]*:/i.test(value)) return true;
  try {
    return new URL(value).origin === origin;
  } catch {
    return false;
  }
}
