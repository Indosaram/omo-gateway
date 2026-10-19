<!-- task=st_01a14520 -->
st_01a14520 [completed] model mahoquot/claude-sonnet-5-5
Ask: Research Hark (Hark Pro, Handoff, Secured by Hark) for agent architecture, approvals, spend limits, credential isolation, pricing, hardware and PSD2. Wanted: a sourced report with design patterns for a Rust gateway. For you: most primary pages were read, but the Bloomberg article, the TNW article and the trust.hark.com contents were not retrieved, and I found no PSD2 evidence. Now: complete. Next: none.

## Findings

Routes that worked: direct curl for hark.com HTML, sitemap, trust.hark.com shell and the TNW/rockingrobots APIs; r.jina.ai (rate-limited, worked on retry) for the hark.com pages; Wayback for the Pro article; the HN Algolia, iTunes lookup and iTunes reviews RSS endpoints.

- **Handoff architecture (PRIMARY).**
  - Each request gets a dedicated virtual computer with its own browser, file system and terminal.
  - The model outputs cursor and keyboard actions as x/y coordinates (click, type, scroll), not DOM or API calls.
  - It is post-trained with SFT then RL; mid-training is under way and pre-training is planned for later this year.
  - Hark says per-turn model latency is low, but browser latency adds roughly 10s per step.
  - [https://hark.com/articles/introducing-hark-handoff] (jina)
- **Product architecture (PRIMARY).**
  - Handoff runs up to 6 browsers at once.
  - Outside the browser, Hark uses direct APIs (Google, Outlook), file reads, external databases and MCP.
  - Persistent memory (it forgets on request) and proactive pacing feed Home, Action Buttons and Panels.
  - [https://hark.com/articles/introducing-hark-pro] (jina; Wayback snapshot 20261006214906 matches)
- **Secured by Hark (PRIMARY).**
  - An in-house encrypted vault for credit cards and passwords. Hark claims it cannot see the contents and cannot see a password as it is filled in.
  - The password is "encrypted all the way from the vault to the website until the moment it lands".
  - The security page says secrets are tokenized "in our hands". Third-party models get 0 data retention.
  - The mechanism is not disclosed: no KMS, enclave or injection-proxy detail.
  - [https://hark.com/security] (jina), [https://hark.com/articles/introducing-hark-pro] (jina)
- **Approvals (PRIMARY).**
  - The homepage says "Hark checks in with you before doing something that requires your approval."
  - The security page says tasks default to consent, and "over time, Hark remembers the actions you want it to take".
  - It also promises a visible live view of the CUA and "always a button to cancel".
  - The Terms add that Hark "will ask you to confirm before it spends your money or sends a communication to others" unless you say otherwise.
  - Standing permission can cover a "category of Transactions" and counts as authorization of each one inside it.
  - [https://hark.com/security] (jina), [https://hark.com/] (direct and jina), [https://hark.com/terms §4.10, §4.8] (jina)
- **Spend limits (PRIMARY, negative).**
  - I found no per-transaction cap, daily cap or merchant allowlist in the Terms, Privacy Policy, security page or Pro article.
  - Hark is not the merchant of record. Chargebacks go to the merchant or card issuer. Hark may refuse or stop any action.
  - [https://hark.com/terms §4.8, §4.13] (jina)
- **Usage-allowance top-ups (PRIMARY).** The Terms allow automatic top-ups that charge your payment method only if you turned them on and agreed to the amount. [https://hark.com/terms §5.4] (jina)
- **Sandbox and data (PRIMARY).**
  - Task execution runs in isolated VMs. Hark may log files, shell commands and outputs, and task logs.
  - It says it does not train on user data, and third-party AI providers are contractually barred from training.
  - [https://hark.com/privacy-policy] (jina)
- **Prompt-injection / abuse policy (PRIMARY).** The Acceptable Usage Policy bans "jailbreaking or prompt injection" against Hark. It does not describe a defense against injection coming from web content. [https://hark.com/acceptable-usage-policy] (jina)
- **Pricing (PRIMARY).**
  - Hark Pro is free, with all features in all plans. $20/month gives 2x usage and $100/month gives 10x.
  - The pricing page says plans differ only in usage.
  - App Store: free, version 2.3.0 (2026-10-13), 4.87 average from 172 ratings.
  - [https://hark.com/pricing] (jina), [https://hark.com/articles/introducing-hark-pro] (jina), [https://itunes.apple.com/search?term=hark+pro&entity=software] (curl)
- **Hardware (PRIMARY).**
  - Hardware is "designing now" and "AI-native". Funding: over $700M Series A at a $6B post-money valuation, about 70 staff, and an NVIDIA B200 training cluster.
  - No specs or dates were found.
  - [https://hark.com/articles/introducing-hark-pro], [https://hark.com/articles/hark-announces-usd700-million-fundraising-round] (jina)
- **Trust center (PRIMARY).** The meta description says SOC 2 Type 2 is "in progress" with Advantage Partners, plus third-party pen testing. The page body is gated behind an access request. [https://trust.hark.com] (curl for the meta, jina for the body)
- **Rockingrobots (SECONDARY).** Dated 2026-08-06. It says Handoff was shown in a first public demo, there was a waitlist, and Brett Adcock (Figure AI CEO) founded Hark. [https://www.rockingrobots.com/hark-unveils-handoff-an-ai-agent-that-completes-tasks-directly-in-a-browser/] (WP REST API)
- **App reviews (SECONDARY).**
  - Mostly 5-star. A skeptic says "zero differentiation" from Muse or Dot.
  - Users cite refund flows on Amazon, form filling, API discovery for a sprinkler system, and a "watch the agent navigate" live view.
  - A 1-star review says there is no way to turn off proactive messages and Hark ignored two "don't message me first" requests.
  - [https://itunes.apple.com/us/rss/customerreviews/id=6772362941/sortby=mostrecent/json] (curl)
- **Not retrieved.**
  - Bloomberg 2026-10-06 and the TNW article: Bloomberg URL unknown, and the TNW fetch and search returned only TNW's 404 shell. HN Algolia has only the 2026-10-06 "Hark Pro" submission (0 comments) and the 2026-08-05 Handoff submission.
  - Reddit and Wayback for those pages were not tried.
  - PSD2/SCA: no mention in any Hark page I fetched.

## Design patterns worth copying into a self-hosted Rust personal-agent gateway

1. **Consent ladder with graduated standing grants.** This is Hark's approval model made explicit.
   - `enum Approval { Ask, AskOnce(Scope), Standing(Scope) }`, where `Scope { action_kind, merchant, max_amount_minor, currency, expires_at }`.
   - Standing grants are minted only from an explicit user "always allow" tap, never inferred by the model.
   - Evaluate in order: deny rules, then standing grant, then Ask.
2. **Money and external-send as separate, non-bypassable effect classes.** Tag each tool `Effect::{Read, Write, Spend, SendExternal}`. `Spend` and `SendExternal` default to Ask, matching the Terms (§4.10), and standing grants must carry hard bounds, which Hark's public docs do not show.
3. **Spend ledger as the enforcement point.** Hark documents no cap, so build one.
   - Add per-transaction, per-day and per-merchant limits in minor units, checked in the gateway before the effect executes.
   - Use a Postgres or SQLite transaction on an append-only `spend_events` table, with an idempotency key to avoid double-spend on retries.
4. **Approval state machine.** `Proposed -> PendingApproval -> Approved | Denied | Expired -> Executing -> Settled | Failed | Cancelled`.
   - Each approval is bound to a hash of the exact action payload (merchant, amount, cart), so the approved action cannot drift.
   - Cancel is a first-class transition reachable at any state, matching Hark's "always a cancel button".
5. **Credential isolation: broker, not prompt.**
   - The model never sees secrets. It emits `fill(credential_ref="login:target.com")`.
   - A separate process, the broker, decrypts the secret and types it into the browser, and returns only success or failure.
   - Store secrets encrypted at rest (age or XChaCha20-Poly1305) with a key held outside the agent process. Redact secrets in logs and screenshots.
   - Hark claims this but does not disclose how, so treat it as a goal, not a verified design.
6. **Per-task sandbox with a scoped session.** One disposable browser/VM per task, with an egress allowlist and a short-lived credential lease. Hark logs commands and outputs per task; keep an audit log of every effect with its approval ID.
7. **Live view plus kill switch.** Stream screenshots or actions over a websocket and expose `POST /tasks/{id}/cancel`. Hark offers both.
8. **Proactivity budget.** A 1-star review shows the cost of unbounded proactive messages. Add per-user quiet hours, a hard daily cap and a global "never message first" flag that the agent cannot override.
9. **Hybrid action surface.** Use APIs and MCP first and fall back to the browser (Hark does this). Make the browser path the least-trusted tier with the strictest approvals.
10. **PSD2/SCA (no Hark evidence; my inference).** For EU payments, never automate SCA challenges. Surface the 3-D Secure or bank app step to the human and resume after confirmation.

## CLAIMS

- CLAIM: Hark's vault means Hark cannot see passwords or cards. RISK: Unverified marketing; mechanism undisclosed and a trust-center audit is still in progress. SOURCES: [https://hark.com/articles/introducing-hark-pro], [https://trust.hark.com]. PRIMARY: yes (claim), no (verification).
- CLAIM: Hark asks before spending money or sending messages. RISK: Standing permissions can waive this, and no documented caps exist. SOURCES: [https://hark.com/terms §4.8, §4.10]. PRIMARY: yes.
- CLAIM: Hark has per-transaction or daily spend limits. RISK: Not found in any fetched page, so it may exist in the app only. SOURCES: all fetched Hark pages. PRIMARY: absence only.
- CLAIM: Handoff is the top-ranked browser-use model. RISK: Self-reported comparison; TNW notes it has not been independently verified. SOURCES: [https://hark.com/articles/introducing-hark-handoff], [rockingrobots WP API]. PRIMARY: partial.
- CLAIM: Pricing is $20 for 2x and $100 for 10x usage. RISK: The App Store plan details were not fetched. SOURCES: [https://hark.com/pricing], [https://hark.com/articles/introducing-hark-pro]. PRIMARY: yes.
- CLAIM: Hark supports PSD2 or EU payments. RISK: No evidence found. The Terms cover EU consumer rights (14-day withdrawal) but say nothing about payment-services regulation. SOURCES: [https://hark.com/terms]. PRIMARY: absence only.
- CLAIM: Hark does not train on user data. RISK: Policy statement only; the privacy policy also permits "research" and "improve" uses. SOURCES: [https://hark.com/privacy-policy]. PRIMARY: yes.

## EXPAND

- Bloomberg 2026-10-06 piece: find the exact URL and try Wayback or jina. Also the TNW article via its sitemap or a TNW search page.
- trust.hark.com body: the subprocessor and controls list is behind a request-access form.
- Search Reddit (old.reddit.com .json) and X for prompt-injection or unauthorized-purchase incident reports.
- Pull the App Store page text for the real Pro-plan limits and purchase confirmations.
- The Hark Pro Android app and the `/content-generation-and-publication-policy` and `/other-policies` pages (the latter two not fetched).