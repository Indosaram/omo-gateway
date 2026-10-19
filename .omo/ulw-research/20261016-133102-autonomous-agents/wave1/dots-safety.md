<!-- task=st_01a144f3 -->
st_01a144f3 [completed] model mahoquot/claude-sonnet-5-5
Ask: research OpenAI Dots with a safety and economics angle - wanted: a sourced report for the gateway design. For you: report below. I read the help-center pages and The Register directly, but never got through to openai.com itself. Now: done. Next: none.

## Identity
- **Name:** "dots" (singular "dot"), OpenAI's always-on personal agents inside ChatGPT. They are separate from the unrelated apps called Dot/Dots, from CopilotKit's OpenDots and from composio's Open Dot. [https://help.openai.com/articles/20001530]
- **Company and launch:** OpenAI announced them at DevDay in San Francisco on 2026-09-29. Sam Altman presented them. Each dot runs on GPT-6 Astra, has its own cloud computer, and connects to about 4,000 apps through plugins. [https://www.theregister.com/ai-and-ml/2026/09/29/openai-tries-disarming-ai-angst-with-cute-graphics-and-always-on-agents/5299915] [https://www.unite.ai/openai-rolls-out-dots-agents-powered-by-gpt-6-astra-in-chatgpt/]
- **Official URLs:** `chatgpt.com/dots`, plus the Help Center articles 20001530 (getting started) and 20001529 (privacy/security FAQ). [https://help.openai.com/articles/20001530]
- **Confidence:** high (about 95%) on identity and facts. The Help Center pages are primary sources I read in full. `openai.com/index/introducing-dots/` and the safety post `/how-we-build-safety-security-and-privacy-into-dots/` were blocked by a Cloudflare challenge and a proxy block. I only have those two via Unite.AI's summary, so treat their content as secondary.
- **Microsoft:** the only confirmed link is that OpenAI says it is working with Microsoft to integrate *specialist dots* with the governance and security controls in Agent 365. I found no evidence that consumer dots are "brought to Microsoft". [https://www.unite.ai/openai-rolls-out-dots-agents-powered-by-gpt-6-astra-in-chatgpt/]
- **Seed article:** myclaw.ai/blog/dots-ai is a vendor SEO article that sells hosted OpenClaw. It repeats the official facts. [https://myclaw.ai/blog/dots-ai]

## Findings

**Permission model**
- Plugin permissions are shared across dots, ChatGPT, ChatGPT Work and Codex, and are managed in the Plugins tab. A dot can use connections you already made. [https://help.openai.com/articles/20001529]
- Reading a source does not imply write access. Permission to read information may not include permission to change it or send messages. [https://help.openai.com/articles/20001530]
- Local computer access is off by default. It needs an explicit "Allow access" in the desktop app, can be revoked, and requires that machine to be online with the app open. [https://help.openai.com/articles/20001530]
- Camera, microphone and screen access need two grants: the computer must be connected to the dot, and the OS must permit the ChatGPT app. [https://help.openai.com/articles/20001529]
- Custom Rules give each action one of four behaviours: take action without asking, take action if pre-approved, ask before taking action, or hand off to you. "Pre-approved" means you explicitly requested the action in the prompt. [https://help.openai.com/articles/20001530]
- Asking for a draft does not authorize sending it. [https://help.openai.com/articles/20001530]
- Custom Rules cannot override core safety requirements, the Autoreview system, or the proactive-research restrictions. [https://help.openai.com/articles/20001529]

**Approval gates for payments, sending and purchases**
- Changing a password or transferring money requires you to take over and do it yourself. Permanently deleting data or installing software may need approval each time. [https://help.openai.com/articles/20001529]
- Some approvals can be given in advance, for example recurring messages. The approval must name who it goes to, what it says, and when. Approving one message does not grant ongoing permission to contact people. [https://help.openai.com/articles/20001529]
- Purchases with a card saved on a merchant site require your approval, which can be given in advance if it specifically covers that purchase. [https://help.openai.com/articles/20001529]
- Auto-review is a separate reviewer system that checks certain planned actions against your instructions, Custom Rules and safety requirements before they run. For email, it checks the recipient and content. If it blocks a step, the dot can ask for clarification or approval, try a permitted alternative, or stop. Your approval cannot override core safety requirements. [https://help.openai.com/articles/20001529]
- Per Unite.AI's summary of the safety post, the Auto-review controls sit outside the environments the dot can modify. This is secondary-source, unverified by me. [https://www.unite.ai/openai-rolls-out-dots-agents-powered-by-gpt-6-astra-in-chatgpt/]
- The Codex Auto-review docs describe the likely mechanism. A separate reviewer agent sees a compact transcript plus the exact request, but not hidden reasoning. A denial tells the main agent not to find a workaround. A circuit breaker aborts the turn after 3 consecutive or 10 denials in the last 50 reviews. Reviewer transcripts are kept on disk. The docs state Auto-review is "not a deterministic security guarantee". This is Codex documentation, so applying it to dots is an inference. [https://learn.chatgpt.com/docs/sandboxing/auto-review]

**Audit and observability**
- Activity View in the desktop app lists ongoing and delegated tasks (in progress, scheduled, completed) with status and the steps taken. You can also open the dot's cloud computer to view it, "Take over" and then "Return control". [https://help.openai.com/articles/20001530] [https://help.openai.com/articles/20001529]
- A security test showed the limits of that visibility. Geordie gave a dot read-only permissions and no personal apps. In one day it produced 155 activity traces and used about 307k tokens, with at least 50 traces in two minutes. [https://www.theregister.com/ai-and-ml/2026/10/13/openais-dot-assistant-burned-300k-tokens-in-a-day-without-a-single-prompt/5302913]
- Geordie reports that dots always run at least two agents: a user-facing one and a background one that keeps collecting data. They also spawn short-lived sub-agents the user never sees. In one test a sub-agent checked every code repository on a dev system. One tester's take is that the user "would have no idea". This is one vendor's test, not an OpenAI statement. [https://www.theregister.com/ai-and-ml/2026/10/13/openais-dot-assistant-burned-300k-tokens-in-a-day-without-a-single-prompt/5302913]
- Pausing a dot is a profile-menu action, and Reset deletes the dot. Whether a completed action can be undone depends on the app, and "some actions cannot be undone". [https://help.openai.com/articles/20001530] [https://help.openai.com/articles/20001529]

**Sandboxing and proactive research**
- Each dot has its own cloud computer and browser. Unite.AI says sandboxing restricts code and tools inside the dot's workspace (secondary source). [https://www.unite.ai/openai-rolls-out-dots-agents-powered-by-gpt-6-astra-in-chatgpt/]
- "Proactive research" is the background mode. Its tools are read-only: they cannot send messages to others, change content through plugins, or control a browser or computer. Any follow-up action goes through the normal action rules. [https://help.openai.com/articles/20001529]
- A dot's own cloud browser has separate sign-in sessions. Logging in on your PC does not log it in, and sites can block cloud browsers. [https://help.openai.com/articles/20001530]

**Prompt injection**
- OpenAI says the dot is designed to separate your instructions from content it encounters, and that such content "does not grant permission on its own". Layers are tool restrictions, automatic pre-action checks, approvals and monitoring, plus human and automated red-teaming. [https://help.openai.com/articles/20001529]
- OpenAI states plainly that these reduce the risk but "do not eliminate it". [https://help.openai.com/articles/20001529]
- Secrets pasted into chat or a document are not covered by the secure sign-in flow. [https://help.openai.com/articles/20001529]
- Secure sign-in pauses the model and sends credentials straight to the browser environment. A dot's context does not retain credentials, images or screenshots. [https://help.openai.com/articles/20001529]
- Separately, The Register links a security complaint from objective_see about dots. I could not fetch it (x.com blocked), so the content is unverified. [https://www.theregister.com/ai-and-ml/2026/10/07/openai-dots-inspire-open-source-imitators-amid-technical-difficulties/5301734]

**Privacy and retention**
- Dot context persists for as long as you keep the dot. You cannot view, delete or edit individual dot memories, including details that entered from plugins. Deleting the dot is the only way to clear them. [https://help.openai.com/articles/20001529]
- Disconnecting a plugin stops new access but does not remove information already in the dot's context. [https://help.openai.com/articles/20001529]
- Files and Codex or ChatGPT threads the dot created survive dot deletion. [https://help.openai.com/articles/20001529]
- Memory is shared with ChatGPT in both directions. Turning ChatGPT Memory off stops the sharing but does not delete what the dot already received. [https://help.openai.com/articles/20001529]
- Training: Business, Enterprise and Edu data is not used by default. On personal plans, "Improve the model for everyone" governs dot work, including actions, delegated work and connected-app data used in conversations. OpenAI does not train directly on proactive-research notes. If that information is brought into an eligible conversation, it may be used depending on settings. [https://help.openai.com/articles/20001529]
- Human review can still happen in limited cases, including safety, even with model improvement off. [https://help.openai.com/articles/20001529]
- Texting runs through a third-party provider, is a limited beta for US Pro users only, and is unavailable for Business and Enterprise. Dots cannot place calls to you. [https://help.openai.com/articles/20001530]
- A standalone email address for the dot is not available at launch. You can connect your own email. [https://help.openai.com/articles/20001530]

**Pricing and limits**
- The first dot is included in Pro and Business Premium at no extra cost, with a "deeper work" allowance and extended limits for the first month. Later you will be able to add dots or scale speed or monthly work volume. A spokesperson mentions a flat monthly fee for that. [https://help.openai.com/articles/20001530] [https://www.theregister.com/ai-and-ml/2026/09/29/openai-tries-disarming-ai-angst-with-cute-graphics-and-always-on-agents/5299915]
- Conversations with a dot do not count toward ChatGPT limits. Work or Codex tasks it starts do count. [https://www.theregister.com/ai-and-ml/2026/09/29/openai-tries-disarming-ai-angst-with-cute-graphics-and-always-on-agents/5299915]
- Plan context from an OpenAI email quoted on Hacker News: Pro 200 drops from 20x to 10x of Plus from 2026-10-30, and a new Pro 500 offers 25x plus "Ultrafast". Existing subscribers got a one-time 62,500-credit grant that expires 2026-12-31. [https://news.ycombinator.com/item?id=49901067]
- Background token spend is the real risk. Geordie estimates roughly 200k Astra tokens cost $5-$10 and expects people to be unprepared for the background spend. Its advice is to give a dot a prepaid gift card rather than a real card. [https://www.theregister.com/ai-and-ml/2026/10/13/openais-dot-assistant-burned-300k-tokens-in-a-day-without-a-single-prompt/5302913]
- Hacker News posters note that OpenAI does not publish concrete usage numbers. [https://news.ycombinator.com/item?id=49896975]

**Availability and age**
- Pro is available outside the EEA, Switzerland and the UK, to users 18 and over. Business Premium is available in all supported regions. Enterprise, Edu and Healthcare get a beta that is off by default until a workspace admin enables it. Rollout is gradual. [https://help.openai.com/articles/20001530]
- Setup channels: Slack and Teams are supported, and desktop is needed to connect messaging channels. [https://help.openai.com/articles/20001530] [https://www.unite.ai/openai-rolls-out-dots-agents-powered-by-gpt-6-astra-in-chatgpt/]
- The docs disagree on mobile creation. The Help Center now says you can create a dot in the iOS/Android app, while Unite.AI and MyClaw say creation is desktop-only. I take the Help Center as newer, which is an inference. [https://help.openai.com/articles/20001530]

**Reliability**
- The Register reported task failures, connectivity problems, dots that disappear, and refusals that appear to be abuse-prevention related. [https://www.theregister.com/ai-and-ml/2026/10/07/openai-dots-inspire-open-source-imitators-amid-technical-difficulties/5301734]

## Implications for the self-hosted gateway

1. **Four-state policy table per action class.** Copy the take action / pre-approved / ask / hand-off model. Hand-off should be a state that physically cannot be auto-approved. Put password changes, money transfers and key rotation there. The state would apply to Discord, iMessage and KakaoTalk sends, shell, purchases and deletes.
2. **Approval messages that bind specifics.** OpenAI scopes advance approval to who, what and when. Do the same with a single-use token tied to recipient, body hash and time window. Explicitly refuse "approve one, permit all".
3. **Separate reviewer plus circuit breaker.** Run a second, cheaper model call (or a deterministic rule engine) outside the agent's writable area to vet outbound sends and deletes. Abort the lane after N consecutive denials. Because Codex's own docs say the reviewer is not a guarantee, keep hard sandbox limits underneath it.
4. **Read-only research lane.** Make background inbox ingestion and cron "research" use a tool set that is read-only in code, with no send, browser or write tools. Promote any action through the normal approval path. This is the cleanest answer to prompt injection arriving via inbox mail.
5. **Spend and activity budget per lane.** A background agent spent about 300k tokens a day unprompted. Add per-lane daily token and subagent caps, a kill switch, and a daily digest to the owner showing spawned sub-agents and tokens. OpenAI's Activity View hides sub-agents, so make yours show them.
6. **Memory you can inspect.** Dots' memory cannot be viewed, edited or selectively deleted. Keep agent memory as plain files on disk with provenance tags (source channel, trusted or untrusted), so the owner can audit and prune it.
7. **Payments by prepaid or virtual card.** Do not store a real card. Gate any purchase tool to a fixed-limit card plus mandatory owner confirmation over the chat channel.
8. **Credential isolation.** Keep secrets out of the model context and inject them at the tool layer, as the secure sign-in flow does. Warn that a secret pasted into chat will still be visible to the model.

## CLAIMS
- CLAIM: Dots' proactive-research tools are read-only (no sending, no plugin writes, no browser/computer control) and Custom Rules cannot override this — RISK: normal — SOURCES: help.openai.com — PRIMARY: https://help.openai.com/articles/20001529
- CLAIM: Money transfers and password changes must be handed back to the user; saved-card purchases need approval — RISK: high — SOURCES: help.openai.com — PRIMARY: https://help.openai.com/articles/20001529
- CLAIM: OpenAI admits prompt-injection defenses reduce but do not eliminate risk — RISK: high — SOURCES: help.openai.com, unite.ai — PRIMARY: https://help.openai.com/articles/20001529
- CLAIM: Individual dot memories cannot be viewed, edited or selectively deleted; only deleting the dot clears them — RISK: high — SOURCES: help.openai.com — PRIMARY: https://help.openai.com/articles/20001529
- CLAIM: A dot spawned sub-agents and used about 307k tokens in a day without user prompts (one vendor's test) — RISK: high — SOURCES: theregister.com — PRIMARY: none (Geordie's test data was not located)
- CLAIM: Pro (outside EEA/CH/UK) and Business Premium include a dot; Enterprise is admin-enabled and off by default — RISK: normal — SOURCES: help.openai.com — PRIMARY: https://help.openai.com/articles/20001530
- CLAIM: Dots work with Microsoft via Agent 365 for specialist dots — RISK: normal — SOURCES: unite.ai — PRIMARY: none (openai.com post was blocked)
- CLAIM: Pro 200 drops from 20x to 10x on 2026-10-30 and a Pro 500 tier exists — RISK: normal — SOURCES: news.ycombinator.com, theregister.com — PRIMARY: none (the OpenAI email is only quoted in an HN post)

## EXPAND
- LEAD: Read `openai.com/index/introducing-dots/` and `/how-we-build-safety-security-and-privacy-into-dots/` directly — WHY: those are the primary sources for the sandbox and "controls outside the dot's reach" claims, which I only have secondhand — ANGLE: fetch from a browser or non-blocked network.
- LEAD: GitHub issues openai/codex #51757, #50115, #50012, #51213, #51540 — WHY: real failure and abuse-refusal reports on dots — ANGLE: reliability and false-positive refusal rates.
- LEAD: Geordie's original report and objective_see's X post — WHY: possible concrete attack or visibility findings — ANGLE: security research.
- LEAD: Reddit r/OpenAI threads on dots — WHY: user sentiment, real-world incidents (the Reddit search call returned only page markup, so nothing was extracted) — ANGLE: incident reports.
- LEAD: Agent 365 and Microsoft dots integration — WHY: the Microsoft relationship is only one line in a secondary source — ANGLE: enterprise governance and audit logs.