import assert from 'node:assert/strict';
import { readFileSync, realpathSync } from 'node:fs';
import { createHash } from 'node:crypto';
const preparedPath="/private/tmp/omon-lane-scratch-qa-TzIHbY/prepared.json";
const digest=bytes=>createHash('sha256').update(bytes).digest('hex');
const bytes=readFileSync(preparedPath);
assert.equal(digest(bytes),"4ae06ac9950ffd644d10ca0b573228e9a45447b4c619c5aedddef4a1193bad78");
const p=JSON.parse(bytes);
assert.equal(p.binarySha256,"60fce5ae8276944e9930bc2c41aa2ff039ad3daa29c05facd79e796cc621bc32");
const sha256=digest(readFileSync(p.binary));
assert.equal(sha256,p.binarySha256);
const cwd=realpathSync(process.cwd());
assert.equal(cwd,p.repo);
const inheritedDatabaseUrlEqual=process.env.DATABASE_URL===p.databaseUrl;
const child=Bun.spawn([p.binary,'--help'],{cwd:p.repo,env:p.env,stdout:'pipe',stderr:'pipe'});
const timer=setTimeout(()=>child.kill(),10000);
try {
  const [out,err,exitCode]=await Promise.all([new Response(child.stdout).text(),new Response(child.stderr).text(),child.exited]);
  console.log('OMON_SCOPE_FILE='+JSON.stringify({cwd,inheritedDatabaseUrlEqual,controlChild:{executable:p.binary,cwd:p.repo,databaseUrl:p.env.DATABASE_URL,databaseUrlEqual:p.env.DATABASE_URL===p.databaseUrl,envOverride:true,exitCode,sha256,argv:[p.binary,'--help'],stdoutBytes:out.length,stderrBytes:err.length}}));
} finally { clearTimeout(timer); }
