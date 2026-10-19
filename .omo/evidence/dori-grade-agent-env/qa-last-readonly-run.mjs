import assert from 'node:assert/strict';
import { extra } from './qa-last-readonly-extra.mjs';
const [id, release] = process.argv.slice(2);
assert([28,29].includes(Number(id)));
const receipt = await extra(Number(id), release);
console.log('LAST_READONLY_RESULT '+JSON.stringify(receipt));
process.exit(receipt.exitCode);
