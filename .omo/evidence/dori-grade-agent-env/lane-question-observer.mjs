import assert from 'node:assert/strict';
import { watch, existsSync } from 'node:fs';

export function settledState(count, r, t, e) {
  assert(r?.status !== 'failed', 'Backend lane failed');
  assert((r?.turns_run ?? 0) <= count, 'Unexpected chained backend turn');
  assert(e.length <= count, 'Duplicate question event');
  if (r?.status !== 'blocked' || r.turns_run !== count || r.last_turn_outcome !== 'completed'
    || e.length !== count || t.filter(v => v.role === 'outcome' && v.text === 'completed').length !== count
    || t.filter(v => v.role === 'user').length !== count) return undefined;
  return { row: r, events: e, transcript: t };
}

export function observeState(p, bus, cancelObservers, label, predicate, ms) {
  let wal, timer, finished = false, resolve, reject;
  const promise = new Promise((a, b) => { resolve = a; reject = b; });
  promise.catch(() => {});
  const cancel = () => finish(new Error(`${label}: cancelled`));
  const directory = watch(p.root, (_event, name) => {
    if (name === null || String(name).startsWith('qa.db')) { attach(); check(); }
  });
  function attach() {
    if (!wal && existsSync(p.dbPath + '-wal')) {
      wal = watch(p.dbPath + '-wal', event => {
        if (event === 'rename') { wal.close(); wal = undefined; attach(); }
        check();
      });
      wal.on('error', error => finish(error));
    }
  }
  function finish(error, value) {
    if (finished) return;
    finished = true; directory.close(); wal?.close(); clearTimeout(timer);
    bus.off('line', check); cancelObservers.delete(cancel);
    error ? reject(error) : resolve(value);
  }
  function check() {
    try { const value = predicate(); if (value) finish(null, value); }
    catch (error) { finish(error); }
  }
  directory.on('error', error => finish(error));
  attach(); bus.on('line', check); cancelObservers.add(cancel);
  timer = setTimeout(() => { check(); if (!finished) finish(new Error(`${label}: deadline`)); }, ms);
  check();
  return promise;
}
