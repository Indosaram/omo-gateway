<!-- task=st_01a144f1 -->
st_01a144f1 [completed] model mahoquot/claude-sonnet-5-5
Ask: research OpenAI Dots from the product angle - wanted: a sourced report on its UX, channels, proactivity and approvals, plus ideas for a self-hosted gateway. For you: the report is complete, but I couldn't read the official openai.com or help.openai.com pages (Cloudflare challenge). Official text below comes from HN quotes, and several seed claims are unverified. Now: done. Next: none.

## Identity

- **Name:** "Dots" (singular: "a dot"), OpenAI's always-on agents inside ChatGPT, powered by GPT-6 Astra. Each dot has its own cloud computer and learns from feedback [https://news.ycombinator.com/item?id=49901067].
- **Company:** OpenAI. Official URL: `https://openai.com/index/introducing-dots/`, titled "Dots: Always-on agents". It was submitted to HN on 2026-09-29 and drew 770 points and 647 comments [https://news.ycombinator.com/item?id=49896604].
- **Launch date:** 2026-09-29 by HN and press timestamps; CNA's photo is dated 2026-09-29 [https://myclaw.ai/blog/dots-ai]. I could not confirm it was announced at DevDay. The seed's "DevDay (Sep 2026)" is unverified.
- **GPT-6 Astra:** a real, separate model. OpenAI began rolling it out on 2026-09-03 [https://www.cnbc.com/2026/09/03/open-ai-astra-gpt-6-cyber.html].
- **Confidence:** high that the product exists and is named Dots, from OpenAI. Medium on detail, because I read the official page only through quotes. Low on "4000+ apps", "DevDay" and "brought to Microsoft" as phrased in the seed.
- **Name collisions:** unrelated products also called Dot or Dots exist, such as getdot.ai and the dots.mocr OCR model. Ignore them [https://myclaw.ai/blog/dots-ai].

## Findings

**Surface and onboarding**
- A dot is a persistent assistant with its own cloud computer and browser for research, files and running software. You give it a responsibility and keep steering it [https://myclaw.ai/blog/dots-ai].
- The first dot is included with eligible Pro and Business Premium plans, at no extra charge. Enterprise gets it only after an administrator enables it [https://myclaw.ai/blog/dots-ai].
- The launch text quoted on HN says Dots roll out "on web, mobile, and desktop" to Pro users outside the EEA, Switzerland and the UK [https://news.ycombinator.com/item?id=49897662].
- MyClaw's summary of the help center says you create a dot in the ChatGPT desktop app or desktop web. Creating on mobile and using mobile web are unsupported [https://myclaw.ai/blog/dots-ai]. The two sources conflict, and the help-center version is more specific and probably newer.
- Pricing tiers are Pro 100, 200 and a new 500. Pro 500 gives 25x Plus usage and "Ultrafast" [https://news.ycombinator.com/item?id=49901067]. Free, Go and Plus users get no dots [https://news.ycombinator.com/item?id=49896810].
- Conversation with a dot doesn't draw on ChatGPT limits. Work or Codex tasks it starts do count [https://news.ycombinator.com/item?id=49901067].

**Channels**
- The only documented ways to reach a dot are ChatGPT, Slack and Teams [https://myclaw.ai/blog/dots-ai]. I found no evidence of SMS, iMessage, voice, or email as a chat channel. This is unconfirmed rather than disproven.
- A channel is separate from app access. Adding a dot to Slack grants no inbox or file access; each app connection has its own permissions [https://myclaw.ai/blog/dots-ai].
- The 2-minute launch video is a Slack-based demo, and one commenter found the dot "slow" in it [https://news.ycombinator.com/item?id=49896834]. This is a commenter's opinion.
- Users asked on HN for iMessage- and Telegram-style channels, saying Telegram and Messages lack the fidelity for managing agent work. This is demand, not a Dots feature [https://news.ycombinator.com/item?id=49913574].

**Task assignment and reporting**
- A dot can hand work to background agents while you keep talking. An Activity view lets you inspect and stop delegated tasks [https://myclaw.ai/blog/dots-ai].
- It uses relevant ChatGPT memory plus its own saved notes on preferences, decisions and assignments. The notes are not a full transcript [https://myclaw.ai/blog/dots-ai].
- One commenter calls it "a work-oriented Muse": a single thread that manages work and delegates to other threads [https://news.ycombinator.com/item?id=49897731]. Another reads it as Codex, claw and ChatGPT Work folded together [https://news.ycombinator.com/item?id=49906561]. Both are interpretation.

**Proactivity**
- Dots supports saved schedules, monitoring of supported events, and follow-ups it decides are needed [https://myclaw.ai/blog/dots-ai].
- A scheduled brief needs a confirmed source, time, time zone and delivery destination. Event monitoring depends on the connected service and your instructions. Adding a dot to a Slack channel does not tell it what to watch [https://myclaw.ai/blog/dots-ai].
- "Proactive research" runs when you aren't working with it. The launch text says it uses connected apps through read-only tools that cannot send messages, change app content, or control your browser or computer [https://news.ycombinator.com/item?id=49897554].
- Any action from that research needs separate permissions. A dot can flag an overdue item without being allowed to change it [https://myclaw.ai/blog/dots-ai].
- Criticism: scheduled-task configuration is a limited dropdown, and external triggers are missing [https://news.ycombinator.com/item?id=49906561].

**Approval and control**
- Each dot is defined by instructions, connected apps, and Custom Rules in three lists: Allow, Require approval, Block. Those rules decide what it does without asking [https://news.ycombinator.com/item?id=49916870]. This is a third-party summary by the author of an unofficial directory.
- Hands-on report from a Pro user, who graded the feature "B-" [https://news.ycombinator.com/item?id=49904198]:
  - The permissions UI runs a rule checker that rejected broad rules, such as "proceed without asking on routine reversible actions". It also rejected a rule to reply as the user to Slack tags.
  - A dot asked for confirmation twice on one booking.
  - It will not read a 2FA code from the user's email and enter it, even when told to.
  - Verdict: not an OpenClaw-like agent.
- Controls: Pause stops the main task only. Activity stops delegated tasks. Scheduled disables or deletes recurring tasks. Pause does not undo completed actions [https://myclaw.ai/blog/dots-ai].
- Custom rules can be followed imperfectly, so pair them with app permissions [https://myclaw.ai/blog/dots-ai].
- Local-computer tasks need your machine online with the ChatGPT app open. Cloud tasks continue while it is off. Logins and verification steps can interrupt unattended runs [https://myclaw.ai/blog/dots-ai].

**Microsoft**
- The quoted launch text says OpenAI is "working with Microsoft to integrate specialist dots" with enterprise governance and security controls in Agent 365, so businesses can manage dots through Microsoft tools [https://news.ycombinator.com/item?id=49898108]. That is a stated goal, not a shipped product. "Brought to Microsoft" overstates it.

**Not found**
- "4000+ connected apps": no source found. I treat it as unverified.
- Voice and SMS channels: nothing found.

## Implications for a self-hosted personal agent gateway

1. **Three-list rules as data.** Implement per-agent Allow / Require approval / Block rules, keyed by action class and target. Gate on the class of action, not each call. Reads run freely, mutations ask, and a new outbound host or credential always asks. A commenter reports this avoids approve-everything fatigue [https://news.ycombinator.com/item?id=49904634]. In Discord, an approval is a button message with a timeout and a default-deny.
2. **Read-only proactive lane.** Add a lane whose tools are structurally read-only: inbox ingestion, repo and CI watchers. It posts findings but cannot send or mutate. A second, approval-gated lane executes. This mirrors Dots' "proactive research" split [https://news.ycombinator.com/item?id=49897554].
3. **External triggers.** Dots' weak spot is no external triggers and limited schedules [https://news.ycombinator.com/item?id=49906561]. Offer cron with real "repeats every", plus webhooks, a Maildir/inbox arrival trigger, and file or git events. Report each trigger's source in the message.
4. **Schedule contract.** For every recurring job store source, time, time zone, destination channel, and whether it is read-only. Echo it back for confirmation on creation, as Dots recommends [https://myclaw.ai/blog/dots-ai].
5. **Pause / Activity / Scheduled equivalents.** Expose three owner commands: pause the main thread, list and kill delegated omo runs, and list and disable cron jobs. State that pause doesn't undo completed actions, and log side effects so the owner can reverse them.
6. **Cheap rolling-session agent plus wakeups.** One commenter runs a rolling session with timer wakeups and lock-based mutual exclusion between wake sources. Items are marked foreground or background, and timers fire more often when a foreground item exists [https://news.ycombinator.com/item?id=49914353]. Your lanes can use the same model: per-lane lock, foreground/background priority, and an inbox that wakes the agent.
7. **Channel parity as the edge.** Dots offers only ChatGPT, Slack and Teams, with no iMessage, KakaoTalk or voice [https://myclaw.ai/blog/dots-ai]. Keep one agent identity across Discord, iMessage and KakaoTalk, with channel access separate from tool access, as Dots does.
8. **Credentials outside the sandbox.** Inject secrets at the proxy and restrict domains, URLs and methods per endpoint. Then a prompt-injected `curl` can't leak them [https://news.ycombinator.com/item?id=49905655]. Treat 2FA codes as an explicit, owner-granted capability, because Dots refuses them and users find that annoying [https://news.ycombinator.com/item?id=49904198].

## CLAIMS

- CLAIM: Dots are OpenAI's always-on ChatGPT agents powered by GPT-6 Astra, each with its own cloud computer, launched around 2026-09-29. — RISK: normal — SOURCES: news.ycombinator.com, myclaw.ai — PRIMARY: https://openai.com/index/introducing-dots/ (blocked; not read directly)
- CLAIM: Dots are reachable via ChatGPT, Slack and Teams only. — RISK: normal — SOURCES: myclaw.ai — PRIMARY: none
- CLAIM: Proactive research runs on read-only tools and cannot send messages or change anything. — RISK: normal — SOURCES: news.ycombinator.com (quoting OpenAI) — PRIMARY: https://openai.com/index/introducing-dots/
- CLAIM: Custom Rules have three lists: Allow, Require approval, Block. — RISK: normal — SOURCES: news.ycombinator.com — PRIMARY: none
- CLAIM: The permissions UI rejects broad auto-approve rules. — RISK: normal — SOURCES: news.ycombinator.com (single user report) — PRIMARY: none
- CLAIM: Microsoft integration is a stated goal for Agent 365 governance, not a shipped feature. — RISK: high — SOURCES: news.ycombinator.com (quoting OpenAI) — PRIMARY: https://openai.com/index/introducing-dots/
- CLAIM: Mobile support differs by source: the launch text says "web, mobile, and desktop", while the help-center summary says desktop-only creation. — RISK: normal — SOURCES: news.ycombinator.com, myclaw.ai — PRIMARY: none
- CLAIM: The "4000+ connected apps" and "unveiled at DevDay" claims are unverified. — RISK: high — SOURCES: none found — PRIMARY: none

## EXPAND

- LEAD: Official help-center article for Dots (permissions, scheduling, channels). — WHY: it is the only source that settles channels, triggers, mobile support and approval UI. I was blocked by Cloudflare. — ANGLE: fetch it from a normal browser or a cached copy.
- LEAD: The 2-minute launch video, https://www.youtube.com/watch?v=uXspbC2srEQ. — WHY: it shows the UI, approval prompts and the Slack flow. — ANGLE: product, transcript.
- LEAD: Microsoft Agent 365 documentation for dots. — WHY: it would confirm or refute "brought to Microsoft". — ANGLE: enterprise governance.
- LEAD: Unite.AI and CNA launch coverage. — WHY: they may state the app count and connectors. — ANGLE: connectors and integrations. Unite.AI exceeded my output buffer.
- LEAD: Third-party directories and reviews (dotbotdirectory.com, amplifying.ai "What Dots Choose"). — WHY: real rule sets and behavior observations. — ANGLE: user experience and failure modes.