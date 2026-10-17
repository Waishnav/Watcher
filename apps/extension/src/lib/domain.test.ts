import { expect, test } from 'bun:test';
import { safeDomain } from './domain';

test('stores only registrable domains, never URLs', () => {
  expect(safeDomain('https://github.com/org/private?secret=yes')).toBe('github.com');
  expect(safeDomain('https://news.bbc.co.uk/a')).toBe('bbc.co.uk');
  expect(safeDomain('chrome://extensions')).toBeNull();
  expect(safeDomain('about:privatebrowsing')).toBeNull();
  expect(safeDomain('http://localhost:8080')).toBeNull();
});
