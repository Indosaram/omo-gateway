<!-- task=st_01a14503 -->
st_01a14503 [completed] model mahoquot/claude-sonnet-5-5
I read this as a research-only request. I'll stop once the report is written with sources.

Ask: research chatgpt_agent and its competitors (background/scheduled execution, approvals, memory) - wanted: a sourced report of at most 1800 words. For you: the Anthropic and Google evidence is primary or press-verified. The OpenAI product pages could not be read, so those items are secondary only. Now: report delivery. Next: none.

## Identity
- **Canonical subject:** "ChatGPT agent" is OpenAI's browser, terminal and connector agent, launched 2025-07-17. It merged Operator (launched about 2025-01-23) with deep research. Confidence is high on name and company, and medium on exact feature state in October 2026. [https://openai.com/index/introducing-chatgpt-agent/ (surfaced via https://hn.algolia.com/api/v1/search?query=%22ChatGPT%20agent%22)]
- **Official URLs:** the launch post is above. The help article is https://help.openai.com/en/articles/11752874-chatgpt-agent. I could not read either page: openai.com returned a Cloudflare challenge and the proxy was rate-limited. I could only confirm that they exist.
- **Scope entities, all verified:**
  - Anthropic: Claude Cowork, which merged into the main Claude app on 2026-09-16, and Claude in Chrome.
  - Google: Project Mariner, shut down 2026-05-04, with its technology moved into Gemini Agent, AI Mode and Chrome "auto-browse".
  - OpenAI: ChatGPT Pulse, launched 2025-09-25.
- **Not researched in depth:** Anthropic computer-use API internals. Gemini Agent scheduling was not verified.

## Findings

