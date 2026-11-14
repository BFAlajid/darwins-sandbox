import assert from 'node:assert/strict';
import { test } from 'node:test';
import { toPrevalencePoint } from './prevalence.ts';

test('converts engine fractions to percent', () => {
  const p = toPrevalencePoint({
    day: 14,
    prevalenceAscaris: 0.117,
    prevalenceTrichuris: 0.5,
    prevalenceHookworm: 0,
    prevalenceAny: 1,
  });
  assert.equal(p.day, 14);
  assert.ok(Math.abs(p.ascaris - 11.7) < 1e-9);
  assert.equal(p.trichuris, 50);
  assert.equal(p.hookworm, 0);
  assert.equal(p.any, 100);
});
