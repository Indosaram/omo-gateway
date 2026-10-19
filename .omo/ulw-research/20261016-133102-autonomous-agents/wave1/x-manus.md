<!-- task=st_01a14504 -->
st_01a14504 [completed] model mahoquot/claude-sonnet-5-5
Ask: research Manus, Genspark Super Agent and Perplexity Comet/Assistant (cloud-sandbox general agents) and deliver a sourced report. Wanted: the report in the requested format. For you: the report is complete, but a few primary pages could not be fetched (gaps listed at the end). Now: none, all tasks done. Next: none.

The web_search tool was not available this turn. I used HN Algolia, gh/curl, and primary-site fetches through r.jina.ai, which rate-limited some calls. Dollar-per-task figures are therefore anecdotal, not from price pages.

## Identity

- **Manus**
  - Company: Manus (Butterfly Effect / Monica.im lineage). Founded in China, HQ moved to Singapore. Official URL: https://manus.im.
  - Launched its first general agent in March 2025, per CNBC. Claimed $100M ARR in December 2025. [https://www.cnbc.com/2026/04/27/meta-manus-china-blocks-acquisition-ai-startup.html]
  - Confidence: high.
- **Genspark**
  - Company: Genspark Inc. Official URL: https://www.genspark.ai.
  - Super Agent launched 2 April 2025 (blog dated 2025-04-02). Product line now centers on "Genspark Claw" and "Cloud Computer". Confidence: high.
- **Perplexity**
  - Comet is a local browser, launched 9 July 2025 (TechCrunch via HN). Comet Assistant is its in-browser agent. It is not a cloud-sandbox agent.
  - The cloud-sandbox general agent is **Perplexity Computer**, launched 25 Feb 2026. **Portable Computer** followed on 25 Aug 2026.
  - Confidence: high on the entities. Medium on Computer details, because the primary blog post was unreachable (403 abuse block). I used ZDNet and HN instead.
- The scope line lumps Comet with the cloud agents. I treated Perplexity Computer as the real comparison.

## Findings

### Manus: execution infra and design

- Every Manus session runs on a dedicated cloud VM. Manus calls the product "a personal cloud computing platform", and says the VM's Turing-completeness is what makes it general. [https://manus.im/blog/introducing-wide-research]
- The agent loop is: model picks an action, the sandbox executes it, the observation is appended to the context. A typical task is about 50 tool calls. The input:output token ratio is about 100:1. [https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus]
- KV-cache hit rate is the key cost metric. With Claude Sonnet, cached input costs $0.30/MTok versus $3/MTok uncached. Practices: stable prompt prefix, append-only context, deterministic JSON serialization, no second-precision timestamps at the top of the prompt. [https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus]
- Manus does not remove tools mid-task. A state machine masks token logits, and tool names share prefixes (`browser_`, `shell_`) so groups can be forced. [https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus]
- The file system is the context. Compression is "restorable": a URL or file path stays while the content is dropped. The agent maintains a `todo.md` and rewrites it as it goes, to keep goals in recent attention. Errors are left in context rather than hidden. [https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus]
- Manus rewrote its agent framework four times. Its stated bet is context engineering over model training. [https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus]

### Manus: Wide Research and cost

- Wide Research, launched 31 July 2025, runs many parallel subagents. Each one is a full general-purpose Manus instance, not a role-specific agent, and each has its own VM. It launched for Pro users first. [https://manus.im/blog/introducing-wide-research]
- Pricing is credit-based. HN users describe the credit system as "too expensive & unpredictable". One user reported a single deep-research task burning about $40. These are anecdotes and are unverified. [https://news.ycombinator.com/item?id=49258764]
- Another HN commenter says Manus is billed at API rates, which is expensive compared with plan-based billing. This is unverified. [https://news.ycombinator.com/item?id=49258764]
- A user reported repeated sandbox resets that lost work and wasted credits. This is one anecdote. [https://news.ycombinator.com/item?id=46575192]
- Manus has cron-style scheduled tasks. This comes from one HN user and is unverified. [https://news.ycombinator.com/item?id=49258764]

### Manus: corporate and desktop

- Manus launched a Desktop app with "My Computer" in March 2026. It works on local files and apps, with approval prompts: "Allow Once" or "Always Allow". It was positioned against OpenClaw, which is local-first and open source. [https://www.cnbc.com/2026/03/18/metas-manus-launches-desktop-app-to-bring-its-ai-agent-onto-personal-devices.html]
- Meta announced the $2B acquisition on 29 Dec 2025. China's NDRC ordered it unwound on 27 April 2026. [https://www.cnbc.com/2026/04/27/meta-manus-china-blocks-acquisition-ai-startup.html]
- Meta began dismantling the deal in June 2026, cutting Manus staff off from Meta's internal data systems. [https://www.cnbc.com/2026/06/12/meta-reportedly-begins-dismantling-2-billion-manus-deal-on-beijings-orders.html]
- On 11 Aug 2026, Manus announced it is returning to independence. Data generated by affected users on or after 29 Dec 2025 was deleted on 23-24 Aug, with backup and restore tooling provided. Data is stored in the US and Singapore. [https://manus.im/blog/a-note-to-our-users]
- Takeaway: single-vendor cloud agents carry real continuity and data-residency risk.

### Genspark: Super Agent and Claw

- Genspark's philosophy is "Less Control, More Tools". It has no hardwired workflows, only an LLM in a plan/execute/observe/backtrack loop. [https://www.genspark.ai/blog/genspark-less-control-more-tools]
- The engine mixes 9 LLMs of different sizes, 80+ specialized tools and 10+ premium datasets. Besides terminal, browser and computer access, tools include a Python interpreter, slides, and the Crunchbase API. [https://www.genspark.ai/blog/genspark-less-control-more-tools]
- An LLM judge scores whole sessions. The reward is attributed per step and feeds tool-selection and routing improvements. [https://www.genspark.ai/blog/genspark-less-control-more-tools]
- Claimed growth is $36M ARR in 45 days. This is vendor-reported. [https://www.genspark.ai/blog/genspark-less-control-more-tools]
- Workspace 3.0 (12 March 2026) introduced **Genspark Claw**. It runs in a per-user, always-on, isolated "Cloud Computer". Users message it from WhatsApp, Telegram, Teams, Slack and more. Claimed run rate is $200M, and Series B is extended to $385M. [https://www.genspark.ai/blog/genspark-ai-workspace-3]
- The Claw page lists WhatsApp, LINE, Slack, Teams, Telegram and Discord as channels. It advertises "Your own Cloud Computer", persistent memory, and a promo of $9.99/mo with up to 10,000 bonus credits. [https://www.genspark.ai/genspark-claw]
- Genspark also ships Workflows across about 20 apps. It open-sourced GenOffice (Apache-2.0, about 9.8k stars), which includes a CLI and agent skill so Claude Code, Codex and Cursor can edit real .docx/.xlsx/.pptx files. [https://github.com/genspark-ai/genoffice]
- A Genspark Super Bowl ad in Feb 2026 prompted an HN thread asking whether it is "a thin wrapper". This is skeptical sentiment, not evidence. [https://news.ycombinator.com/item?id=46941269]
- Per-task pricing is not published on the pages I fetched.

### Perplexity: Comet and Computer

- Comet is a local browser agent, not a cloud sandbox. Brave disclosed indirect prompt injection in Comet in Aug 2025. [https://brave.com/blog/comet-prompt-injection/]
- Perplexity Computer is a multi-model orchestrator. Its reasoning core is Claude Opus 4.6. Nano Banana and Veo 3.1 handle images and video, Grok handles lightweight tasks, and GPT-5.2 handles long-context recall and web search. It runs "dozens of tasks in parallel", can work in the background for months, and checks in only "if it truly needs you". [https://www.zdnet.com/article/perplexity-computer-openclaw/]
- Computer runs in "a safe and secure development sandbox", so a security fault should not spread to the user's main network. This is Perplexity's claim, via ZDNet. [https://www.zdnet.com/article/perplexity-computer-openclaw/]
- Portable Computer (25 Aug 2026) runs the harness locally, tied to NVIDIA DGX Spark, with optional cloud escalation. A commenter says it is closed source and needs a Pro or Max account. [https://news.ycombinator.com/item?id=49439535]
- One HN user says Computer "requires buying credits" and that Perplexity upsells to it. [https://news.ycombinator.com/item?id=49439535]
- An unverified HN link claims a hack that gave "unlimited Claude Code" on Computer. Treat it as noise. [https://twitter.com/YousifAstar/status/2032214543292850427]

### Cross-cutting

- The market is converging on a per-user persistent cloud computer, reached through messaging apps. Manus Desktop and Portable Computer are both moving toward local-first, so the cloud-only model is eroding. [https://www.cnbc.com/2026/03/18/metas-manus-launches-desktop-app-to-bring-its-ai-agent-onto-personal-devices.html]
- One HN commenter argues Manus's harness advantages shrank as base models improved ("the bitter lesson"). This is opinion. [https://news.ycombinator.com/item?id=49258764]

## Implications for the self-hosted gateway

1. **Per-task ephemeral workspace.** Give each omo task or lane its own directory, optionally a macOS user or VM. Record a snapshot and rollback point so a reset cannot silently lose work, which is the failure HN users report with Manus. [https://news.ycombinator.com/item?id=46575192]
2. **Wide fan-out lane.** A "wide" job spawns N identical general omo workers, as in Wide Research. It needs a concurrency cap, a per-job budget, and a reducer step that merges results. [https://manus.im/blog/introducing-wide-research]
3. **Cost ledger per task.** Log tokens in and out, cache-hit ratio, and dollars per task, and show them in the Discord or iMessage reply. Opaque credits are the top complaint about Manus. Keep prompts append-only with a stable prefix to preserve the KV-cache. [https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus]
4. **Restorable context via files.** Persist a `todo.md` and URL or path references per task, and reload them on daemon restart or cron wake. Compress by dropping content but keeping pointers. [https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus]
5. **Approval tiers.** Implement "Allow Once" and "Always Allow" per tool, with approvals requested over Discord, iMessage or KakaoTalk. Add a quiet-by-default policy that pings the owner only "if it truly needs you". [https://www.cnbc.com/2026/03/18/metas-manus-launches-desktop-app-to-bring-its-ai-agent-onto-personal-devices.html] [https://www.zdnet.com/article/perplexity-computer-openclaw/]
6. **Inbox ingestion with a prompt-injection boundary.** Ingested mail and web content gets untrusted-data framing, and any action derived from it needs approval. Agentic browsers are demonstrably injectable. [https://brave.com/blog/comet-prompt-injection/]
7. **Model-router lane.** Route by task class, for example a coding model for omo and a cheap model for triage. Genspark and Perplexity both do this. Log the routing decision per step, as an input to later eval. [https://www.genspark.ai/blog/genspark-less-control-more-tools]
8. **Export and backup of all state.** Keep tasks, memory and cron definitions in plain files in your own control. The Manus unwind deleted users' data over a jurisdictional dispute. [https://manus.im/blog/a-note-to-our-users]

## CLAIMS

- CLAIM: Manus runs each session on a dedicated cloud VM, and Wide Research subagents are full general-purpose instances. — RISK: normal — SOURCES: manus.im — PRIMARY: https://manus.im/blog/introducing-wide-research
- CLAIM: Meta's $2B Manus acquisition was ordered unwound by China, and Manus announced a return to independence on 11 Aug 2026, with user-data deletion and restore. — RISK: high — SOURCES: manus.im, cnbc.com — PRIMARY: https://manus.im/blog/a-note-to-our-users
- CLAIM: Manus's cached input tokens cost 10x less than uncached on Claude Sonnet, and KV-cache hit rate is its key metric. — RISK: normal — SOURCES: manus.im — PRIMARY: https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus
- CLAIM: Genspark Claw runs in a per-user always-on Cloud Computer and takes messages from WhatsApp, Telegram, Slack, Teams, Discord and LINE. — RISK: normal — SOURCES: genspark.ai — PRIMARY: https://www.genspark.ai/blog/genspark-ai-workspace-3
- CLAIM: Genspark has a $200M run rate and a $385M Series B. — RISK: high — SOURCES: genspark.ai (vendor-reported, unverified) — PRIMARY: https://www.genspark.ai/blog/genspark-ai-workspace-3
- CLAIM: Perplexity Computer is a multi-model orchestrator running in a sandbox, with Opus 4.6 as its core. — RISK: normal — SOURCES: zdnet.com — PRIMARY: none (Perplexity blog unreachable)
- CLAIM: Comet is a local browser agent and was shown vulnerable to indirect prompt injection. — RISK: normal — SOURCES: brave.com, news.ycombinator.com — PRIMARY: https://brave.com/blog/comet-prompt-injection/
- CLAIM: Manus costs are high and unpredictable, for example about $40 for one deep-research task. — RISK: high — SOURCES: news.ycombinator.com (anecdote only) — PRIMARY: none

## EXPAND

- LEAD: Manus pricing and credit table, plus the help-center credit-consumption rules. — WHY: per-task cost is the central question, and I only have anecdotes. — ANGLE: fetch help.manus.im and manus.im/pricing, which were rate-limited this turn.
- LEAD: Perplexity Computer primary blog and pricing. — WHY: the sandbox and credit details were all second-hand. — ANGLE: retry perplexity.ai/hub/blog after the block expires (about 14:35 UTC), or check the Wayback copy.
- LEAD: Manus "My Computer" blog (manus.im/blog/manus-my-computer-desktop). — WHY: its local-execution permission model maps directly onto the gateway's approval design. — ANGLE: fetch the post directly.
- LEAD: The Meta-Manus "sandbox runtime code leaked" thread from March 2025 and dataleadsfuture's "Code Sandbox Tech Behind Manus". — WHY: it may describe the real VM and tool-server architecture. — ANGLE: read the thread and article, and treat both as unverified.
- LEAD: Genspark Claw pricing and its Cloud Computer specs. — WHY: it shows what an always-on per-user box costs. — ANGLE: genspark.ai/helpcenter and the pricing page.