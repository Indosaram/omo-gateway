import assert from 'node:assert/strict';
import { executeTodo } from './qa-last-scenarios.mjs';
import { extra } from './qa-final-extra.mjs';
const [id, release] = process.argv.slice(2);
const n = Number(id);
assert([15,24,25,26,28,29,31,32].includes(n));
const results = [28,29,31,32].includes(n) ? [await extra(n,release)] : await executeTodo(n,release);
console.log('LAST_QA_RESULT '+JSON.stringify(results));
process.exit(results.some(x=>x.exitCode===1)?1:results.some(x=>x.exitCode===3)?3:0);
