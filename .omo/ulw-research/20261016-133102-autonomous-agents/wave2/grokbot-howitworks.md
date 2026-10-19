<!-- task=st_01a14519 -->
st_01a14519 [completed] model mahoquot/claude-sonnet-5-5
Ask: read the Lee Robinson transcript, the talk's YouTube description and the HN 49261514 staff comments, then report on queue, interrupt, compaction, action review, routines, sandbox and models. Wanted: a sourced report.
For you: the transcript and a secondary article were retrieved. The YouTube description was not, and the transcript's final section and Q&A were cut off. Details below.
Now: done. Next: none.

## Findings

Routes used (per source):
- **Transcript:** direct curl returned the HTML, but the response was truncated at about 50KB. r.jina.ai returned clean markdown, also truncated at about 50KB (it stops at the "Delete the product" slide, ~37:44). The Wayback Machine has not archived the page.
- **YouTube:** oEmbed returned only the title and channel. The r.jina.ai route for YouTube was blocked (403, anonymous access to the domain temporarily blocked).
- **HN:** the Algolia API worked.
- **Peter's article:** direct curl on the RSS feed returned the full text.

**PRIMARY (Lee Robinson, transcript of the Stanford CS146S talk):**
- **Queue and interrupt:** a message from the user to their own bot stops the running agent loop and takes priority. Group-chat, Slack and routine messages go to the back of the queue. Several messages arriving quickly are batched into one turn, with "whatever heuristic you want". Turns then run one at a time. [https://www.peterp.me/transcripts/how-grok-bot-works/ at 17:19] (r.jina.ai)
- **Wake path:** triggers (Slack, email, meetings, routines, background work) go through a router that dedupes them and puts them into the conversation. A heuristic then decides whether to bother the user at all. [same, 11:14] (r.jina.ai)
- **Durability:** he contrasts a job queue, where a crash replays steps 1 to 3, with a durable workflow that resumes at step 3. They use Temporal. Each helper is its own durable workflow. [same, 16:17 and 24:19] (r.jina.ai)
- **Send path:** the client acknowledges immediately. The turn is saved to the database, clients subscribe to change signals and then fetch the latest state, and retries are deduped. [same, 15:27] (r.jina.ai)
- **Thin client:** the only client-to-server tool is `send user`. Each client decides how to render it (sign-in forms, payments, a draft to edit and approve before irreversible actions, or an emoji reaction). [same, 18:21 and 19:16] (r.jina.ai)
- **Action review ("autoreview"):** a separate model checks every shell command before it runs. Lee claims in a blind test it beats a fatigued human. Untrusted inputs (email, webhooks) must not be treated as user instructions. Irreversible actions such as email or Slack go to the user first. Passwords are kept outside the model conversation. [same, 4:41 and 26:30] (r.jina.ai)
- **Escalation ladder:** try the cheapest option first: conversation context, then plugin/MCP/API, then web search, then browser, then full desktop. Ask the user only if all of those fail. For the browser, prefer the accessibility tree over HTML or a screenshot, and click pixels last. [same, 21:32 and 22:37] (r.jina.ai)
- **Sandbox:** a Firecracker microVM that can be snapshotted, slept and resumed. It has Chrome and virtual desktops, and bots share the infrastructure. The agent loop deliberately runs on the server, not in the VM, so the VM can be rebuilt (for example for a zero-day patch) without downtime. Everything is saved on the server. [same, 12:20 and 14:44] (r.jina.ai)
- **Compaction:**
  - Prompt prefix: keep the system prompt and tool definitions static, and never add or remove tools mid-turn. Hash prompt sections and keep a fixed order, so an A/B test cannot bust the cache for everyone. Warm the cache while the user is typing. Treat a cache miss as a bug.
  - Away-compaction: if the context is over about 100K tokens and the user leaves, compact while the provider cache is still warm (10 to 60 minutes) so it is cheaper.
  - Offloading: put long outputs in files and delegate screenshots and heavy work to helpers.
  - The 200K token figure in the talk is his "let's say" example.
  [same, 27:47 to 32:33] (r.jina.ai)
- **Memory layers:** core user facts included in every prompt, dated logs, ephemeral scratchpad notes that expire in about 30 to 60 minutes, and everything else as greppable files. [same, 32:50] (r.jina.ai)
- **Routines:** only briefly covered. They are cron-like scheduled prompts, shipped inside a plugin system alongside skills and MCP servers. Skills are markdown files. Tools are discovered lazily by name. [same, 9:32 and 20:01] (r.jina.ai)
- **Models:** the main agent is tuned for low latency, while helpers are tuned for throughput ("10 vs 12 minutes is negligible"). SFT teaches the model the harness's tools, and RL teaches instruction following and skill selection. Failures become evals, then harness fixes (inner loop), then retraining (outer loop). No model names are given in the text I retrieved. [same, 34:30 to 37:20] (r.jina.ai)

**SECONDARY (Peter Piekarczyk's article, which says it fills gaps from his own distributed-systems experience):**
- It states that your own messages interrupt and everything else is non-interrupting, and that helper completion is a non-interrupting queue message. [https://www.peterp.me/articles/what-actually-happens-when-you-message-grok-bot/] (direct curl via RSS)
- The `agentTurn` pseudocode, the "~2 minute idle sleep", Postgres/S3, the claim that all of a user's bots share one VM, and the statement that history is never deleted (only the working context is compacted) are his inference or stated without a timestamp. He labels them "my best guess" or "I assume".
- He lists four helper types (general, computer use, video, cloud coding). I could not confirm them in the retrieved transcript text. They may come from a slide image.
- The article ends with open questions to Lee, which show what remains undocumented: batch window, interrupted tool-call semantics, whether helpers can spawn helpers, and whether compaction blocks the main thread.

**HN 49261514 ("Grok Bot", landing page x.ai/bot):**
- There is only one identifiable staff voice, `leerob`, with two comments:
  - ChatGPT Work spins up and tears down a VM per cloud agent. Grok Bot is an "always-on Linux box, which stays logged in" and bots can message each other. [https://news.ycombinator.com/item?id=49266915] (Algolia)
  - It runs on the highest-tier Cursor/Grok accounts during the beta. [https://news.ycombinator.com/item?id=49266920] (Algolia)
- Neither comment says anything about queue, compaction or sandbox internals.
- Community context: pricing is reported as $120/$200 per user per month. [https://news.ycombinator.com/item?id=49261795]
- Linkregister: system-prompt rules "are one compaction away" from being lost, so gate remote calls through an MCP or API gateway. [https://news.ycombinator.com/item?id=49276687] (SECONDARY)
- Theshrike79: enforce read-only access at the service side. [https://news.ycombinator.com/item?id=49269085] (SECONDARY)

## Design patterns worth copying into a Rust personal-agent gateway

1. **Per-conversation inbox with an explicit priority class.**
   - Use `enum Source { Owner, Group, Routine, HelperDone, Webhook }` and `struct Inbound { id: Uuid, source, ts, payload }`.
   - Dedupe on the client-generated `id` with a UNIQUE constraint.
   - Only `Owner` sets a `CancellationToken` on the running turn. Everything else is appended.
2. **Turn state machine.** `Idle -> Collecting(window) -> Running -> {Interrupted, Compacting, Done} -> Idle`.
   - `Collecting` is a short debounce, and its length is my choice. Lee only says "whatever heuristic".
   - On `Interrupted`, keep completed steps and drop only the in-flight tool call, then merge the new message into the next batch.
   - Persist an append-only step log (`model_call` and `tool_call` results) in SQLite so a crash resumes at the last step. This gets Temporal's durability without Temporal.
   - Tools that cause side effects must carry an idempotency key.
3. **Helpers as child tasks.** Each helper gets its own step log, and a helper returns only a summary. Completion enqueues `HelperDone` as non-interrupting.
4. **Approval policy as a pure function.** `fn review(Action) -> Allow | Draft | Deny`.
   - `Allow` covers reads and reversible actions after an auto-review model check.
   - `Draft` covers irreversible outward actions (email, Slack, payments). The user edits and approves the draft before it is sent.
   - Content from email, web and webhooks is wrapped as `Tainted<String>` and can never be an instruction. This is my design addition, not stated by Lee.
   - Enforce limits in the tool layer or gateway, not the prompt. This matches the HN comments.
   - Keep secrets out of the model context and inject them at the tool layer.
5. **Compaction scheduler.**
   - On `Idle`, if `ctx_tokens > T` and `now < cache_expiry`, compact immediately, ahead of the cache TTL.
   - A hard-limit compaction blocks the turn.
   - Keep the full transcript immutable and compact only the model's working view (this is Peter's claim, so verify it).
   - Memory is files in layers: core facts, dated log, expiring scratchpad.
6. **Prompt layout.** Sections are hashed, ordered and fixed. The tool set is frozen per turn. Tools are discovered lazily by name. A cache-hit-rate metric alerts on regressions.
7. **Sandbox runtime.** The gateway owns the loop and state. The sandbox (a Firecracker or similar microVM) is disposable and snapshot-restorable, and shell tools run only inside it.
8. **Routines.** A `routines` table (`cron`, `prompt`, `bot`) fires an `Inbound{source: Routine}` event. A quiet-by-default notifier decides whether to message the user.
9. **One `send_to_user` tool** with a typed payload (`Text | Draft | Form | Reaction | Silent`), so every client renders it its own way.

## CLAIMS

- CLAIM: A separate autoreview model is safer than humans approving commands. RISK: High. This is Lee's assertion with no data in the transcript. HN comments show prompt-injection rates of roughly 2 to 5 percent and false-positive concerns. SOURCES: transcript at 4:41; HN items 49265215, 49275321, 49265487. PRIMARY: yes (claim); the counter-evidence is community opinion.
- CLAIM: Owner messages interrupt and batch while the rest queue FIFO. RISK: Low. SOURCES: transcript at 17:19. PRIMARY: yes.
- CLAIM: Temporal-style durable workflows make interrupted work resumable. RISK: Medium. The article's "interrupted does not mean lost" is inference. SOURCES: transcript at 16:17; the article. PRIMARY: partly.
- CLAIM: Background compaction while the cache is warm saves cost. RISK: Low to medium. The 100K and 10-to-60-minute figures are examples. SOURCES: transcript at 31:31. PRIMARY: yes.
- CLAIM: Firecracker VMs are snapshotted and all of a user's bots share one VM. RISK: Medium. Firecracker is stated by Lee. The shared VM is only partly stated by Lee ("virtual desktops", shared infrastructure) and the rest is Peter's wording. SOURCES: transcript at 12:20; the article. PRIMARY: partly.
- CLAIM: There are four helper types, a ~2-minute idle sleep and a ~200K context for Grok Bot. RISK: Unverified. SOURCES: the article only. PRIMARY: no.
- CLAIM: Grok Bot has an always-on logged-in Linux box, unlike per-agent VMs. RISK: Low. SOURCES: HN 49266915. PRIMARY: yes (staff comment).
- CLAIM: Routines are cron-like prompts in a plugin system. RISK: Low, but there is little detail. SOURCES: transcript at 9:32. PRIMARY: yes.

## EXPAND

- Missing: the YouTube description, the transcript after 37:44 (including the Q&A, which probably covers helpers and queue details), and any HN staff comments besides leerob's two.
- Retry the transcript tail with a different route (the r.jina.ai target-selector header, or fetching the page after rate limits clear). Retry the YouTube description via a non-blocked route, since r.jina.ai was blocked until about 14:37 UTC.
- Check Lee Robinson's X posts and the x.ai/bot docs for model names and routine specifics.