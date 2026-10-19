import { test, expect } from 'bun:test';
import { EventEmitter } from 'node:events';
import { dirname } from 'node:path';
import { observeState, settledState } from './lane-question-observer.mjs';

test('pending boolean, blocked completion, answer terminal and initial state use actual observer', async () => {
  const bus = new EventEmitter(), cancellations = new Set();
  const p = { root: dirname(import.meta.path), dbPath: '/nonexistent-task12-regression/qa.db' };
  let state = false;
  const pending = observeState(p, bus, cancellations, 'first', () =>
    typeof state === 'object' && state !== null && state.row?.omo_thread_id ? state : undefined, 1000);
  expect(bus.listenerCount('line')).toBe(1);
  bus.emit('line');
  expect(cancellations.size).toBe(1);
  expect(settledState(1, undefined, [], [])).toBeUndefined();
  const row = { status: 'working', turns_run: 1, last_turn_outcome: null, omo_thread_id: 'same-thread' };
  expect(settledState(1, row, [{ role: 'user' }], [])).toBeUndefined();
  row.status = 'blocked'; row.last_turn_outcome = 'completed';
  const transcript = [{ role: 'user', text: 'brief' }, { role: 'outcome', text: 'completed' }];
  state = settledState(1, row, transcript, [{}]);
  bus.emit('line');
  expect((await pending).row.omo_thread_id).toBe('same-thread');
  expect(bus.listenerCount('line')).toBe(0);
  state = undefined;
  const answered = observeState(p, bus, cancellations, 'answer', () =>
    typeof state === 'object' && state !== null && state.row
      && state.transcript.some(v => v.role === 'user' && v.text === 'answer') ? state : undefined, 1000);
  row.turns_run = 2;
  transcript.push({ role: 'user', text: 'answer' }, { role: 'outcome', text: 'completed' });
  state = settledState(2, row, transcript, [{}, {}]);
  bus.emit('line');
  expect((await answered).row.omo_thread_id).toBe('same-thread');
  expect(await observeState(p, bus, cancellations, 'initial', () => state, 1000)).toBe(state);
  expect(cancellations.size).toBe(0);
  expect(bus.listenerCount('line')).toBe(0);
});
