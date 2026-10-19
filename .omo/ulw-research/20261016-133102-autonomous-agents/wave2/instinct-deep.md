<!-- task=st_01a1451d -->
st_01a1451d [completed] model mahoquot/claude-sonnet-5-5
Ask: research Instinct (Spear Street Technology) across terms/privacy, press, incidents and funding, with sources. Wanted: a sourced report.
For you: ledger 8 of 13 topics found and 5 not retrieved (details below). Now: report complete. Next: none.

I read this as an information ask. I stopped when each topic had a source or a stated gap.

## Findings

**Route per source:**
- instinct.com/terms and /privacy-policy: direct curl for privacy (server-rendered), r.jina.ai for terms.
- TechCrunch: its WP REST API (`/wp-json/wp/v2/posts?slug=…`) returned full article bodies with no challenge.
- Tweets: api.fxtwitter.com.
- Business Insider: r.jina.ai on the Yahoo syndication.
- HN: Algolia API.
- Wayback CDX: index only (body was gzip-garbled).

**Primary pages**
- The legal entity is Spear Street Technology, Inc. d/b/a Instinct. Terms and privacy were both "last revised August 26, 2026". [https://instinct.com/terms] (jina, PRIMARY)
- The terms contain the agency clause: "You hereby appoint the Services as your agent to enter into agreements, commitments or transactions on your behalf… binding on you as if entered into directly by you." The words "power of attorney" do not appear in the current terms. [https://instinct.com/terms §3 Actions] (jina, PRIMARY)
- On purchases, the terms say Instinct is not the buyer, seller or payment processor, and disputes are between you and the merchant. [https://instinct.com/terms] (jina, PRIMARY)
- Vault: materials added to the Vault are used "only to provide the Services" and never for model training. Opt-out of training is at app.instinct.com/settings, but flagged-for-safety content is still used. [https://instinct.com/terms §3 Materials] (jina, PRIMARY)
- Approval behavior in the terms: "We may implement safeguards, confirmation requirements… no representation that such safeguards will prevent unintended… Actions." You are told to configure your own restrictions and monitor actions, and "Actions may not always be reversible." [https://instinct.com/terms §3] (jina, PRIMARY)
- The terms bar scraping, bots and benchmarking. [https://instinct.com/terms §6] (jina, PRIMARY)
- Privacy: the assistant may be given payment cards and third-party usernames/passwords. Workspace data is excluded from AI training. Disconnecting an integration does not delete indexed data; deletion is at app.instinct.com/workspace. [https://instinct.com/privacy-policy] (curl, PRIMARY)
- Privacy security section: it admits unintended payments and communications, and "third parties who may include misleading instructions" aimed at agents. [https://instinct.com/privacy-policy] (curl, PRIMARY)
- The homepage says "text or call it. It can use a phone and computer in the same way that you do." The privacy policy mentions a "Mac OS" app. [https://instinct.com/] (curl, PRIMARY)
- Wayback shows instinct.co/terms captured 2026-08-26 and then returning 307 redirects from 09-04 onward. instinct.com/terms was captured through 10-06. [CDX API] (PRIMARY, index only)

**Architecture**
- Per TechCrunch, Instinct uses "its own phone number and computer" for tasks. It reads email, messages, calendar, and device audio, location and screen. [https://techcrunch.com/2026/09/28/viral-ai-agent-instinct-raises-1b-series-c-at-a-10b-valuation/] [https://techcrunch.com/2026/08/24/instincts-powerful-ai-assistant-is-raising-privacy-and-security-concerns/] (WP API, SECONDARY)
- The original terms, as screenshotted by critics, covered screen captures, cursor movements and keystrokes from devices, plus a "perpetual and irrevocable" license. [TechCrunch 08-24] (SECONDARY; I could not retrieve the original terms body)
- Shinn says isolation uses "isolated sandboxes… short-lived local credentials… identity-signed tool execution." [https://x.com/noahrshinn/status/2102896837522804954] (fxtwitter, PRIMARY)
- Each agent gets its own mailbox (mail.instinct.com), with 1Password for logins and Stripe for payments. [https://techcrunch.com/2026/09/09/viral-ai-assistant-instinct-now-has-its-own-email-address/] (WP API, SECONDARY)
- Features added later: Concierge phone calls (09-16), a "trusted person network" between agents, and group chats. [https://techcrunch.com/2026/09/17/rival-ai-agents-instinct-and-metas-muse-both-add-the-ability-to-make-calls/] (SECONDARY)
- Group-chat approval design: the personal agent asks before joining a group, and trust is per group and revocable. The group agent has no access to personal accounts. Pending replies are held when a new member joins. [https://x.com/noahrshinn/status/2107161132192690558] (fxtwitter, PRIMARY)

**Incidents**
- **Claire Vo:** she disconnected Google at 11 AM and still got an email summary at 2 PM. [https://x.com/clairevo/status/2090929592853037078] (fxtwitter, PRIMARY). TechCrunch adds that the bot said emails were stored in plain text.
- **Peter Yang:** Instinct would not delete his Gmail records. A deletion tool was added later. [TechCrunch 08-24] (SECONDARY)
- **Alex Cohen:** he phished Instinct with an email from a fresh Gmail account to his real one, and "Instinct happily followed the instructions!" He then deleted his account. [https://x.com/anothercohen/status/2091251836917350512] (fxtwitter, PRIMARY)
- **Resy:** Anita Kirkovska found that Instinct pulled a signup code from her Gmail and logged into Resy in its own browser "without asking me." [https://x.com/anitakirkovska/status/2090907681091403863] (fxtwitter, PRIMARY)
- **Katie Jacobs Stanton:** Instinct sent an email without checking first, and she disconnected it. [https://x.com/KatieS/status/2091152514603422074] (fxtwitter, PRIMARY)
- **Sep 21-23 cross-user leak scare:** @prit4k posted on 09-21 that Instinct described someone else's document. Shinn replied on 09-23 that it was a hallucination, not a breach, and announced a detector that can intercept the next thinking trace or tool call. [https://tech.yahoo.com/ai/deals/articles/instincts-founder-says-viral-ai-063054528.html] (jina, SECONDARY, Business Insider) and the Shinn tweet above (PRIMARY).
- **Resy abuse:** one HN comment refers to "vc twitter abusing resy thru instinct". [https://news.ycombinator.com/item?id=49624103] (HN Algolia, SECONDARY, weak)
- **Restaurant bookings:** Shinn said agents can check restaurant sites "every five seconds" and favor "special occasions". Critics pointed out agents could lie about anniversaries. [https://techcrunch.com/2026/09/29/instinct-founder-said-more-than-50-of-transactions-on-the-platform-are-travel-related/] (WP API, SECONDARY)

**Daily brief, pricing, funding**
- **Product recommendations:** Instinct Selections (09-29) pushed unsolicited product suggestions, and users complained. TechCrunch notes it is "ripe for some sort of advertising, commerce, or affiliate play." [https://techcrunch.com/2026/09/30/instincts-new-product-recommendations-are-giving-some-users-the-ick/] (SECONDARY)
- **Pricing:** I found no subscription price. TechCrunch reports a plan of "taking a cut of purchases", citing a Patrick O'Shaughnessy tweet. The terms add that payments are non-refundable and pricing can change. [https://techcrunch.com/2026/09/30/the-ugly-economics-of-consumer-ai/] (SECONDARY)
- **Funding:**
  - Series B: $250M at $2.5B (Index, Benchmark), $350M total. [https://techcrunch.com/2026/08/26/viral-ai-startup-instinct-has-raised-350-million-at-a-2-5-billion-valuation/]
  - Series C: $1B at $10B (Sequoia, Benchmark, Coatue). [TechCrunch 09-28]
  - Earlier backers reported: Kleiner Perkins and Conviction. [TechCrunch 08-24]
  - Scale claims: about $1B annualized transactions, over 50% travel, a 14-person team, 10% daily growth. [TechCrunch 09-29]
  - All via WP API, SECONDARY.

**Not retrieved**
- The CNN Resy-ban piece: CNN was blocked and a CNN search overflowed the fetch buffer.
- The Anish Acharya comparison tweet: I had no status ID and no search route.
- A "Sep 21-22 outage": I found only the leak scare above, no outage.
- The Forbes piece: it returned gzip garbage.
- "Residential identity" and the daily brief: no first-party description found.

## Design patterns for a self-hosted Rust personal-agent gateway

- **Approval ladder per action class.** Use `enum Risk { Read, Draft, SendExternal, Spend, Irreversible }` with a per-connector `ApprovalPolicy { auto_up_to: Risk, spend_cap_cents, per_merchant_allowlist }`. Default to confirming at `SendExternal` and above. This closes the Katie Jacobs Stanton and Resy cases, and Kirkovska's "progressive trust" idea fits here.
- **Credential-use gate.** Treat "agent uses a secret it found" (an OTP in email, a stored password) as a distinct `CredentialUse` event needing a grant. A login scoped to the connector is allowed. A code scraped from an inbox to log in elsewhere requires approval.
- **Taint tracking.** Mark inbound email and web content `Untrusted` in the context type. Forbid tool calls whose arguments derive from untrusted spans unless the user confirms. This addresses Cohen's phishing test directly.
- **Disconnect means purge.** Make `Connector::revoke()` a state machine: `Active → Revoking → Purged`, with a tombstone. Retrieval must filter on `connector.state == Active`. Claire Vo's and Peter Yang's incidents were index retention after disconnect.
- **Action journal.** Append-only `ActionRecord { id, intent, tool, args_hash, approval_ref, reversible, result }`. The terms admit Instinct's records "may not always be accurate", so the journal should be the source of truth for the daily brief.
- **Vault as a separate store.** Use a different table or key, excluded from any export or training path, and enforced at the type level (`VaultRef` cannot flow into the logging or telemetry sinks).
- **Per-scope agent instances.** Mirror the group-chat design with `Scope::{Personal, Group(id)}`. The group agent holds no personal credentials. Cross-scope disclosure goes through a `ShareRequest` approval. Hold queued outbound replies when `group.members` changes.
- **Pre-tool-call interceptor.** Add a `Guard::check(&Trace, &ToolCall) -> Allow | Steer | Block` hook before execution. This is Shinn's hallucination layer made explicit. Also add a grounding check that every proper noun in an outbound message appears in a retrieved source.
- **Rate and identity policy for sites.** Per-domain token buckets, because Shinn's "every five seconds" polling invites bans like the Resy one. Since the sites are the ones that ban, keep the user's own IP and identity for browsing and make it opt-in per domain.
- **Opt-in recommendations.** Keep any suggestion or notification channel opt-in and separate from the action channel. Instinct Selections drew backlash for pushing suggestions unprompted.

## CLAIMS

| CLAIM | RISK | SOURCES | PRIMARY |
|---|---|---|---|
| The terms appoint the service as the user's binding agent | Low; the wording is verbatim | instinct.com/terms §3 | Yes |
| Vault content is never used for training | Low for the terms; implementation unverified | instinct.com/terms §3 | Yes |
| Resy login used an email OTP without asking | Medium; one user's account | Kirkovska tweet | Yes (user report) |
| Disconnect did not stop summaries | Medium; user report, vendor confirmation via TechCrunch | Vo tweet and TC 08-24 | Yes (user report) |
| Sep 21 "leak" was a hallucination | Medium; the vendor's claim is unverified | Shinn tweet and BI | Yes (vendor claim) |
| Funding: $2.5B on 08-26, $10B on 09-28 | Low | TC 08-26 and 09-28 | No |
| Over 50% of transactions are travel; about $1B annualized | High; self-reported, calculation unspecified | TC 09-29 | No |
| Instinct was banned by Resy (CNN) | Unverified | none retrieved | No |
| Anish Acharya comparison tweet | Unverified | none retrieved | No |
| A Sep 21-22 outage occurred | Unverified; only the leak scare found | none | No |
| Cloud desktop plus local devices, "residential identity" | Unverified; only "own phone number and computer" and the Mac app found | TC 09-28, privacy policy | No |

## EXPAND

- Get the CNN Resy story. A browser or search-engine route is needed; r.jina.ai and direct curl were blocked or overflowed.
- Get the Forbes piece (a gzip-encoded response that the tool could not decode).
- Find the Acharya tweet by status ID (it needs a human search of X).
- Capture the original August instinct.co terms body via a Wayback `id_` fetch once the jina block clears.
- Find Instinct's pricing page or tweet, and any daily-brief documentation.