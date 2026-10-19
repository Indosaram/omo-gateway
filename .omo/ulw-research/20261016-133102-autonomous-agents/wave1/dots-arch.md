<!-- task=st_01a144f2 -->
st_01a144f2 [completed] model mahoquot/claude-sonnet-5-5
Ask: research OpenAI Dots' architecture and report. Wanted: a sourced markdown report. For you: the primary pages are blocked, so this report rests on secondary sources and HN quotes of the launch post. Now: done. Next: none.

**Verification gap:** `openai.com/index/introducing-dots/`, `/index/dots/` and the help-center article all returned a Cloudflare challenge page to curl. I could not read OpenAI's text directly. Several bullets below are quotes of the launch post made by HN commenters. I treat those as secondary.

## Identity
- **Name:** "dots" (lowercase in places), always-on agents inside ChatGPT. Each individual agent is "a dot".
- **Company:** OpenAI.
- **Official URL:** https://openai.com/index/introducing-dots/. This is the URL of the HN submission "Dots: Always-on agents".
- **Launch date:** the HN thread was posted 2026-09-29 [https://hn.algolia.com/api/v1/items/49896604]. MyClaw's article is dated 2026-09-30 [https://myclaw.ai/blog/dots-ai].
- **"DevDay" framing:** I found no source tying the launch to DevDay. Treat that as unverified.
- **Confidence:** high that the product is real and the name is correct. Medium on architecture details, which are second-hand. Low on the "4000+ apps" figure.
- **Disambiguation:** MyClaw notes it is unrelated to other products called Dot or Dots [https://myclaw.ai/blog/dots-ai]. An HN post about the "Dot" data bot at getdot.ai is a different product [https://hn.algolia.com/api/v1/search?query=OpenAI%20Dots%20agents&tags=story].

## Findings

**Execution environment**
- Each dot has its own cloud computer and browser. It can browse, analyze, create files and run tools, and it keeps making progress when you are not engaged. This is quoted from the launch post by an HN commenter [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- Sandboxing restricts which code and tools the dot can reach. Users' cloud environments are isolated from one another, and OpenAI maintains the underlying Linux OS and Chrome. Quoted second-hand [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- An "Auto-review" step checks actions that need review before they run. Also quoted second-hand [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- OpenAI's own summary: powered by GPT-6 Astra, own cloud computer, learns from feedback, works toward goals 24/7. This comes from the OpenAI email to Pro users, posted verbatim on HN [https://hn.algolia.com/api/v1/search?query=OpenAI%20Dots%20agents&tags=story].
- Local device access is opt-in. You choose which apps to connect and whether to connect your personal computer [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- Local tasks need your computer online with the ChatGPT app open. Cloud tasks keep running while your machine is off [https://myclaw.ai/blog/dots-ai].
- One HN commenter inferred that the local path goes through the ChatGPT/Codex app on your computer. That is inference, not confirmed [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- Where to use it: create a dot in the ChatGPT desktop app or desktop web. Mobile creation and mobile web are unsupported [https://myclaw.ai/blog/dots-ai].

**Connectors and auth**
- Each app connection carries its own permissions. Connecting Slack or Teams as a chat channel does not grant inbox, file or computer access [https://myclaw.ai/blog/dots-ai].
- Channels to reach the same dot are ChatGPT, Slack and Teams [https://myclaw.ai/blog/dots-ai].
- Proactive research runs in the background when you are idle, using connected apps through read-only tools. Those tools cannot send messages, change app content, or control your browser or computer. Quoted from the post [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- Password handling, from one hands-on user: the dot can log into many sites with your password. When a login triggers an emailed 2FA code, it refuses to fetch and enter the code even with email access, and hands it back to you. This is a single anecdote, but it shows a deliberate human-in-the-loop boundary [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- I found no description of the credential vault itself. One HN commenter described the general pattern of injecting secrets outside the sandbox. That is the commenter's reasoning about the design space, not a Dots claim [https://hn.algolia.com/api/v1/search?query=dots%20credentials%20OR%20vault%20OR%20password%20OR%20sandbox&tags=comment,story_49896604].
- Website logins, verification steps and cloud-browser restrictions can interrupt an unattended run [https://myclaw.ai/blog/dots-ai].
- **"4000+ connected apps": I could not corroborate this.** Nothing I read states the number [none].

**Permission and rule model**
- Each dot comes down to three things: instructions, reachable apps, and Custom Rules with three buckets (Allow / Require approval / Block). This is an HN builder's summary, not OpenAI's wording [https://hn.algolia.com/api/v1/search?query=OpenAI%20Dots%20agents&tags=story].
- A rule validator runs in the permissions UI and rejects rules it judges too broad. Rejected examples included blanket "proceed without asking for low-risk reversible actions" and "reply as me to Slack tags". The user got only 2 rules accepted. Single anecdote [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- The same user reported repeated confirmation loops. The dot filled a booking form, then asked again whether to book [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- Custom rules can be followed imperfectly, so MyClaw advises pairing them with app-level permissions [https://myclaw.ai/blog/dots-ai].

**Memory and personal context**
- A dot uses relevant ChatGPT memory plus its own saved notes on preferences, decisions and assignments. The notes are not a full transcript [https://myclaw.ai/blog/dots-ai].
- OpenAI's email says dots "learn from feedback over time" [https://hn.algolia.com/api/v1/search?query=OpenAI%20Dots%20agents&tags=story].

**Long-running and continuation model**
- Dots support saved schedules, monitoring of supported events, and follow-ups the dot decides are needed [https://myclaw.ai/blog/dots-ai].
- A schedule needs source, time, time zone and destination spelled out. Event monitoring depends on the connected service and your instructions [https://myclaw.ai/blog/dots-ai].
- Controls: Pause stops the main task only. Activity lets you inspect and stop delegated tasks. Scheduled lets you disable or delete recurring tasks. Pause does not undo completed actions [https://myclaw.ai/blog/dots-ai].

**Multi-agent structure**
- A dot can assign work to background agents while you keep talking to it, and an Activity view shows delegated tasks and results [https://myclaw.ai/blog/dots-ai].
- Billing implies delegation goes to Codex and ChatGPT Work. Conversations with the dot are free of usage limits, but tasks it starts or manages in Codex or ChatGPT Work count against those limits [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
- One commenter described it as a work-oriented Muse: one thread that manages and delegates to other threads. This is opinion, and Muse is Meta's agent [https://hn.algolia.com/api/v1/items/49896604].

**Models**
- GPT-6 Astra powers dots [https://hn.algolia.com/api/v1/search?query=OpenAI%20Dots%20agents&tags=story].
- The same email mentions GPT-6.1 Sol as a near-Astra model for everyday use [https://hn.algolia.com/api/v1/search?query=OpenAI%20Dots%20agents&tags=story].
- A Luna-based "Decisions API" is in limited preview, for classifying content, routing requests and choosing an agent's next action. The launch coverage mentions it next to Dots, but it is not stated to be part of Dots [https://hn.algolia.com/api/v1/items/49896604].

**Access and pricing**
- Included with Pro 100/200/500 for users over 18 outside the EEA, UK and Switzerland. Business Premium is worldwide. Enterprise is worldwide but off by default until an admin enables it [https://myclaw.ai/blog/dots-ai].
- Pro 500 is a new tier. Pro 200 usage is cut after Oct 29 [https://hn.algolia.com/api/v1/search?query=OpenAI%20Dots%20agents&tags=story].
- Plus, Free and Go are not eligible [https://hn.algolia.com/api/v1/items/49896604].

**Microsoft**
- OpenAI says it is working with Microsoft to integrate specialist dots with enterprise governance and security controls in Agent 365, so businesses can manage dots through tools they already use. Quoted from the post by an HN commenter [https://hn.algolia.com/api/v1/items/49896604].

**Developer APIs and SDKs**
- I found no public Dots API, SDK or webhook interface in anything I could read. The HN search for API/SDK in the thread returned zero hits [https://hn.algolia.com/api/v1/search?query=API%20SDK&tags=comment,story_49896604]. This is a coverage gap, not proof of absence.

**Community and third-party stack**
- Third-party dot-rule directories exist, e.g. DotBotDirectory [https://dotbotdirectory.com].
- An open-source rival, "open-instinct", uses Pi, Maritime VMs, Composio for managed OAuth, and Inkbox for iMessage and email [https://github.com/mariagorskikh/open-instinct].
- Reception: HN commenters called it "hosted OpenClaw" [https://hn.algolia.com/api/v1/items/49896604]. Pricing and EU exclusion drew most of the complaints.

## Implications for a self-hosted personal agent gateway
1. **Three-bucket rule engine per lane or connector.** Add Allow / Require-approval / Block lists, evaluated before any tool call and approved over Discord, iMessage or KakaoTalk. Make them machine-enforced, not prompt-only, because Dots' own rules are followed imperfectly [https://myclaw.ai/blog/dots-ai].
2. **Read-only "proactive research" mode.** Scheduled cron lanes would get a tool allowlist that excludes send and write operations. Run inbox-ingestion digests in this mode and queue any write for approval [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
3. **Human-only 2FA and password boundary.** Have the daemon detect login or verification walls, pause the task, ping the owner on chat, and resume after the code is supplied. Dots deliberately does this, and it is cheap to copy [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].
4. **Credential injection outside the agent's sandbox.** Keep secrets in the Rust daemon or the macOS Keychain and inject them at an HTTP proxy or tool boundary. Allowlist domains and methods per scenario, and add a "learning mode" that records the calls a task makes so you can then lock the policy [https://hn.algolia.com/api/v1/search?query=dots%20credentials%20OR%20vault%20OR%20password%20OR%20sandbox&tags=comment,story_49896604]. This is a design idea from the thread, not a Dots feature.
5. **Activity and Scheduled views with per-task stop.** Add `/activity`, `/pause`, `/cancel <task>` and `/schedules` commands in chat. Pause should stop the main lane only, with separate controls for delegated omo runs, matching Dots' split controls [https://myclaw.ai/blog/dots-ai].
6. **Schedules with explicit contract fields.** Require source, time, time zone and destination when a cron job is created from chat, to avoid ambiguous recurring work [https://myclaw.ai/blog/dots-ai].
7. **Curated notes memory, not transcripts.** Keep a small notes file per lane covering preferences, decisions and assignments, separate from raw logs. This mirrors "notes, not a complete transcript" [https://myclaw.ai/blog/dots-ai].
8. **Delegation with budget accounting.** Treat omo coding agents as delegated workers. The parent lane is cheap or unmetered, and spawned runs count against a per-lane budget, as Dots charges Codex and Work usage [https://hn.algolia.com/api/v1/search?query=apps%20connected&tags=comment,story_49896604].

## CLAIMS
- CLAIM: Dots are always-on ChatGPT agents powered by GPT-6 Astra, each with its own cloud computer. — RISK: normal — SOURCES: hn.algolia.com (quoting OpenAI's email), myclaw.ai — PRIMARY: none readable (openai.com blocked by Cloudflare)
- CLAIM: Dots run in sandboxed Linux plus Chrome workspaces, isolated per user. — RISK: normal — SOURCES: hn.algolia.com (quoting the launch post) — PRIMARY: https://openai.com/index/introducing-dots/ (not fetched)
- CLAIM: Proactive research is limited to read-only tools. — RISK: normal — SOURCES: hn.algolia.com (quoting the launch post) — PRIMARY: https://openai.com/index/introducing-dots/ (not fetched)
- CLAIM: Dots refuses to enter emailed 2FA codes itself. — RISK: high (single anecdote) — SOURCES: hn.algolia.com — PRIMARY: none
- CLAIM: A rule-validator rejects broad custom rules in the permissions UI. — RISK: high (single anecdote) — SOURCES: hn.algolia.com — PRIMARY: none
- CLAIM: Dots is being integrated with Microsoft Agent 365 governance. — RISK: normal — SOURCES: hn.algolia.com (quote of the post) — PRIMARY: https://openai.com/index/introducing-dots/ (not fetched)
- CLAIM: Dots connects to 4000+ apps. — RISK: high (unverified) — SOURCES: none found — PRIMARY: none
- CLAIM: Dots was unveiled at OpenAI DevDay. — RISK: high (unverified) — SOURCES: none found — PRIMARY: none
- CLAIM: There is no public developer API or SDK for Dots. — RISK: high (absence of evidence) — SOURCES: hn.algolia.com thread search only — PRIMARY: none

## EXPAND
- LEAD: Read the OpenAI help-center article on dots and the launch post text. — WHY: it is the only source for connector count, auth and credential handling, memory details, and the exact Auto-review and permission semantics. I was blocked by Cloudflare. — ANGLE: official docs, via a browser-capable fetch.
- LEAD: Check the OpenAI developer docs, changelog and the Agent 365 and Microsoft announcement for a Dots API, MCP or connector SDK. — WHY: no developer surface was found. — ANGLE: devs/api.
- LEAD: Read the Substack "agent swarm" post and the Amplifying.ai "What Dots Choose" study (https://amplifying.ai/research/openai-dots-choose). — WHY: they may reveal tool and stack behavior, i.e. what the harness actually does. — ANGLE: empirical behavior.
- LEAD: Compare with Meta Muse and open-source OpenClaw-style harnesses. — WHY: HN framed Dots as "hosted OpenClaw", so the architectural overlap is relevant to a self-hosted design. — ANGLE: competitor architecture.