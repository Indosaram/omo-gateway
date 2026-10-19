<!-- task=st_01a14508 -->
st_01a14508 [completed] model mahoquot/claude-sonnet-5-5
Ask: research memory architectures for always-on personal agents. Wanted: a sourced report with implications for your gateway. For you: the report is below, covering ChatGPT, Mem0, Letta, Zep/Graphiti, Supermemory and Limitless/Bee. Now: done. Next: nothing queued.

Method note: no `web_search` tool was available, so I used curl and gh against official sites, arXiv, GitHub READMEs and the HN Algolia API. `help.openai.com` and `openai.com` were Cloudflare-blocked, so I read the ChatGPT Memory FAQ through a Wayback snapshot dated 2026-01-08. That means the FAQ may have changed since.

## Identity

| System | Company / owner | Official URL | Launch / date | Confidence |
|---|---|---|---|---|
| ChatGPT Memory | OpenAI | help.openai.com/en/articles/8590148-memory-faq | Saved memories and "Reference chat history" are documented. I could not confirm the launch date from a primary source. | High on features, low on dates |
| Mem0 | Mem0 (YC S24) | mem0.ai, github.com/mem0ai/mem0 | Repo created 2023-06-20. Paper arXiv:2504.19413 dated 2025-04-28. | High |
| Letta (MemGPT) | Letta | docs.letta.com, github.com/letta-ai/letta | Repo created 2023-10-11. Sleep-time compute post 2025-04-21, shipped in Letta 0.7.0. | High |
| Zep / Graphiti | Zep (getzep) | help.getzep.com/graphiti, github.com/getzep/graphiti | Graphiti repo created 2024-08-08. Paper arXiv:2501.13956 dated 2025-01-20. | High |
| Supermemory | Supermemory | supermemory.ai, github.com/supermemoryai/supermemory | Repo created 2024-02-27. | High on identity, medium on benchmark claims |
| Limitless (formerly Rewind) | Acquired by Meta, announced 2025-12-05 | limitless.ai | Acquisition per TechCrunch and Reuters. | High |
| Bee | Acquired by Amazon, announced 2025-07-22 | n/a | Acquisition per TechCrunch and The Verge. | Medium. I only saw headlines and HN metadata. |

## Findings

