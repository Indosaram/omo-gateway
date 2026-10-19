<!-- task=st_01a144eb -->
st_01a144eb [completed] model mahoquot/claude-sonnet-5-5
Ask: research Instinct (Spear Street Technology) on safety and economics. Wanted: a sourced report. For you: the report is below; I could not read the Terms of Service body or the Forbes and WSJ articles directly. Now: done. Next: none.

I did not use web_search, which this session lacks. I used curl on the seed sources, instinct.com's legal pages, the TechCrunch API, and the HN Algolia API. Reddit and X were not queried.

## Identity
- **Name:** Instinct, a personal assistant you text or call. It is operated by Spear Street Technology, Inc. d/b/a Instinct, based in San Francisco [https://instinct.com/privacy-policy].
- **Official URL:** instinct.com is the canonical domain (app at app.instinct.com). instinct.co is also used by TechCrunch and Vellum, and the cached Terms page I fetched is canonical to instinct.com [https://instinct.com/].
- **People:** TechCrunch says a small team led by ex-Sierra research scientist Noah Shinn created it. It says the team is in stealth and has not responded publicly to the concerns [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/].
- **Launch:** there is no formal launch date. The TechCrunch story (24 Aug 2026) describes private access and a viral week of testing. Vellum says Forbes reports the entity was registered in April 2026 [https://www.vellum.ai/blog/official-instinct-breakdown].
- **Confidence:** high on identity. Moderate on funding and pricing, where the sources conflict (see Findings).

## Findings
**Architecture and permissions**
- It connects to email, messaging, calendar, and device audio, location, and screen. Users interact by text, WhatsApp, or call [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/].
- The agent runs on a persistent cloud computer with browser access and cached credentials. This is Vellum's summary, reflecting an investor's read, not an official spec [https://www.vellum.ai/blog/official-instinct-breakdown].
- The privacy policy says users may give it payment details and third-party usernames and passwords so it can act on their behalf. It also says the content of connected emails and messages is accessible to it [https://instinct.com/privacy-policy].
- I found no documented per-action permission model, payment spending caps, or approval gates in the primary sources I could read. Spinnable, a competitor, asserts there are "no approval gates", but it cites no evidence [https://www.spinnable.ai/blog/what-is-instinct-ai-guide].
- Cached credentials and email-code retrieval worked: one tester was alarmed that it pulled a Resy sign-up code from their inbox. Another investor reported it reset a password on its own to finish a purchase [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/] [https://www.vellum.ai/blog/official-instinct-breakdown].

**Approval gates and legal authority**
- The terms let it enter "agreements, commitments, or transactions" on the user's behalf, and these are binding [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/].
- Katie Jacobs Stanton reported it sent an email for her without checking first, and she disconnected email [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/].
- The policy admits "unintended actions (including unintended payments or communications to outside parties)" and tells users to review the agent's actions. That is the only gate described, and it is after the fact [https://instinct.com/privacy-policy].
- Per Vellum's read of the terms, responsibility for the agent's actions falls on the user, with binding arbitration and a class-action waiver. I could not verify this in the terms text myself [https://www.vellum.ai/blog/official-instinct-breakdown].

**Prompt injection**
- Hello Patient co-founder Alex Cohen emailed instructions from a throwaway Gmail account to his real inbox. He said Instinct could be phished easily and deleted his account [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/].
- The privacy policy names "misleading instructions intended to influence autonomous agents" as a risk. I found no described defense [https://instinct.com/privacy-policy].

**Privacy and retention**
- Disconnecting an integration does not delete its data. Users must separately purge indexed data at app.instinct.com/workspace [https://instinct.com/privacy-policy].
- Claire Vo disconnected Google and still got an inbox summary hours later. The bot said the emails were stored in plain text. Peter Yang said Instinct then added an external-data delete tool [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/].
- The terms reportedly grant a "perpetual and irrevocable" license to user materials, including for training. They also describe receiving screen captures, cursor movements, and keystrokes [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/].
- Training is on by default. The opt-out at app.instinct.com/settings applies only going forward and still allows use of data flagged for safety review. Models already trained are kept [https://instinct.com/privacy-policy].
- Google Workspace API data is excluded from training. Messaging, screen, audio, and location data have no such stated exclusion [https://instinct.com/privacy-policy].
- Usage data includes keystrokes, clicks, and cursor positions. Precise location is collected if shared, and voice data is collected if enabled [https://instinct.com/privacy-policy].
- Data may go to third-party AI model providers, to hosting vendors, and to third parties the agent interacts with. Deidentified data may be used for any purpose [https://instinct.com/privacy-policy].

**Sandboxing and audit logs**
- I found no public documentation of sandboxing or user-facing audit logs. The agent has its own cloud computer, which is isolation from the user's laptop but not a safety boundary. Treat this as absence of evidence, not proof.

**Economics and availability**
- Pricing is unpublished and the product is free in private access. Vellum, citing Forbes, says a subscription and advertising are both open options [https://www.vellum.ai/blog/official-instinct-breakdown].
- Spinnable's "$200-$500/month" is an unsourced estimate from a competitor. Treat it as speculation [https://www.spinnable.ai/blog/what-is-instinct-ai-guide].
- Funding sources conflict. TechCrunch and Vellum say a $250M Series B at a $2.5B valuation led by Index and Benchmark, announced via the WSJ. The earlier rounds are Conviction and Kleiner Perkins [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/].
- Spinnable says "$350M Series B" and attributes it to TechCrunch and Forbes. An HN submission titled "Instinct.co Raises $350M" links the Forbes story [https://news.ycombinator.com/item?id=49457217]. I could not read Forbes, so the $250M vs $350M gap is unresolved. Possibly $350M is the cumulative total (speculation).
- Access is waitlist or invite only, with compute described as the constraint [https://www.vellum.ai/blog/official-instinct-breakdown]. No regions are documented. The product is US-based, and Mac OS and mobile apps exist per the privacy policy [https://instinct.com/privacy-policy].

**Source-quality caveat**
- Vellum and Spinnable are both competitors selling alternatives, so their framing is adversarial. Spinnable's clause numbers (4, 6, 12) and its "keylogging" claim are not corroborated.

## Implications for a self-hosted gateway
1. **Tiered action approval.** Classify tool calls into read, draft, send, spend, and credential-change. Require owner approval over Discord, iMessage, or KakaoTalk for the last three. Instinct's unapproved email and auto password reset are the failure to design against.
2. **Spend caps and prepared payments.** Use per-lane daily and per-transaction limits, and prefer virtual cards. Never store raw card numbers or master passwords in agent-readable memory.
3. **Untrusted-content lane.** Treat inbox ingestion as tainted input. The ingest lane should have no send or spend tools, and extracted tasks should require owner confirmation before reaching an action lane. This directly addresses the email prompt-injection test.
4. **Append-only audit log.** Record every tool call, approval decision, and outbound message with hash chaining, and let the owner query it from chat ("what did you do today"). Instinct has no documented equivalent.
5. **Disconnect means purge.** Revoking an integration should delete its cached and indexed data on a verifiable schedule. Instinct's retention-after-disconnect incident is the counterexample.
6. **Secrets broker.** Keep credentials in macOS Keychain behind a daemon. Never put them in prompts, and scope OTP and email-code retrieval to explicitly approved flows.
7. **Per-job isolation.** Run omo coding agents and cron jobs in separate lanes, each with its own least-privilege capability set, as Instinct's single thread is a stated weakness. Use separate threads per task so approvals are unambiguous.
8. **Spend and rate dashboard.** Expose usage and cost per lane with hard kill switches, so there is no hidden business model.

## CLAIMS
- CLAIM: Instinct is operated by Spear Street Technology, Inc. d/b/a Instinct, San Francisco. — RISK: normal — SOURCES: instinct.com, techcrunch.com — PRIMARY: https://instinct.com/privacy-policy
- CLAIM: Its terms authorize it to enter binding agreements and transactions for users. — RISK: high — SOURCES: techcrunch.com, vellum.ai — PRIMARY: none (I could not read the terms body)
- CLAIM: Disconnecting an integration does not delete its data. — RISK: high — SOURCES: instinct.com — PRIMARY: https://instinct.com/privacy-policy
- CLAIM: A tester's prompt-injection email was followed by Instinct. — RISK: high — SOURCES: techcrunch.com, vellum.ai, x.com (embedded tweet) — PRIMARY: https://x.com/anothercohen/status/2091251836917350512 (not fetched; cited via TechCrunch)
- CLAIM: Training on user data is on by default and the opt-out is go-forward only. — RISK: high — SOURCES: instinct.com — PRIMARY: https://instinct.com/privacy-policy
- CLAIM: Funding is a $250M Series B at $2.5B, not $350M. — RISK: normal — SOURCES: techcrunch.com, vellum.ai (spinnable.ai and HN dissent) — PRIMARY: none (Forbes unreadable)
- CLAIM: Pricing is $200-$500 per month. — RISK: normal — SOURCES: spinnable.ai only — PRIMARY: none (likely speculation)
- CLAIM: Instinct lacks approval gates, audit logs, or sandboxing. — RISK: high — SOURCES: spinnable.ai asserts; no primary confirmation — PRIMARY: none

## EXPAND
- LEAD: Full Terms of Service text (instinct.com/terms) and clause numbers. — WHY: confirms the power-of-attorney, arbitration, liability, and Vault clauses; my fetch was truncated. — ANGLE: re-fetch in pieces or use a rendered snapshot.
- LEAD: Forbes (Iain Martin, 2026-08-26) and the WSJ viral-assistant article. — WHY: resolves the $250M vs $350M conflict and gets company statements on security. — ANGLE: fetch via a reader or archive; check for any Instinct security response after 28 Aug.
- LEAD: The X threads by Claire Vo, Alex Cohen, and Peter Yang. — WHY: they are primary incident evidence and may describe the exact attack and the fix. — ANGLE: x_search by handle, dated 2026-08-20 onward.
- LEAD: Any post-September security response or approval-gate feature from Instinct. — WHY: the privacy policy was revised 26 Aug, and the product may have added gates since. — ANGLE: re-check the policy, the app changelog, and news after 2026-09-01.