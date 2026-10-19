// Exact existing QA-thread lifecycle; importing performs no I/O.
import assert from 'node:assert/strict';
const thread = '1558726010181132309';
const parent = '1529738911377657967';
export function existingQaSink({token, env, cwd, artifact, record, spawn}) {
  assert(token && !/[\r\n]/.test(token));
  const firstToken = token.split(',')[0].trim().replace(/^['"]|['"]$/g, '');
  let sequence = 0, original, changed = false;
  async function request(method, suffix = '', payload) {
    assert(suffix === '' || /^\/messages\/[0-9]+$/.test(suffix));
    assert(method === 'GET' || (method === 'PATCH' && suffix === ''));
    const url = `https://discord.com/api/v10/channels/${thread}${suffix}`;
    const argv = ['/usr/bin/curl', '-i', '--silent', '--show-error', '--max-time', '30', '--config', '-', '--request', method, url];
    if (payload) argv.push('--header', 'Content-Type: application/json', '--data', JSON.stringify(payload));
    record({surface:'HTTP', invocation:argv, stdin:'Authorization header supplied privately'});
    const escaped = firstToken.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
    const child = spawn(argv, {stdin:'pipe'});
    child.stdin.end(`header = "Authorization: Bot ${escaped}"\n`);
    const [raw, stderr, exit] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
    const id = `sink-http-${++sequence}`;
    artifact(`${id}.private.txt`, raw || stderr || `curl exit ${exit}`);
    assert.equal(exit, 0, `${id}: curl failed`);
    const matches = [...raw.matchAll(/HTTP\/\S+ ([0-9]{3})[^\r\n]*\r?\n/g)];
    const last = matches.at(-1); assert(last, `${id}: no HTTP status`);
    const start = last.index, separator = /\r?\n\r?\n/.exec(raw.slice(start));
    assert(separator, `${id}: no HTTP body separator`);
    record({id, method, thread, status:Number(last[1]), artifactRef:`${id}.private.txt`});
    assert.equal(Number(last[1]), 200, `${id}: Discord rejected request`);
    return JSON.parse(raw.slice(start + separator.index + separator[0].length));
  }
  function validate(value) {
    assert.equal(value.id, thread); assert.equal(value.parent_id, parent);
    assert([10,11,12].includes(value.type), 'Destination is not a thread');
    assert.equal(typeof value.thread_metadata?.archived, 'boolean');
    assert.equal(value.thread_metadata?.locked, false, 'Do not change a locked QA thread');
  }
  return {
    thread, parent,
    async prepare() {
      // Read and validate before unarchive, scratch binding, or milestone trigger.
      original = await request('GET'); validate(original);
      record({kind:'qa_destination', thread, parent, archived:original.thread_metadata.archived});
      if (original.thread_metadata.archived) {
        changed = true;
        const value = await request('PATCH', '', {archived:false}); validate(value);
        assert.equal(value.thread_metadata.archived, false);
      }
    },
    async readMilestoneReceipt(rows, expectedBody) {
      assert.equal(rows.length, 1, 'Require exactly one thread post after binding');
      const {id, text} = rows[0], post = JSON.parse(text);
      assert.equal(post.thread, thread); assert.equal(post.body, expectedBody);
      assert.equal(post.state, 'sent', 'Pending or uncertain is not delivery');
      assert(/^[0-9]+$/.test(post.message_id ?? ''));
      const delivered = await request('GET', `/messages/${post.message_id}`);
      assert.equal(delivered.id, post.message_id); assert.equal(delivered.channel_id, thread);
      assert.equal(delivered.content, post.body); assert.equal(String(delivered.nonce), post.nonce);
      record({kind:'qa_milestone_delivery', transcriptId:id, messageId:delivered.id, thread,
        nonce:post.nonce, bodyBytes:Buffer.byteLength(post.body), exactBodyMatch:true});
    },
    async restore() {
      if (changed) {
        const value = await request('PATCH', '', {archived:original.thread_metadata.archived});
        validate(value); assert.equal(value.thread_metadata.archived, original.thread_metadata.archived);
        changed = false;
        record({kind:'qa_archive_restored', thread, archived:value.thread_metadata.archived});
      }
    },
  };
}
