/** Durable browser-only activity and retry queue; no URLs or page titles. */
export type BrowserInterval = {
  source_id: string;
  seq: number;
  start_ms: number;
  end_ms: number;
  domain: string;
};
export type LocalInterval = BrowserInterval & { synced: boolean };
export type Active = { domain: string | null; since: number; seen: number };
export type DomainTime = { domain: string; duration_ms: number };

const NAME = 'watcher-browser-v1';
let database: Promise<IDBDatabase> | undefined;

function req<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

function connection(): Promise<IDBDatabase> {
  database ??= new Promise((resolve, reject) => {
    const request = indexedDB.open(NAME, 1);
    request.onupgradeneeded = () => {
      const db = request.result;
      db.createObjectStore('intervals', { keyPath: 'seq', autoIncrement: true });
      db.createObjectStore('meta');
    };
    request.onsuccess = () => {
      request.result.onversionchange = () => request.result.close();
      resolve(request.result);
    };
    request.onerror = () => reject(request.error);
  });
  return database;
}

function finished(tx: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error);
  });
}

/** A single transaction captures the updated cursor and all closed segments. */
export async function record(domain: string | null, now: number, recovered = false): Promise<void> {
  const db = await connection();
  const tx = db.transaction(['meta', 'intervals'], 'readwrite');
  const done = finished(tx);
  const meta = tx.objectStore('meta');
  const intervalStore = tx.objectStore('intervals');
  let source = await req(meta.get('source_id')) as string | undefined;
  if (!source) {
    source = crypto.randomUUID();
    meta.put(source, 'source_id');
  }
  const previous = await req(meta.get('active')) as Active | undefined;
  if (previous?.domain) {
    // After the MV3 service worker restarts, do not invent time while asleep.
    const end = recovered ? previous.seen : Math.min(now, previous.seen + 120_000);
    for (let from = previous.since; from < end; from += 120_000) {
      intervalStore.add({
        source_id: source,
        start_ms: from,
        end_ms: Math.min(end, from + 120_000),
        domain: previous.domain,
        synced: false,
      });
    }
  }
  meta.put({ domain, since: now, seen: now } satisfies Active, 'active');
  await done;
}

export async function localReport(fromMs: number, toMs: number): Promise<{
  domains: DomainTime[];
  pending: number;
}> {
  const db = await connection();
  const tx = db.transaction('intervals', 'readonly');
  const all = await req(tx.objectStore('intervals').getAll()) as LocalInterval[];
  const groups = new Map<string, number>();
  for (const row of all) {
    const ms = Math.max(0, Math.min(toMs, row.end_ms) - Math.max(fromMs, row.start_ms));
    if (ms) groups.set(row.domain, (groups.get(row.domain) ?? 0) + ms);
  }
  return {
    domains: [...groups].map(([domain, duration_ms]) => ({ domain, duration_ms }))
      .sort((a, b) => b.duration_ms - a.duration_ms),
    pending: all.filter((row) => !row.synced).length,
  };
}

export async function pendingBatch(limit = 64): Promise<LocalInterval[]> {
  const db = await connection();
  const tx = db.transaction('intervals', 'readonly');
  const all = await req(tx.objectStore('intervals').getAll()) as LocalInterval[];
  return all.filter((row) => !row.synced).slice(0, limit);
}

export async function acknowledge(sequences: number[]): Promise<void> {
  const db = await connection();
  const tx = db.transaction('intervals', 'readwrite');
  const done = finished(tx);
  const store = tx.objectStore('intervals');
  for (const seq of sequences) {
    const row = await req(store.get(seq)) as LocalInterval | undefined;
    if (row) store.put({ ...row, synced: true });
  }
  await done;
}
