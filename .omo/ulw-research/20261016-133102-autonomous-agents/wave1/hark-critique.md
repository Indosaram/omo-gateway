<!-- task=st_01a14500 -->
st_01a14500 [completed] model mahoquot/claude-sonnet-5-5
## Identity

Handoff / Hark Pro is a consumer personal-AI agent from Hark. Hark is the AI lab founded by Brett Adcock, who also runs Figure AI. Confidence is high on identity and low on reception, because almost no independent evidence exists yet.

- **Company and funding:** TNW says Hark raised $700M at a $6B valuation in May, two months after leaving stealth. Nvidia, AMD Ventures, Intel Capital and Qualcomm Ventures were in the round. Adcock put in $100M first. [https://thenextweb.com/news/hark-pro-agent-hardware-europe]
- **Official site:** hark.com. The Handoff research preview is at hark.com/articles/introducing-hark-handoff. The Hark Pro launch post is at hark.com/articles/introducing-hark-pro.
- **Handoff preview:** announced about 5–6 Aug 2026, with a waitlist. Hacker News (HN) shows a submission on 2026-08-05. Rocking Robots ran a story on 2026-08-06. [https://hark.com/articles/introducing-hark-handoff] [https://www.rockingrobots.com/hark-unveils-handoff-an-ai-agent-that-completes-tasks-directly-in-a-browser/]
- **Hark Pro launch:** 6 Oct 2026, on web, iOS and Android. The tiers are free, $20 (2x usage) and $100 (10x usage). [https://hark.com/articles/introducing-hark-pro]
- **Naming:** Handoff is the computer-use model and cloud agent inside Hark Pro. Hark Pro is the app.
- **Not to confuse with:**
  - The old hark.com, a sound-clip site. HN comments from 2011–2017 link to hark.com/clips. [https://hn.algolia.com/api/v1/search?query=%22hark.com%22&tags=comment]
  - The "Hark decision intelligence" listing. I did not find or verify it, so I can only say it is not this product.

## Findings

**Evidence gap (the main finding)**
- I found no hands-on reviews, Reddit threads, user complaints or incident reports for Hark Pro or Handoff. The product was about 10 days old on 2026-10-16. This is an absence in what I could reach, not proof that none exist. [https://hn.algolia.com/api/v1/search?query=hark.com]
- HN has two Hark submissions: "Hark Handoff" (5 points, 0 comments) and "Hark Pro" (3 points, 0 comments). Neither has any discussion. [https://hn.algolia.com/api/v1/items/49980710]
- Search coverage was thin. The `web_search` tool was not available. Google, DuckDuckGo and Bing returned a challenge page or a localized results page I could not read. Reddit returned a login page. Only HN Algolia, TNW and Rocking Robots were readable.

**What press and the company say**
- TNW's launch piece is a news summary, not a hands-on test. It lists these capabilities:
  - Buys groceries, books cars and pays bills.
  - Refills a child's lunch card.
  - Infers from email that a flight is booked but a hotel is not.
  - Runs up to six browsers at once.
  - Stores card details and passwords in an encrypted vault that Hark says even it cannot see into.
  [https://thenextweb.com/news/hark-pro-agent-hardware-europe]
- TNW had to correct its own article. It first said Handoff runs 36 browsers, then fixed it to six after Hark said so. Treat early coverage as thin and error-prone. [https://thenextweb.com/news/hark-pro-agent-hardware-europe]
- Rocking Robots says Hark's claims that Handoff is faster and cheaper than agents built on GPT-5.5 and Claude Opus 4.8 "have not yet been independently verified". [https://www.rockingrobots.com/hark-unveils-handoff-an-ai-agent-that-completes-tasks-directly-in-a-browser/]
- Hark's own wording differs from that. Its Pro post calls Handoff "independently verified", and its Handoff post says it tops the Online-Mind2Web human-evaluation leaderboard. [https://hark.com/articles/introducing-hark-pro] Those leaderboard numbers come from the official leaderboard except entries Hark marks as self-reported by OpenAI or reported by Browserbase. [https://hark.com/articles/introducing-hark-handoff]
- The two sources also name different comparison models. Rocking Robots says GPT-5.5. Hark's post says GPT 5.4 and Opus 4.8. Hark's own post is the more reliable source here. [https://hark.com/articles/introducing-hark-handoff]

**Failure modes, from Hark's own text and context**
- Hark admits the web is hostile to agents: "Bot blocking, pop-ups, banners, and ads exist specifically to stop, slow, or limit automated agents." It also says "every session introduces new friction". [https://hark.com/articles/introducing-hark-handoff]
- Latency is real. Hark says browser latency adds roughly 10 seconds per step on top of model latency. [https://hark.com/articles/introducing-hark-handoff] A multi-step purchase would therefore take minutes (my inference from the per-step figure).
- Handoff acts by emitting cursor and keyboard inputs at x/y screen coordinates, not DOM or API calls. [https://hark.com/articles/introducing-hark-handoff] Layout changes can break this (inference).
- I could not confirm that any site blocks Handoff. I found no evidence that Amazon blocks it. Amazon is not on Hark's demo list, which names DoorDash, Target, Walmart, Costco, OpenTable, Resy, LinkedIn, the airlines and Booking.com. [https://hark.com/articles/introducing-hark-handoff]
- Context from the competing Meta Muse launch: Axios reported that a Meta researcher's own agent experiment ended with files deleted, and that booking platforms including Resy told customers not to point automated agents at them. [https://thenextweb.com/news/meta-muse-personal-ai-agent-launch] Resy is also one of Hark's advertised demo sites. This is a sector-level risk, not a Hark incident.
- Hark says it "checks in with you before doing something that requires your approval". [https://hark.com/) It also says that if you allow it, Hark can top off a lunch card on its own. [https://hark.com/articles/introducing-hark-pro] The exact approval policy is undocumented, so unattended spending is a risk to verify.

**Competition and regulatory gaps**
- TNW says Meta's Muse launched four weeks earlier at the same free/$20/$100 ladder. Muse uses an isolated VM per user, a "Sentinel" proxy with time-limited read/write permissions, an audit trail, and Stripe Link for payments. [https://thenextweb.com/news/meta-muse-personal-ai-agent-launch] Hark's published security story is only the encrypted vault and a promise to ask first. No audit trail or permission-scoping detail was visible to me.
- Hark has not said whether Hark Pro is available in Europe. TNW notes that software approving a payment has no carve-out from the EU's strong customer authentication rules. [https://thenextweb.com/news/hark-pro-agent-hardware-europe]
- Hardware is due in 2027. Nobody outside Hark has seen it. [https://thenextweb.com/news/hark-pro-agent-hardware-europe]

## Implications for a self-hosted personal agent gateway

These are inferences from the evidence above, not things reviewers reported.

1. **Per-action approval ledger.** Send spend or irreversible actions (pay, cancel, send) to the owner on Discord/iMessage/KakaoTalk as a one-tap approve/deny with the amount, payee and a screenshot. Default to ask-first. This is the gap in Hark's published policy and the point of Meta's Sentinel.
2. **Scoped, expiring permissions.** Give each lane or job a time-limited capability, such as read-only for inbox ingestion and a spend cap for a purchase lane. Copy Muse's read/write split.
3. **Immutable audit log.** Record every browser step with timestamp, URL and screenshot. This gives you something to replay when a site blocks the agent or a wrong purchase happens.
4. **Per-site health and block detection.** Detect CAPTCHA, bot-wall and login-challenge pages, mark that site "needs human", and hand the owner a live-session link instead of retrying blindly. Hark itself calls bot-blocking the main enemy.
5. **Prefer APIs and MCP before pixels.** Use email, calendar and other services directly where possible, and use the browser only as a fallback. Hark does this too for Google and Outlook.
6. **Step budget and wall-clock timeouts per cron job.** With ~10s of browser latency per step, long runs need hard caps and a "still working" heartbeat to the owner's chat.
7. **Proactive suggestions as a separate lane.** Hark's "Action Buttons" and lunch-card top-up are suggestions the user taps. Gate any autonomous top-up behind an explicit per-rule opt-in.
8. **Sandbox mutation-capable agents.** Run omo coding agents and browser agents in isolated worktrees or VMs, given the file-deletion anecdote in the Muse coverage.

## CLAIMS
- CLAIM: Hark Pro launched 6 Oct 2026 with free/$20/$100 tiers on web, iOS and Android. — RISK: normal — SOURCES: hark.com, thenextweb.com — PRIMARY: https://hark.com/articles/introducing-hark-pro
- CLAIM: No independent hands-on reviews or user-complaint threads for Hark exist yet in the sources I could reach. — RISK: high — SOURCES: hn.algolia.com (HN submissions have 0 comments); Reddit and web search were unreachable — PRIMARY: none
- CLAIM: Hark says Handoff leads the Online-Mind2Web leaderboard, but some comparison figures are self-reported by OpenAI or reported by Browserbase, and Rocking Robots says the speed and cost claims are unverified. — RISK: high — SOURCES: hark.com, rockingrobots.com — PRIMARY: https://hark.com/articles/introducing-hark-handoff
- CLAIM: Hark says browser latency adds roughly 10 seconds per step, and describes bot-blocking and pop-ups as a core obstacle. — RISK: normal — SOURCES: hark.com — PRIMARY: https://hark.com/articles/introducing-hark-handoff
- CLAIM: Handoff acts via coordinate-based cursor and keyboard inputs in a dedicated virtual computer. — RISK: normal — SOURCES: hark.com — PRIMARY: https://hark.com/articles/introducing-hark-handoff
- CLAIM: Hark has not stated whether Hark Pro is available in Europe. — RISK: normal — SOURCES: thenextweb.com — PRIMARY: https://thenextweb.com/news/hark-pro-agent-hardware-europe
- CLAIM: Meta Muse is a same-price competitor with a documented permission-scoping layer, and Resy has told customers not to point agents at it. — RISK: normal — SOURCES: thenextweb.com citing Axios — PRIMARY: none (the Axios article was not fetched directly)
- CLAIM: Hark raised $700M at $6B in May 2026. — RISK: normal — SOURCES: thenextweb.com (the linked May article was not fetched) — PRIMARY: none

## EXPAND
- LEAD: Fetch the Bloomberg launch article (bloomberg.com/news/articles/2026-10-06/ai-startup-hark-debuts-proactive-hark-pro-app-ahead-of-2027-hardware-launch) and the Axios Muse article — WHY: they are the only other primary press, and may carry analyst or hands-on remarks — ANGLE: critique/press
- LEAD: Re-run Reddit (r/singularity, r/LocalLLaMA, r/artificial, r/iOSBeta) and X searches for "Hark Pro" in 2–4 weeks, using a client with API access — WHY: real user complaints should appear after the first few weeks of use, and Reddit blocked my fetches — ANGLE: user complaints
- LEAD: Check the App Store and Play Store reviews, and Hark's help centre or terms for the approval policy and spend limits — WHY: the best source for reliability complaints and the autonomous-spend policy, and I could not reach them — ANGLE: failure modes
- LEAD: Verify the "Hark decision intelligence" listing separately — WHY: I could not locate it, so the disambiguation rests only on domain history and launch context — ANGLE: identity