**ChatGPT**
- Memory has two separate toggles. "Reference saved memories" holds things you explicitly asked it to remember. "Reference chat history" draws on past chats and can change over time. [https://help.openai.com/en/articles/8590148-memory-faq (Wayback 2026-01-08)]
- Saved memories are always injected as context, like custom instructions, except the model updates them itself. ChatGPT can update, merge or remove them on request. [https://help.openai.com/en/articles/8590148-memory-faq]
- Turning off saved memory also turns off chat-history reference. Turning off chat history deletes what was derived from it, within 30 days. Deleted saved memories are logged for up to 30 days. [https://help.openai.com/en/articles/8590148-memory-faq]
- A saved-memory delete does not remove the source chat. Full removal means deleting both the memory and the chat. [https://help.openai.com/en/articles/8590148-memory-faq]
- Plus and Pro users get automatic memory management. It keeps memories "top of mind" using recency and frequency, and moves others to the background (shown greyed out). Users can pin or deprioritize memories. [https://help.openai.com/en/articles/8590148-memory-faq]
- Users can also search and sort memories, and view and restore prior versions of saved memories by date. [https://help.openai.com/en/articles/8590148-memory-faq]
- Temporary Chat neither reads nor writes memory. [https://help.openai.com/en/articles/8590148-memory-faq]
- The model is trained not to proactively remember sensitive information such as health details unless asked. [https://help.openai.com/en/articles/8590148-memory-faq]
- Memory rewrites search queries. For example, "restaurants I'd like" becomes "good vegan restaurants in San Francisco". [https://help.openai.com/en/articles/8590148-memory-faq]
- ChatGPT Pulse runs nightly asynchronous research on your behalf using saved memories and chat history, and delivers summaries the next day. It was Pro-only on mobile at the snapshot date. This is the closest shipped analogue to consolidation plus proactivity. [https://help.openai.com/en/articles/8590148-memory-faq]

**Mem0**
- The paper describes extract, consolidate and retrieve over conversations, plus a graph-memory variant. It reports 26% relative gain over OpenAI's memory on an LLM-as-judge metric, 91% lower p95 latency and over 90% token savings versus full context. These are the authors' own LOCOMO numbers. [https://arxiv.org/abs/2504.19413]
- The April 2026 algorithm in the README is "single-pass ADD-only extraction": one LLM call, no UPDATE or DELETE. Memories accumulate and nothing is overwritten. [https://github.com/mem0ai/mem0 (README, main)]
- Retrieval fuses semantic, BM25 keyword and entity matching, plus time-aware ranking. Mem0 claims 92.5 on LoCoMo and 94.4 on LongMemEval for its managed platform. It says open-source users should expect "directionally similar" but not identical results. [https://github.com/mem0ai/mem0]
- Memory is scoped to user, session and agent. A self-hosted server (docker compose) and a CLI exist. Apache 2.0. [https://github.com/mem0ai/mem0]

**Letta (MemGPT)**
- Sleep-time compute turns "raw context" into "learned context" during idle time. A sleep-time agent edits the primary agent's memory blocks asynchronously, while the primary agent has no core-memory edit tools. [https://www.letta.com/blog/sleep-time-compute/]
- The blog argues this fixes MemGPT's problems: slow, less reliable conversation when one agent does both jobs, and messy incremental memory. The sleep-time agent can run a stronger, slower model on a configurable frequency. [https://www.letta.com/blog/sleep-time-compute/]
- Current Letta docs use MemFS: memory is a git repo of Markdown files with YAML frontmatter. Files at the root load into the system prompt every turn. Directories with their own `MEMORY.md` stay out of context until needed. [https://docs.letta.com/concepts/memfs/index.md]
- Every memory edit is a git commit. Dreaming and "memory doctor" subagents use git worktrees so they can write concurrently with the main agent. [https://docs.letta.com/concepts/memfs/index.md]
- MemFS has no vector index by default. Agents use ordinary file search, and semantic search is an optional mod. [https://docs.letta.com/concepts/memfs/index.md]
- "Dreaming" runs background subagents that consolidate recent conversations. It triggers after N agent steps or on context compaction. An optional "Agent reviews before applying" mode adds a second pass but does not ask the user for approval. [https://docs.letta.com/configuration/memory/index.md]
- `/remember` teaches the agent a lesson explicitly, `/memory` shows what was saved, and `/doctor` audits duplication and prompt-token cost. [https://docs.letta.com/configuration/memory/index.md]

**Zep / Graphiti**
- Graphiti is a temporal context graph with episodes (raw data, as provenance), entities, and facts that carry validity windows. [https://github.com/getzep/graphiti (README)]
- When a fact changes, the old one is invalidated and not deleted, so you can query "true now" or "true at time T". [https://github.com/getzep/graphiti]
- Ingestion is incremental with no batch recompute. Retrieval combines embeddings, BM25 and graph traversal. [https://github.com/getzep/graphiti]
- Backends are Neo4j, FalkorDB (including an embedded "lite" mode, Python 3.12+) and Neptune. Kuzu is deprecated because its upstream is unmaintained. Extraction needs a model that supports structured output, and small local models often fail it. [https://github.com/getzep/graphiti]
- Graphiti ships an MCP server and a REST service. Telemetry is opt-out via `GRAPHITI_TELEMETRY_ENABLED=false`. [https://github.com/getzep/graphiti]
- The Zep paper reports 94.8% against MemGPT's 93.4% on DMR, and up to 18.5% accuracy gain with 90% lower latency on LongMemEval. These are vendor-reported. [https://arxiv.org/abs/2501.13956]

**Supermemory**
- It extracts facts, tracks contradictions and does "automatic forgetting": temporary facts such as "exam tomorrow" expire. It maintains a user profile split into `static` and `dynamic`, claimed at about 50ms. [https://github.com/supermemoryai/supermemory (README)]
- It runs RAG and memory together in "hybrid" search, with connectors for Gmail, Drive, Notion and GitHub. Memory is scoped by container tags, for example work versus personal. [https://github.com/supermemoryai/supermemory]
- A self-host "local" mode is a single binary, with data in `~/.supermemory` and an API on localhost:6767. It can run offline with Ollama. It uses an embedded graph engine and local `bge-base` embeddings. [https://github.com/supermemoryai/supermemory]
- Its "#1 on LongMemEval, LoCoMo, ConvoMem" claim is self-reported. Treat it as marketing until independently reproduced. [https://github.com/supermemoryai/supermemory]

**Limitless / Bee lifelogging**
- Meta acquired Limitless (formerly Rewind) on 2025-12-05. [https://techcrunch.com/2025/12/05/meta-acquires-ai-device-startup-limitless/]
- HN commenters say the service was shut down to new users and the Rewind Mac app was cut off from 2025-12-19. One user said the local SQLite DB is encrypted and cannot be exported. These are unverified commenter claims. [https://hn.algolia.com/api/v1/items/46166356]
- The same thread's claim that the product had no proprietary tech and just fed recordings to GPT and Claude APIs is one commenter's unverified opinion. It fits the lesson: lifelogging value lives in the memory layer, not the capture hardware. [https://hn.algolia.com/api/v1/items/46166356]
- Amazon acquired Bee, a wearable that "records everything you say", on 2025-07-22. [https://techcrunch.com/2025/07/22/amazon-acquires-bee-the-ai-wearable-that-records-everything-you-say/]
- Speculation: both exits show that cloud-only lifelog memory has vendor-lock and sunset risk. Local-first, exportable storage is the real differentiator.

## Implications for a self-hosted personal agent gateway

1. **Git-backed Markdown memory store, MemFS-style.** Keep owner memory as a plain-text repo with a root index file always injected, plus on-demand subdirectories. Every write is a commit. This gives you versioning, diffs and restore (matching ChatGPT's "view history") with no database. It also means the owner can edit memory in any editor and omo coding agents can read it as files.
2. **Dreaming lane on the cron scheduler.** Run a low-priority consolidation job that reads recent inbox and chat episodes, then proposes memory diffs. Use a git worktree so it never blocks the live lane. Trigger it on idle time or compaction, as Letta does. Use a stronger, slower model than the chat lane.
3. **Proposals, not direct writes, with owner approval over Discord or iMessage.** Letta's review mode explicitly does not ask the user. Gateway diffs could be sent as "Remember: X? 👍/👎", which gives user-editable memory over the channels you already use. Add `/remember`, `/forget` and `/what-do-you-know` commands, mirroring ChatGPT's "ask what it remembers".
4. **Bi-temporal facts for time-varying state.** Store facts with `valid_from` and `valid_to`, and invalidate instead of delete. This fits things like "lives in X", "current project" and "KakaoTalk contact changed numbers". It avoids Mem0's accumulate-everything drift. You can start with a simple SQLite table plus the Markdown repo, without adopting a graph DB.
5. **Provenance back to raw episodes.** Keep every derived memory linked to the source inbox item or message ID, as Graphiti does. Deleting a source should flag or remove dependent memories. ChatGPT's FAQ shows the gap: delete the memory and the chat or it lingers.
6. **Hybrid retrieval with local indexes.** Use BM25 (SQLite FTS5) plus local embeddings plus entity or tag match, fused. Supermemory and Mem0 both converge on this. Use per-context scopes (`work`, `personal`, per-repo) so omo coding agents get only repo-relevant memory and personal chat does not leak into code runs.
7. **TTL and decay forgetting.** Tag memories as ephemeral (expiry date), durable, or sensitive. Default sensitive categories such as health and finance to "only store when the owner explicitly asks". Rank by recency and frequency like ChatGPT's auto-management, and grey out and archive instead of hard delete.
8. **A "temporary mode" per lane or channel.** Equivalent to Temporary Chat: no memory read or write. Keep memory export and wipe first-class, and avoid the Limitless lock-in lesson.

## CLAIMS

- CLAIM: ChatGPT memory has separate saved-memory and chat-history layers, with version history/restore and auto-prioritization for Plus/Pro — RISK: normal — SOURCES: help.openai.com (via web.archive.org, 2026-01-08) — PRIMARY: https://help.openai.com/en/articles/8590148-memory-faq
- CLAIM: Letta sleep-time agents rewrite the primary agent's memory asynchronously, and the primary agent lacks core-memory edit tools — RISK: normal — SOURCES: letta.com — PRIMARY: https://www.letta.com/blog/sleep-time-compute/
- CLAIM: Letta's current memory is a git-backed Markdown filesystem (MemFS), with dreaming subagents using git worktrees — RISK: normal — SOURCES: docs.letta.com — PRIMARY: https://docs.letta.com/concepts/memfs/index.md
- CLAIM: Graphiti invalidates superseded facts with temporal validity windows instead of deleting them, and keeps provenance to episodes — RISK: normal — SOURCES: github.com/getzep/graphiti — PRIMARY: https://github.com/getzep/graphiti
- CLAIM: Mem0's April 2026 algorithm is ADD-only with multi-signal retrieval, and its 92.5 LoCoMo / 94.4 LongMemEval figures are for the managed platform — RISK: high — SOURCES: github.com/mem0ai/mem0 — PRIMARY: https://github.com/mem0ai/mem0
- CLAIM: Supermemory is #1 on LongMemEval, LoCoMo and ConvoMem — RISK: high (self-reported, no independent verification) — SOURCES: github.com/supermemoryai/supermemory — PRIMARY: https://github.com/supermemoryai/supermemory
- CLAIM: Meta acquired Limitless on 2025-12-05 and Amazon acquired Bee on 2025-07-22 — RISK: normal — SOURCES: techcrunch.com, hn.algolia.com — PRIMARY: https://techcrunch.com/2025/12/05/meta-acquires-ai-device-startup-limitless/
- CLAIM: The Rewind Mac app was cut off on 2025-12-19 and its local DB cannot be exported — RISK: high (single HN commenter) — SOURCES: hn.algolia.com — PRIMARY: none (https://hn.algolia.com/api/v1/items/46166356 is only the thread)

## EXPAND

- LEAD: Zep/Graphiti's bi-temporal edge schema and invalidation prompts (`graphiti_core`) — WHY: it is the best-documented contradiction-handling design, and it could be ported to SQLite — ANGLE: fetch the source via `gh api` at a pinned SHA.
- LEAD: The Letta `memfs-search` mod and QMD (`tobi/qmd`) — WHY: it is a concrete local hybrid-search stack over Markdown that could run on a Mac — ANGLE: read the READMEs and check for a Rust-callable interface.
- LEAD: ChatGPT Pulse help article (articles/12293630) — WHY: it is the shipped model of proactive nightly agent runs, relevant to your cron and inbox lanes — ANGLE: read it through the Wayback route, since the live site is Cloudflare-blocked.
- LEAD: Independent memory benchmarks (LongMemEval, LoCoMo, BEAM, supermemory's MemoryBench) — WHY: every vendor claims #1 on its own runs — ANGLE: look for third-party reproductions before choosing a retrieval design.
- LEAD: Bee's post-acquisition status and Limitless export options — WHY: it would confirm the lock-in risk, and I only saw headlines — ANGLE: read the full TechCrunch and Verge articles.