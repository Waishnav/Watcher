import { getDomain } from 'tldts';

/** A domain only; paths, search params, fragments, and page titles are dropped. */
export function safeDomain(url: string | undefined): string | null {
  if (!url) return null;
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') return null;
    return getDomain(parsed.hostname, { allowPrivateDomains: true })?.toLowerCase() ?? null;
  } catch {
    return null;
  }
}
