<!-- task=st_01a144f4 -->
st_01a144f4 [completed] model mahoquot/claude-sonnet-5-5
Ask: research reception and failure modes of OpenAI Dots, critique angle - wanted: a sourced report with identity, findings, gateway implications, claims and leads. For you: the report is done. The main finding is that hands-on critique is thin. It consists of one HN failure report, one vendor-neutral benchmark and the HN launch thread. Press reviews could not be fetched. Now: done. Next: none.

## Identity
- **Name:** "Dots" (singular "a dot"), described as always-on agents inside ChatGPT. The agent runs on GPT-6 Astra, has its own cloud computer, and keeps notes and memory across conversations. [https://news.ycombinator.com/item?id=49901067] (OpenAI's own email, pasted by a Pro subscriber)
- **Company and official URL:** OpenAI, at `https://openai.com/index/introducing-dots/`. That is the URL the HN submission points to. I could not read it directly because the openai.com fetch returned a Cloudflare challenge page. [https://news.ycombinator.com/item?id=49896604]
- **Launch date:** September 29, 2026. That is the HN submission date (770 points, 647 comments), and Amplifying's report also says OpenAI "introduced Dots on September 29, 2026". [https://amplifying.ai/research/openai-dots-choose]
- **Unconfirmed seed claims:**
  - "Unveiled at DevDay" is unconfirmed. Amplifying mentions a "launch presentation" with Sam Altman, but nothing I read calls it DevDay.
  - "4000+ connected apps" is unconfirmed. An HN search for it returned nothing.
- **Microsoft:** this is confirmed only as an enterprise integration. An HN commenter quotes OpenAI as "working with Microsoft to integrate specialist dots with... Agent 365". [https://news.ycombinator.com/item?id=49898108]
- **Not the same product:** MyClaw's blog notes Dots is "separate from the unrelated apps and services also named Dot or Dots". [https://myclaw.ai/blog/dots-ai]
- **Confidence:** high that the entity exists and is as described. Medium on details, because the primary page was blocked and I relied on HN quotes of it and on third-party summaries.

## Findings
**Hands-on failures**
- One HN user, a developer, called Dots "Super unimpressed". During a GitHub-issue task:
  - The cloud machine "died multiple times".
  - It had no preview-link support.
  - Even with "unrestricted" internet access it could not reach his tunnel service on a non-standard port.
  - It then went silent overnight, showing a typing indicator with no reply.
  - He said Claude finished 6+ issues end to end, including preview URLs.
  - His verdict was "a glimpse of the future, but it's not ready". This is one anecdote and the only first-hand failure report I found. [https://news.ycombinator.com/item?id=49920558]
- Amplifying ran the same app prompt eight times on Dots. All 8 apps went public, with first delivery in 13 to 29 minutes. [https://amplifying.ai/research/openai-dots-choose]
- The same Amplifying tests found reliability gaps:
  - Scheduled scraping was inactive in some runs. In Runs 3, 5 and 7 the schedule was missing or the collector connection was pending. [https://amplifying.ai/research/openai-dots-choose]
  - Some sources stayed blocked or unsupported. Mistral was blocked, Bedrock had no usable feed, and OpenRouter returned HTTP 503 in one run. [https://amplifying.ai/research/openai-dots-choose]
  - The authors called background operation "unverified" in most runs. In one run, Run 4, they observed new collection timestamps while the browser was closed. [https://amplifying.ai/research/openai-dots-choose]
  - The apps reported "testing" steps, and the authors did not independently verify those results. [https://amplifying.ai/research/openai-dots-choose]
- Amplifying's "Dots vs. Muse" comparison, using the same prompt eight times per product, found Dots delivered 8/8 public apps and Meta Muse 0/8 verified public apps. [https://amplifying.ai/research/dots-vs-muse]
  - Muse's local servers needed tunnels or hosting.
  - Both products "still needed work" on scheduling and scraping.
- Vendor lock-in surfaced in the benchmark. Dots used Cloudflare Workers, D1 and ChatGPT sign-in in 8/8 builds, with no comparison of alternatives. The authors say OpenAI "keeps the customer relationship through the platform running it". This is the authors' interpretation. [https://amplifying.ai/research/openai-dots-choose]
- MyClaw lists limits in its own review (a vendor selling managed OpenClaw hosting, so read it as promotional):
  - Website logins, verification steps and cloud-browser restrictions can interrupt unattended runs.
  - Local-computer tasks need the machine online with the ChatGPT app open.
  - "Pause" stops only the current main task and does not undo completed actions.
  - Custom rules "can also be followed imperfectly", so pair them with real app permissions. [https://myclaw.ai/blog/dots-ai]
- Access is restricted. Pro users must be outside the EEA, UK and Switzerland, and over 18. Enterprise is off by default. Creating a dot on mobile is unsupported. [https://myclaw.ai/blog/dots-ai] (as of Sep 30)

**Usage limits and pricing backlash**
- OpenAI's Pro email, pasted on HN, bundled dots into Pro. In the same message it:
  - Added a Pro 500 tier.
  - Cut Pro 200's Work and Codex allowance from 20x to 10x Plus from October 30.
  - Cut GPT-6 Pro chat from 200 to 100 messages per week.
  - Gave a one-time credit grant of $2,500 value. [https://news.ycombinator.com/item?id=49901067]
- Conversations with a dot are free of usage limits, but Codex or Work tasks it starts do count. One HN commenter pointed this out. [https://news.ycombinator.com/item?id=49899382]
- Another HN commenter called the launch "Most predictable announcement ever" and wanted scheduled-task limits removed in favour of plain usage. [https://news.ycombinator.com/item?id=49899005]

**Blocking by sites (Amazon)**
- Amazon blocked Meta's Muse agent from its retail site, citing credentials and transparency concerns (The Register, Sep 21, 2026). I found no source saying Amazon blocked Dots. [https://www.theregister.com/ai-and-ml/2026/09/21/amazon-shows-metas-muse-ai-shopping-agent-the-door/5297777]
- An HN commenter on a later thread wrote that "Amazon blocks Muse/Dots", but gave no source. Treat that as unverified for Dots. [https://news.ycombinator.com/item?id=50037849]
- A first-hand HN poster predicted that exclusivity deals with service providers will become the moat. [https://news.ycombinator.com/item?id=49900817]
- Another poster predicted that cloud-VM agents will be easy to block because egress IPs are identifiable. The third commenter in this thread said you can't block every DigitalOcean or Cloudflare egress IP. That is speculation. [https://news.ycombinator.com/item?id=50037849]

**Trust, security and privacy**
- Dots uses a password store whose credentials are "not exposed to the model". An HN commenter quoted this from the announcement while rebutting a claim that credentials leak. This is vendor-asserted. [https://news.ycombinator.com/item?id=49904285]
- The worries on HN were prompt injection (an agent "with access to all my stuff"), cloud lock-in, and a large vendor holding personal data. [https://news.ycombinator.com/item?id=49897306] [https://news.ycombinator.com/item?id=49898456] [https://news.ycombinator.com/item?id=49897297]
- A commenter argued that approving every action trains users to say yes, so it stops being a control. They proposed gating on classes of action: reads free, mutating writes ask, and a new outbound host or credential always asks. [https://news.ycombinator.com/item?id=49904634]
- Third-party Show HN projects exist because Dots has Custom Rules (Allow / Require approval / Block). One author says he built a rules directory because nobody was sharing rules, and that all 26 of his entries are untested. [https://dotbotdirectory.com]

**Positioning and competitor framing**
- Commenters repeatedly described Dots as "OpenClaw absorbed into ChatGPT" or "OpenClaw but from OpenAI". One noted OpenAI employs OpenClaw's creator. [https://news.ycombinator.com/item?id=49910988] [https://news.ycombinator.com/item?id=49906298]
- The same commenter had run an OpenClaw instance for months on a VM and called it "fun but also very flaky". [https://news.ycombinator.com/item?id=49906298]
- Competitors named in the thread are Meta Muse, xAI's Grok Bot, Claude, Codex and Apple's Siri/Gemini. [https://news.ycombinator.com/item?id=49906751]
- A skeptical thread says the target is non-technical users who lack a good Codex or Claude Code setup. Others say the use cases are marginal over scripts or first-party features. [https://news.ycombinator.com/item?id=49907150] [https://news.ycombinator.com/item?id=49904418]
- One reviewer-style article claims an agent swarm of Dots produced a new math result with Lean verification. The title is admitted clickbait and I did not verify the claim. [https://unexcitedneurons.substack.com/p/i-used-openai-dots-as-an-agent-swarm]

**Not found**
- I found no incident postmortems or mainstream hands-on press reviews. The general web search tool was not available to me, so coverage of press outlets (CNA, Unite.AI, tickernews) is limited to what other pages linked.
- Reddit was not searched. I only used HN's Algolia search and direct page fetches.

## Implications for a self-hosted personal agent gateway
These are my inference, drawn from the failures above.
1. **Visible liveness and dead-man alerts.** Dots' silent overnight stall with a typing indicator is the worst failure mode. Add a heartbeat per lane and push "lane X silent for N min" to the owner on Discord or iMessage.
2. **Schedule verification, not only configuration.** Several Dots builds shipped with a schedule that was never confirmed running. Have cron jobs record "last successful run" and surface "stale/unknown, not healthy" in every status view.
3. **Action-class approval policy.** Instead of per-call prompts, allow reads, ask on mutations, and always ask on a new outbound host or credential. Keep the approval history so you can see which prompts were rubber-stamped.
4. **Hard permissions behind soft rules.** Natural-language rules are "followed imperfectly", so enforce egress allow-lists and per-connector scopes in the Rust daemon rather than in prompts.
5. **Stop controls that actually stop.** Dots' Pause leaves delegated tasks running. Provide one command that kills a whole lane, including its omo child agents, and states what was already done.
6. **Quarantine inbox ingestion.** Treat inbound email and messages as untrusted. Give the model read-only tools until the owner approves, and flag instruction-like text.
7. **Home-IP advantage with explicit escalation.** A Mac on a residential IP is less likely to be blocked than a cloud-VM agent. When a site still blocks, or demands a login or CAPTCHA, hand off to the owner over chat with a resume token rather than retrying.
8. **Preview and test-evidence outputs for coding lanes.** The Dots user cited missing preview URLs as a reason Claude won. Have omo lanes return a tunnelled preview or test log with each finished task.

## CLAIMS
- CLAIM: Dots is an OpenAI always-on agent in ChatGPT, powered by GPT-6 Astra, with its own cloud computer, launched around Sep 29, 2026. — RISK: normal — SOURCES: news.ycombinator.com, amplifying.ai, myclaw.ai — PRIMARY: https://openai.com/index/introducing-dots/ (not directly fetched)
- CLAIM: Dots is rolling out to Pro, Business Premium and Enterprise, excluding the EEA, UK and Switzerland for Pro. — RISK: normal — SOURCES: myclaw.ai — PRIMARY: none (OpenAI help page not fetched)
- CLAIM: A first-hand user reported Dots' cloud machine died repeatedly and stopped replying overnight, while Claude finished more issues end to end. — RISK: high (single anecdote) — SOURCES: news.ycombinator.com — PRIMARY: https://news.ycombinator.com/item?id=49920558
- CLAIM: In an 8-run benchmark Dots published 8/8 public apps via ChatGPT Sites, but some scheduled-scraping tasks were missing or unverified. — RISK: normal — SOURCES: amplifying.ai — PRIMARY: https://amplifying.ai/research/openai-dots-choose
- CLAIM: Amazon blocked Meta's Muse agent from its retail site. — RISK: normal — SOURCES: theregister.com, bloomberg.com, forbes.com, neowin.net — PRIMARY: https://www.theregister.com/ai-and-ml/2026/09/21/amazon-shows-metas-muse-ai-shopping-agent-the-door/5297777
- CLAIM: Amazon blocked Dots specifically. — RISK: high — SOURCES: news.ycombinator.com (uncited comment only) — PRIMARY: none. No supporting source found.
- CLAIM: The launch coincided with Pro 200 usage cuts and a new Pro 500 tier. — RISK: normal — SOURCES: news.ycombinator.com (quoted OpenAI email) — PRIMARY: https://news.ycombinator.com/item?id=49901067
- CLAIM: OpenAI is working with Microsoft to integrate dots with Agent 365. — RISK: normal — SOURCES: news.ycombinator.com (quote of announcement) — PRIMARY: https://news.ycombinator.com/item?id=49898108
- CLAIM: Dots was unveiled at DevDay and connects 4000+ apps. — RISK: high — SOURCES: none found — PRIMARY: none

## EXPAND
- LEAD: Read `https://openai.com/index/introducing-dots/` and OpenAI's Help Center page on dots, through a browser or another route. — WHY: this would confirm the DevDay, 4000+ apps, rules and approval claims against the primary source. — ANGLE: official capability and limits.
- LEAD: Read the HN thread `https://news.ycombinator.com/item?id=49896604` beyond keyword searches, especially comments after Oct 2. — WHY: there may be more hands-on failure reports, since 647 comments exist and I only sampled by search. — ANGLE: user complaints.
- LEAD: Search Reddit (r/OpenAI, r/ChatGPT, r/singularity) with the `.json` endpoints. — WHY: Reddit was not covered here and usually has more complaints about stalls and limits. — ANGLE: user complaints.
- LEAD: Read CNA, Unite.AI and Forbes coverage of the launch. — WHY: these were the press pieces the seed cited, and I could not fetch them. — ANGLE: press reviews and Microsoft details.
- LEAD: Read Bloomberg and Forbes on the Amazon–Muse block, and check whether Amazon has acted against Dots. — WHY: it would settle whether Dots is blocked too. — ANGLE: site blocking.