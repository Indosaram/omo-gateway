import { executeTodo } from './qa-final-scenarios.mjs';
import { extra } from './qa-final-extra.mjs';
const release = new URL('./final-release.json',import.meta.url).pathname;
const results = await Promise.all([15,16,18,20,24,25,26,35,28,29,31,32].map(async n=> {
  const rows = n===28||n===29||n===31||n===32 ? [await extra(n,release)] : await executeTodo(n,release);
  console.log('FINAL_TASK_DONE '+JSON.stringify({todo:n,rows:rows.map(x=>({root:x.root,status:x.status}))}));
  return rows;
}));
console.log('FINAL_ALL_DONE '+JSON.stringify(results.flat().map(x=>({todo:x.todo,root:x.root,status:x.status}))));
process.exitCode=results.flat().some(x=>x.exitCode===1)?1:0;