**Anthropic (primary sources, strongest evidence)**
- Cowork is where you hand Claude a task and it works across files, calendar, email, messaging, web and connected tools. [https://claude.com/blog/cowork-web-mobile]
- Cowork launched as a research preview on 2026-01-12. [https://claude.com/blog/cowork-research-preview, via HN https://news.ycombinator.com/item?id=46593022]
- Cowork came to mobile and web on 2026-07-07, rolling out starting with Max. [https://claude.com/blog/cowork-web-mobile]
- Scheduled tasks now run with no device online. The example is a Monday 6am client-prep task that builds a briefing doc and leaves the follow-up email drafted but unsent. [https://claude.com/blog/cowork-web-mobile]
- When Claude hits a decision only the user can make, it asks and the question is pushed to the user's phone. Anthropic says nothing ships until the user has reviewed and approved it. [https://claude.com/blog/cowork-web-mobile]
- Desktop remains the "full" surface. Local files and the local browser are desktop-only, while web and mobile run the background-capable version. This implies a split between a local-capability runner and a cloud runner (my inference). [https://claude.com/blog/cowork-web-mobile]
- On 2026-09-16, Cowork and chat merged into one Claude on Pro and Max. A task can start from any conversation, and you can schedule a recurring report ("every Monday"). [https://claude.com/blog/cowork-is-now-claude]
- Approval is a user-selectable mode. The default is that Claude asks before taking an action. Users can opt into letting it keep working and check in only when something needs a closer look. [https://claude.com/blog/cowork-is-now-claude]
- Anthropic's usage analysis says over 90% of Cowork use is not software development. The largest categories are business operations and content creation. [https://claude.com/blog/cowork-web-mobile]
- Claude in Chrome reads the signed-in page and then clicks, types and fills forms. [https://claude.com/claude-in-chrome]
- It has two approval regimes:
  - An "act without asking each step" mode, in which a separate safety check reviews each action for risk and hidden instructions before it runs.
  - A Permissions Mode, in which the user grants access one site at a time and Claude cannot reach anything unapproved.
  [https://claude.com/claude-in-chrome]
- Purchases, financial actions and other "one-way doors" always wait for the user. On Team and Enterprise, admins set org-wide on/off and site allowlists and blocklists. [https://claude.com/claude-in-chrome]
- Anthropic itself says prompt injection defenses "aren't foolproof" and advises avoiding banking, health and credentials. [https://claude.com/claude-in-chrome]
- Cowork now ships a built-in, separate browser. Claude in Chrome stays the default if you already use it, and you can change this in Settings → Cowork → Preferred browser. [https://claude.com/claude-in-chrome]
- Security criticism came early. PromptArmor reported that Cowork exfiltrates files (HN, 870 points). [https://www.promptarmor.com/resources/claude-cowork-exfiltrates-files]
- Cowork runs a Linux VM through Apple's virtualization framework (Simon Willison gist). [https://gist.github.com/simonw/35732f187edbe4fbd0bf976d013f22c8]
- A user video reports Cowork deleting 11GB of files (anecdote). [https://www.youtube.com/watch?v=_6C9nMvQsGU]
- Cowork "Dispatch" was announced on 2026-03-17. I only saw the headline and have not seen the content, so it is likely a phone-to-desktop task handoff (speculation). [https://www.threads.com/@boris_cherny/post/DWAE3-_E8ui]

**OpenAI (secondary only; primary pages unreadable)**
- ChatGPT agent launched on 2025-07-17 (HN thread, 686 points). [https://openai.com/index/introducing-chatgpt-agent/]
- The EU launch followed on about 2025-08-09. [https://help.openai.com/en/articles/11752874-chatgpt-agent]
- A third-party quote of the launch limits says Pro gets 400 messages per month and other paid plans get 40, with credits available beyond that. This is second-hand and may be outdated. [https://news.ycombinator.com/item?id=44916639]
- Ars Technica reported that the agent clicked through an "I am not a robot" check. [https://arstechnica.com/information-technology/2025/07/openais-chatgpt-agent-casually-clicks-through-i-am-not-a-robot-verification-test/]
- A system card exists for ChatGPT agent. [https://cdn.openai.com/pdf/6bcccca6-3b64-43cb-a66e-4647073142d7/chatgpt_agent_system_card_launch.pdf]
- ChatGPT Pulse was launched on 2025-09-25 as an overnight, proactive research feed (HN, 627 points). ZDNet describes it as working overnight to produce personalized morning updates. [https://openai.com/index/introducing-chatgpt-pulse/, https://www.zdnet.com/article/chatgpt-pulse-works-overnight-to-produce-personalized-morning-updates-for-you-how-it-works/]
- Altman said there were no current plans for ads in Pulse, but did not rule them out. [https://www.theverge.com/news/793073/chatgpt-pulse-no-plans-for-ads-sam-altman]
- Pulse reception on HN centered on privacy and "connect everything" skepticism. Pulse's value is disputed (an Ask HN from 2026-05 asks whether anyone gets value). [https://news.ycombinator.com/item?id=45375477, https://news.ycombinator.com/item?id=48062680]
- On 2026-10-15, The Verge ran a headline that "ChatGPT can peek at your Chrome tabs now". This shows OpenAI moving toward reading the user's own browser context. I saw the headline only, not the article body. [https://www.theverge.com/ai-artificial-intelligence/1012510/openai-chatgpt-google-chrome-tabs]
- I could not verify the current state of ChatGPT scheduled Tasks, Pulse memory controls, or the approval flow ("takeover mode"). Treat these as gaps.

**Google**
- Project Mariner was revealed in December 2024. It later supported up to 10 parallel tasks. [https://www.theverge.com/tech/925559/google-project-mariner-shut-down]
- Mariner was shut down on 2026-05-04. Google's landing page says its technology "voyaged to other Google products". [https://www.theverge.com/tech/925559/google-project-mariner-shut-down]
- Its capabilities live on in Gemini Agent (which can archive emails or help book a hotel) and in AI Mode. [https://www.theverge.com/tech/925559/google-project-mariner-shut-down]
- Chrome "auto-browse" was shown early in 2026 for multi-step tasks. Google has not said whether it is powered by Mariner. [https://www.theverge.com/tech/925559/google-project-mariner-shut-down]
- The pattern across all three vendors is that standalone agent research previews get folded into mainstream products. Mariner did, and Cowork merged into chat (my inference).

**Open-source and community signals (HN, 2026)**
- Self-hosted Matrix assistant Silta runs Claude Code with scheduled tasks, per-user sandboxes, and handoff and compaction to preserve context. Its author notes the sandbox does not protect against prompt injection leaking data. [https://github.com/dmitry-markin/silta]
- Valmis uses a credential-ID proxy: the agent container asks the host to make API calls, so the container can run with no internet access. It has cron, webhook and app-event triggers. [https://github.com/valmishq/valmis]
- Rook is a browser-extension agent with scheduled routines and Slack/Discord channels, plus a phone companion for captchas. [https://chromewebstore.google.com/detail/rook/opojeelojlkcinlhkbahpcekdolfjmhi]
- BrowserOS proposes browser-level ACLs ("agent can never click buy"). [https://github.com/browseros-ai/BrowserOS]
- Taurus Agents gives each persistent agent its own `MEMORY.md`, episodic continuity logs, and a container. [https://taurusagents.com/]
- Screenpipe offers a local, MCP-accessible memory of screen activity, with PII redaction. [https://github.com/screenpipe/screenpipe]

## Implications for a self-hosted personal agent gateway
1. **Draft-don't-send as the default for scheduled lanes.** Cron-triggered runs should produce artifacts (drafts, diffs, summaries) and park outbound side effects. Anthropic's pattern leaves the follow-up email drafted but unsent.
2. **Approval inbox over chat transports.** When a run reaches an irreversible step, push a structured question to Discord, iMessage or KakaoTalk with approve/deny/redirect replies. Store the pending approval durably so a daemon restart does not lose it. Cowork's phone-routed questions are the model.
3. **Per-lane autonomy modes.** Offer "ask before every action" as the default and "check in only on risk" as an opt-in, per lane or per task. Mirror Claude in Chrome's Permissions Mode with per-site or per-tool grants.
4. **Hard "one-way door" classes that ignore autonomy mode.** Examples are payments, outbound messages to third parties, deletion beyond a size threshold, and credential use. These always require owner approval. Cowork's reported 11GB deletion argues for a delete-count or size guard.
5. **Credential proxy for omo agents.** Follow Valmis: agents request actions by credential ID and the daemon makes the call. This limits exfiltration from prompt-injected inbox content and can run the agent with network egress off.
6. **Inbox ingestion treated as untrusted input.** Tag content from email, iMessage and web as tainted. A tainted run cannot use outbound or side-effect tools without approval. This is cheap taint tracking, and every vendor here concedes injection defenses are imperfect.
7. **File-based memory with provenance.** Use per-lane markdown memory plus an episodic log, as Taurus and Silta do, with a compaction or handoff step. Add a "what I remember" command and a forget command, since Pulse drew privacy criticism.
8. **A Pulse-style digest job.** A nightly cron lane can produce a morning briefing from calendar, inbox and repos. Make it opt-in per source and rate-limited. Pulse's disputed value suggests keeping it narrow and actionable.

## CLAIMS
- CLAIM: Cowork scheduled tasks run in the background with no device online, and ask the user's phone when a human decision is needed. — RISK: normal — SOURCES: claude.com — PRIMARY: https://claude.com/blog/cowork-web-mobile
- CLAIM: Cowork and chat merged into one Claude on 2026-09-16, with a default of asking before actions and an opt-in to check in only on risk. — RISK: normal — SOURCES: claude.com — PRIMARY: https://claude.com/blog/cowork-is-now-claude
- CLAIM: Claude in Chrome has a safety-classifier mode and a per-site Permissions Mode, and purchases and financial actions wait for the user. — RISK: normal — SOURCES: claude.com — PRIMARY: https://claude.com/claude-in-chrome
- CLAIM: Anthropic admits browser-agent prompt injection defenses are not foolproof. — RISK: high — SOURCES: claude.com — PRIMARY: https://claude.com/claude-in-chrome
- CLAIM: Google shut down Project Mariner on 2026-05-04 and folded it into Gemini Agent and AI Mode. — RISK: normal — SOURCES: theverge.com — PRIMARY: https://labs.google.com/mariner/landing (cited by The Verge, not fetched by me)
- CLAIM: ChatGPT agent launched on 2025-07-17 and Pulse on 2025-09-25. — RISK: normal — SOURCES: hn.algolia.com, openai.com — PRIMARY: https://openai.com/index/introducing-chatgpt-agent/ (unreadable; date from HN submission timestamp)
- CLAIM: ChatGPT agent had launch message limits of 400/month for Pro and 40/month for other paid plans. — RISK: normal — SOURCES: HN Show HN quote — PRIMARY: none (secondhand, possibly outdated)
- CLAIM: ChatGPT can now read Chrome tabs (2026-10-15). — RISK: normal — SOURCES: theverge.com headline via HN — PRIMARY: none (article body not read)
- CLAIM: Cowork exfiltrated files in a PromptArmor demo. — RISK: high — SOURCES: promptarmor.com via HN — PRIMARY: https://www.promptarmor.com/resources/claude-cowork-exfiltrates-files (not fetched; HN title only)

## EXPAND
- LEAD: OpenAI help-center and release-notes pages for ChatGPT agent, Tasks, Pulse and Codex automations — WHY: primary evidence on scheduling, approvals and memory was unreadable here, which is the largest gap — ANGLE: retry from a different network, or use `gh` against any OpenAI-hosted docs repos.
- LEAD: Anthropic help-center article 15520349 (Cowork surfaces) and 12902428 (Claude in Chrome safely) — WHY: exact per-surface capabilities and the risk model — ANGLE: fetch support.claude.com directly with `curl`.
- LEAD: Anthropic's "prompt-injection-defenses" research post — WHY: it describes the classifier approach behind the act-without-asking mode — ANGLE: https://anthropic.com/research/prompt-injection-defenses.
- LEAD: Gemini Agent and Chrome auto-browse documentation — WHY: scheduling and approval behavior was not verified — ANGLE: Google blog and support.google.com.
- LEAD: "Dispatch" for Cowork — WHY: phone-to-desktop handoff is directly relevant to a Mac daemon — ANGLE: Boris Cherny's Threads post and Claude release notes.