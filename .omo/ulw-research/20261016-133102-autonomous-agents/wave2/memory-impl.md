<!-- task=st_01a14523 -->
st_01a14523 [completed] model mahoquot/claude-sonnet-5-5
Ask: research Graphiti bi-temporal edges and invalidation prompts, Letta sleep-time, qmd and Mem0 OSS for a SQLite + Rust gateway - wanted: a sourced report. For you: all four systems are covered from primary sources, and the report is below. Now: done. Next: none.

**Routes:** `curl` straight to `raw.githubusercontent.com` and the project docs sites worked for everything. The `.md` suffix on the Letta and Mem0 docs gave clean markdown. `gh search code` worked for the qmd schema. `gh api` with an Accept header and `curl --range` were rejected by the broker. I never needed Wayback, r.jina.ai, HN or Reddit. Pinned SHAs: graphiti `d50741d1bf463cbe3859eb73914cbff1b66839e6`, qmd `86d2365d17fc682e491c2c53d1e4a884aee8682a`. Permalinks are file-level because I did not capture line numbers.

## Findings

**Graphiti (PRIMARY, curl raw @ pinned SHA)**
- `EntityEdge` holds the whole bi-temporal model: `created_at` and `expired_at` (transaction time, when the system learned or retired it) plus `valid_at` and `invalid_at` (event time). It also carries `fact`, `fact_embedding`, `episodes[]` (provenance list), `reference_time` and a typed `attributes` dict. [https://github.com/getzep/graphiti/blob/d50741d1bf463cbe3859eb73914cbff1b66839e6/graphiti_core/edges.py] curl-raw
- The schema comments say future-tense plans use the episode reference time as `valid_at`, and soft deadlines are not `invalid_at`. A regex layer enforces this after the LLM answers: it clamps `valid_at` and drops `invalid_at` on "by Friday" phrasing unless the text says "until/ends/no longer". [https://github.com/getzep/graphiti/blob/d50741d1bf463cbe3859eb73914cbff1b66839e6/graphiti_core/utils/maintenance/temporal_edge_utils.py] curl-raw
- The dedupe/contradiction prompt returns only `duplicate_facts: [idx]` and `contradicted_facts: [idx]`, with continuous indexing over two lists: existing same-endpoint facts, then broader invalidation candidates. [https://github.com/getzep/graphiti/blob/d50741d1bf463cbe3859eb73914cbff1b66839e6/graphiti_core/prompts/dedupe_edges.py] curl-raw
- The prompt is deliberately conservative. The system message says "NEVER mark facts as contradicted unless the new fact explicitly makes the old fact false." It forbids inferring contradiction from recency alone, and forbids treating project-activity facts as contradicting role or ownership facts. Numeric or date differences block duplicate marking. Duplicates can only come from the first list. [same file] curl-raw
- Contradiction is applied deterministically in code, not by the LLM. `resolve_edge_contradictions` skips candidates whose validity intervals don't overlap. If the candidate started earlier than the new edge, it sets `candidate.invalid_at = new.valid_at` and `expired_at = now`. `_apply_temporal_invalidation` also expires the new edge itself when a later-starting candidate already exists, or when it arrives with `invalid_at`. [https://github.com/getzep/graphiti/blob/d50741d1bf463cbe3859eb73914cbff1b66839e6/graphiti_core/utils/maintenance/edge_operations.py] curl-raw
- The resolution pipeline is cheap-first:
  1. In-batch dedupe on (src, tgt, normalized fact).
  2. Verbatim-match fast path against existing edges, which only appends the episode uuid.
  3. Hybrid search (BM25 + embedding with RRF) restricted to the same endpoint pair for duplicates, plus an unrestricted search for invalidation candidates.
  4. A small-model LLM call.
  5. Invalid indices from the LLM are logged and ignored rather than trusted. [edge_operations.py] curl-raw

**Letta (PRIMARY docs; source not retrieved)**
- `/guides/agents/architectures/sleeptime` now returns a 404 page. The current docs call this "Memory & dreaming". [https://docs.letta.com/configuration/memory/index.md] curl
- Dreaming runs background subagents over recent conversations and edits memory without interrupting the main agent. Triggers are after N completed agent steps or on context compaction. [same] curl
- "Agent reviews before applying" has the agent revise proposed updates in a second background conversation. The docs say explicitly that it does not ask the user for approval. [same] curl
- Memory is a git repo (MemFS): Markdown with YAML frontmatter, root files always in the system prompt, subdirectories behind a `MEMORY.md` index. Every edit is a commit. Dreaming and "memory doctor" subagents use git worktrees to avoid blocking the main agent. Reorganization backs up the repo first. [https://docs.letta.com/concepts/memfs/index.md] curl
- MemFS has no vector index by default. Semantic search is an optional mod that uses qmd. [same] curl
- I did not retrieve the Letta server source. The guessed path `letta/groups/sleeptime_multi_agent_v4.py` returned 404 and `gh search code` returned nothing. Which step counts trigger dreaming is therefore docs-only.

**qmd (PRIMARY, README + `src/store.ts` @ pinned SHA)**
- qmd is a retrieval engine, not a memory consolidator. It has no contradiction handling and no consolidation job. Its pipeline is FTS5 BM25 plus sqlite-vec vectors, RRF with k=60 (original query weighted ×2, top-rank bonus), LLM rerank, and position-aware blending of retrieval and reranker scores. [https://github.com/tobi/qmd/blob/86d2365d17fc682e491c2c53d1e4a884aee8682a/README.md] curl-raw
- The storage design is directly reusable. `content(hash PK, doc)` is content-addressed. `documents(collection, path, hash, active, ...)` is a soft-delete layer. There are also `content_vectors(hash, seq, pos, model, embed_fingerprint)`, `llm_cache(hash, result)`, `file_sync_state(mtime, size, hash)`, an FTS5 table with the porter tokenizer, and FTS sync triggers. A `user_version` pragma gates migrations inside `BEGIN IMMEDIATE`. [https://github.com/tobi/qmd/blob/86d2365d17fc682e491c2c53d1e4a884aee8682a/src/store.ts] curl-raw
- An embedding fingerprint is stored with each vector (model, prompt format, chunk size), so a model change flags what needs re-embedding. [store.ts] curl-raw
- Trust gating: a checked-in project config's shell hooks and external paths are skipped unless approved, the approval is recorded for the exact gated set, and re-approval is required on any change. Non-interactive callers (agents, MCP) get skip, not auto-approve. [README] curl-raw
- The HTTP daemon rejects non-loopback `Origin` and `Host` headers (DNS-rebinding defense) and has no auth. [README] curl-raw
- Default chunking is 900 tokens with 15% overlap, using scored break points that never split code fences. [README + store.ts] curl-raw

**Mem0 OSS (PRIMARY docs; version: Python 2.x / TS 3.x per llms.txt)**
- Automatic extraction is ADD-only. "I moved from Austin to Seattle" stores a new fact without rewriting the old one. Correction needs an explicit `update` or `delete`. [https://docs.mem0.ai/core-concepts/how-it-works.md] curl
- The OSS library defaults to a local Qdrant vector store and a SQLite history store at `~/.mem0/history.db`. The self-hosted server stack uses Postgres + pgvector. [https://docs.mem0.ai/open-source/overview.md] curl
- OSS retrieval boosts on entity overlap only and has no graph memory. [how-it-works.md] curl
- The "Dream" feature (supersede, merge, synthesis) is listed as Platform only. Supersede and merge run inline at add time. Retired facts are linked to their replacement, never deleted, and read flags such as `latest_only` and `include_merged` choose which states a read returns. [https://docs.mem0.ai/platform/features/dream.md] curl
- Synthesis is a scheduled job. It needs at least 20 memories per user and a 7-day cadence on Pro. It only considers memories created after it was enabled and only user-scoped ones. Pattern memories link back to their sources, and re-runs are idempotent. [dream.md] curl
- `expiration_date` hides a memory from search without deleting it. [https://docs.mem0.ai/core-concepts/memory-operations/add.md] curl

## Design patterns worth copying

**Recommendation:** Use Graphiti's edge schema and conservative invalidation logic, qmd's SQLite layout for retrieval, and Mem0/Letta's non-destructive, scheduled consolidation. Skip a graph database. A single `facts` table with timestamps covers the schema, and entities can be plain columns.

**Schema (SQLite, rusqlite + FTS5 + sqlite-vec):**
- `episodes(id, hash, content, ref_time, source, created_at)` is immutable and content-addressed, like qmd's `content`. It is the ground truth.
- `facts(id, subject, predicate, object, text, valid_at, invalid_at, created_at, expired_at, superseded_by, merged_into, state, confidence, scope)` is the Graphiti edge flattened.
- `fact_episodes(fact_id, episode_id)` is the provenance list.
- `fact_fts` (FTS5 over `text`) and `fact_vec` (sqlite-vec) are the retrieval indexes. `embed_fingerprint` per vector marks what needs re-embedding after a model change.
- `llm_cache(hash, result)` is carried over from qmd.
- `facts.state` is `active | superseded | merged | proposed | rejected`.
- Reads default to active facts only. A history flag adds superseded ones, like Mem0's `latest_only`.

**Write path, a state machine in Rust:**
1. Extract candidate facts from an episode. This is a small-model call returning structured output.
2. Dedupe on a normalized (subject, predicate, object, text) key. A verbatim hit appends only a provenance row.
3. Fetch candidates: same-pair facts for duplicates, plus a top-k hybrid search for contradictions.
4. Ask the LLM for `{duplicate_idx[], contradicted_idx[]}` over one continuous index space. Reject out-of-range indices.
5. Apply invalidation in plain Rust with Graphiti's interval rules. Never let the LLM write timestamps directly.
6. Normalize future-tense and deadline phrases to Graphiti's rule: `valid_at` is the statement time, and a deadline is not an end.

**Contradiction policy:**
- Copy Graphiti's prompt rules: contradiction needs explicit incompatibility, not recency or topic overlap.
- Role or ownership facts are never invalidated by activity facts.
- Add a gate Graphiti lacks. If a contradicted fact is high-confidence or user-authored, or the contradiction came from a non-user source, park it as `proposed` and ask for approval. Otherwise auto-supersede, which is reversible because rows are kept.

**Dream jobs (a tokio scheduled task plus a durable job table):**
- Triggers are the same two Letta uses (N agent steps or context compaction), plus a nightly cron and a minimum-memory threshold like Mem0's 20.
- Passes:
  1. Merge near-duplicates.
  2. Supersede stale facts.
  3. Synthesize pattern facts that link to their sources, idempotent per source set.
  4. Expire facts past `expiration_date`.
- Dream writes only `proposed` rows or non-destructive state changes. A diff is emitted per run.
- Copy Letta's backup-before-reorganize: snapshot with `VACUUM INTO` or a copy before any bulk pass. A SQLite savepoint per pass makes each run atomic.
- Letta's "agent reviews its own changes" step is not human approval. Copy it as a cheap second check, and keep a separate approval rule for high-impact changes.

**Approval rules, borrowed from qmd's trust model:**
- Anything that executes (hooks, update commands) or reaches outside its scope (new paths, model URIs) requires approval stored against the exact gated set. A changed set re-prompts. Headless callers skip, not auto-approve.
- Daemon: bind to loopback, validate `Origin` and `Host`, and add real auth. qmd's own docs say its endpoints are unauthenticated.

## CLAIMS

- Graphiti separates event time from transaction time; contradiction is resolved by deterministic interval logic after an LLM picks candidate indices — RISK: low — SOURCES: edges.py, edge_operations.py, dedupe_edges.py @ d50741d — PRIMARY
- Graphiti's prompt is tuned to under-invalidate — RISK: low — SOURCES: dedupe_edges.py — PRIMARY
- qmd has no contradiction or consolidation logic — RISK: medium (absence claim; I searched the README and the `store.ts` schema but not every file) — SOURCES: qmd README, store.ts — PRIMARY
- Mem0 OSS auto-extraction is ADD-only; supersede and merge are Platform features — RISK: medium (the OSS code may have changed; I read docs only, not Mem0 source) — SOURCES: how-it-works.md, dream.md, llms.txt — PRIMARY (docs)
- Letta dreaming's review step needs no user approval — RISK: low — SOURCES: configuration/memory.md — PRIMARY
- Letta dreaming triggers on step count or compaction — RISK: medium (docs only, source not seen) — SOURCES: configuration/memory.md — PRIMARY (docs)
- Letta's old sleep-time docs URL is gone — RISK: low — SOURCES: 404 response — PRIMARY
- The SQLite + FTS5 + sqlite-vec design fits a Rust daemon — RISK: medium (inference; qmd proves the schema in TS, not in Rust) — SOURCES: qmd store.ts — SECONDARY (inference)

## EXPAND

- Read the Graphiti extraction prompt (`prompts/extract_edges.py`) and `search_config_recipes` for the candidate-retrieval details.
- Read the Letta server source for the dreaming/memory subagent trigger and the worktree merge flow.
- Read the Mem0 OSS Python `memory/main.py` for the actual add and dedupe code, to confirm the ADD-only claim against the code.
- Read the qmd `vec-layout.ts` per-collection vector partitioning and the `store-migrations.ts` migration pattern.
- Check the Rust crates `rusqlite`, `sqlite-vec` and `tokio-cron` for fit. This is outside the assigned sources.