import 'fake-indexeddb/auto';
import { expect, test } from 'bun:test';
import { record, pendingBatch, acknowledge, localReport } from './db';

test('offline observations remain local and replay only unacknowledged batches', async () => {
  const base = Date.now() - 20_000;
  await record('example.com', base);
  await record('github.com', base + 5000);
  await record(null, base + 8000);
  const batch = await pendingBatch();
  expect(batch).toHaveLength(2);
  expect(batch[0]!.domain).toBe('example.com');
  expect(batch[1]!.domain).toBe('github.com');
  expect(batch[0]!.source_id).toBe(batch[1]!.source_id);
  const before = await localReport(base, base + 10_000);
  expect(before.pending).toBe(2);
  expect(before.domains.reduce((sum, domain) => sum + domain.duration_ms, 0)).toBe(8000);
  await acknowledge(batch.map((row) => row.seq));
  expect(await pendingBatch()).toHaveLength(0);
  const after = await localReport(base, base + 10_000);
  expect(after.pending).toBe(0);
  expect(after.domains).toEqual(before.domains);
});
