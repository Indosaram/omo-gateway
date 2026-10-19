<!-- task=st_01a14506 -->
st_01a14506 [completed] model mahoquot/claude-sonnet-5-5
Ask: research agent credential and payment rails, with a sourced report. Wanted: Identity, Findings, Implications, CLAIMS and EXPAND, at most 1800 words. For you: the report is below. Visa, Mastercard, Stripe, the 1Password product pages and Pickle's "open-source credential component" were not verified from primary sources. Now: report delivered. Next: none.

I used GitHub API, raw GitHub files and the HN Algolia API. No web_search tool was available. Several vendor pages were unreachable or blocked by the broker.

## Identity

This is a cross-cutting scope, so each entity gets its own line.

| Entity | Company / maintainer | Official URL | Date | Confidence |
|---|---|---|---|---|
| Bitwarden Agent Access ("aac") | Bitwarden | github.com/bitwarden/agent-access | SDK announced 2026-03-30 (HN submission date) | High |
| Google AP2 | Google; standardization moving to FIDO | github.com/google-agentic-commerce/AP2, ap2-protocol.org | Announced 2025-09-16 | High |
| ACP (Agentic Commerce Protocol) | OpenAI and Stripe (maintainers) | github.com/agentic-commerce-protocol/agentic-commerce-protocol | Spec 2025-09-29; latest tag 2026-04-17; beta | High |
| MCP auth | Model Context Protocol project | modelcontextprotocol.io | Spec 2026-07-28 is the newest dated version; also a "draft" | High |
| 1Password agent features | 1Password | 1password.com | WSJ headline "Claude can now securely use your Passwords with 1Password", 2026-07-16 | Low |
| Visa Intelligent Commerce / TAP | Visa | developer.visa.com/capabilities/trusted-agent-protocol | Per a secondary report: TAP in developer preview; ICC Connect launched 2026-04-08 | Medium-low |
| Mastercard Agent Pay | Mastercard | mastercard.com | Per a secondary report: live in 10+ markets | Medium-low |
| Pickle | Nu Terra Labs (footer of picklebrowser.com) | picklebrowser.com | Show HN 2026-08-11 | Medium on identity, no on "credential component" |

The "1Password Secure Agent Access" name and "agentic autofill" returned no HN hits. I could not load the 1Password blog (the URL I tried returned a 404 page).

Pickle is an agent browser with policy-gated actions, not a credential product. Its distribution repo `ismailntl/pickle-releases` says "No source code". I found no open-source Pickle credential component, so that part of the brief is unconfirmed. Possibly the brief means a different "Pickle".

## Findings

- **Bitwarden Agent Access** is an open protocol, CLI and SDK that gives agents credentials from a password manager without exposing the whole vault. It uses an end-to-end encrypted Noise tunnel. The repo is Rust, Apache-2.0, and was pushed 2026-10-15 [https://github.com/bitwarden/agent-access]
- The README labels it "early preview" and warns that APIs may change. It recommends `aac run`, which injects secrets as env vars into a child process, over putting secrets in LLM context [https://github.com/bitwarden/agent-access]
- `aac listen` runs on the user's machine and creates a pairing token. `aac connect --token ... --domain ... --output json` is the non-interactive path for agents. Vault items can be fetched by domain or by item ID [https://github.com/bitwarden/agent-access]
- Bitwarden ships an OpenClaw skill example, and OneCLI integrates Bitwarden Agent Access [https://github.com/bitwarden/agent-access] [https://www.onecli.sh/blog/bitwarden-agent-access-sdk-onecli]
- **OneCLI** is an open-source credential gateway. It sits between agents and services, matches requests by host and path, swaps a placeholder for the real credential, and forwards. It needs nothing but `HTTPS_PROXY` on the agent [https://github.com/onecli/onecli] [https://news.ycombinator.com/item?id=49023427]
- OneCLI's approvals are enforced at the network layer, so they hold whether the agent uses MCP, CLI, curl or code it wrote itself. The authors say it cannot stop an agent misusing access it legitimately has [https://news.ycombinator.com/item?id=49023427]
- OneCLI v2 now pitches itself as a team agent harness. It keeps a Rust gateway with AES-256-GCM secrets, on-demand injection from Bitwarden or 1Password, and "deterministic human-in-the-loop approvals in the chat itself". It uses an enterprise licence for the `ee/` directories [https://github.com/onecli/onecli]
- **Agent IAP** (Rust, MIT, 2026-09-23, by Viktor Petersson) is an identity-aware proxy that pulls credentials from 1Password via `op://` references. The agent holds only a proxy-minted token. Rules are allow, deny or ask. "Ask" parks the request and prompts the owner [https://vpetersson.com/2026/09/23/little-snitch-meets-1password-for-ai-agents/]
- Agent IAP's defaults: the prompt starts on the narrowest scope and "Once", grants can carry deadlines, and unanswered prompts are denied. It writes a hash-chained audit log naming the agent and the rule, and has a kill switch (`L`) that denies everything [https://vpetersson.com/2026/09/23/little-snitch-meets-1password-for-ai-agents/]
- Its author says it does not stop exfiltration of data the agent can legitimately read. The proxy host holds every credential in memory, and the tokens are bearer tokens. This is a development tool, not a production one [https://vpetersson.com/2026/09/23/little-snitch-meets-1password-for-ai-agents/]
- **1Password:** a WSJ piece on 2026-07-16 headlined "Claude can now securely use your Passwords with 1Password" exists. A 1Password "SCAM" benchmark for agents was also posted. I could not read either, so the mechanism is unverified [https://news.ycombinator.com/item?id=48933975] [https://1password.github.io/SCAM/]
- **AP2** expresses authority as chained Verifiable Digital Credentials. Checkout Mandates and Payment Mandates each have an Open state (constraints) and a Closed state (a specific authorization). It covers human-present and human-not-present flows [https://github.com/google-agentic-commerce/AP2/blob/main/docs/index.md]
- AP2 is an extension of A2A and UCP. Standardization continues in FIDO working groups. The docs mention "v0.2" and a FIDO donation. A third-party report from April 2026 said v0.1 with 60+ collaborators, so the version differs by source [https://raw.githubusercontent.com/google-agentic-commerce/AP2/main/docs/index.md] [https://github.com/custena/agent-payment-protocols]
- **ACP** is Apache-2.0, beta, and maintained by OpenAI and Stripe. It has an Agentic Checkout API and a Delegate Payment spec. The 2026-04-17 version added cart, feed, orders, authentication and MCP modules [https://github.com/agentic-commerce-protocol/agentic-commerce-protocol]
- Stripe's docs are the implementation reference, linked from the ACP README. I did not read them [https://docs.stripe.com/agentic-commerce]
- A third-party report (vendor-authored, with a disclosed conflict of interest) says ChatGPT Instant Checkout was wound down in early March 2026 and OpenAI pivoted to discovery and referral [https://github.com/custena/agent-payment-protocols]
- **Visa and Mastercard**, per that same report: Visa TAP uses RFC 9421 HTTP message signatures for agent identity. Visa ICC Connect (2026-04-08) bridges TAP, MPP, ACP and UCP. Mastercard Agent Pay uses layered SD-JWT credentials, with identity via Cloudflare Web Bot Auth. Treat these as unverified secondary claims [https://github.com/custena/agent-payment-protocols]
- **MCP auth (spec 2026-07-28)** supports three client-registration mechanisms. Client ID Metadata Documents come first, then pre-registration, with Dynamic Client Registration as a deprecated fallback. Clients must bind stored credentials to the authorization server's `issuer` [https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/basic/authorization/client-registration.mdx]
- The spec splits authorization into discovery, registration and security-considerations pages. There is also an "enterprise-managed authorization" extension; I saw only its file listing and last-commit date of 2026-07-10, not its content [https://github.com/modelcontextprotocol/modelcontextprotocol/tree/main/docs/extensions/auth]
- **Pickle Browser** is local-first and has policy-gated actions: blocked domains, approval required for purchases, and a log of every action. It connects to MCP clients through a `pickle-mcp` server (72 tools, per the site). It is closed distribution with installers only [https://picklebrowser.com/] [https://news.ycombinator.com/item?id=49260102]
- HN practitioners converge on one pattern: the trust unit is the policy, not the payment. They pre-authorize a class of actions with a cap and gate irreversible ones, and prepaid virtual cards are the pragmatic rail today [https://news.ycombinator.com/item?id=47371289]

## Implications for a self-hosted personal agent gateway

1. **Credential-injecting egress proxy (Rust).** Run a local MITM or forward proxy that omo agents and cron jobs reach through `HTTPS_PROXY`. Agents hold only a gateway-minted token. The proxy swaps in the real secret by host and path. This is the OneCLI and Agent IAP model, and a Rust daemon on a Mac is a natural home for it.
2. **Pluggable secret backends, not your own vault.** Read from 1Password via `op://` references, from Bitwarden via `aac`, or from macOS Keychain. Keep only pointers in config. Do not cache secrets on disk.
3. **Ask-through-chat approvals.** An "ask" rule parks the request and sends the owner a Discord, iMessage or KakaoTalk prompt with the agent, the lane, the method plus URL, and Once, 1h or Deny buttons. Default to the narrowest scope. Deny on timeout. Rate-limit prompts so approval fatigue does not set in, which is the "Little Snitch trap" Agent IAP names.
4. **Lane-scoped grants.** Tie each ACL to a lane or agent identity, so the inbox-ingestion lane (untrusted input) gets read-only and no-write rules. A coding lane gets GitHub access, and cron gets only pre-approved standing rules. This limits the prompt-injection blast radius from ingested email or messages.
5. **Hash-chained audit log plus a kill switch.** Log every credential use with the agent, lane, rule and approver, and add a `/panic` command from the chat that denies all egress. Also add egress host allow-listing, because credential brokering does not stop exfiltration of data the agent can read.
6. **Spend policy object.** For purchases, model an AP2-style Open mandate in your own config: cap, merchants, expiry, human-present or not. The agent never sees card data. Use single-use or prepaid virtual cards and have the gateway call the payment API. Start with the policy and audit model. Adopt AP2 or ACP wire formats only once a merchant requires them.
7. **MCP OAuth client behavior.** When the gateway hosts or consumes MCP servers, store tokens in the gateway and key them by authorization-server `issuer`. Prefer a Client ID Metadata Document, which means a stable HTTPS URL for the daemon. Fall back to pre-registration. Avoid DCR.
8. **Expiring grants.** Every standing rule gets a deadline, and the gateway re-asks when it lapses.

## CLAIMS

- CLAIM: Bitwarden Agent Access is an open-source Rust protocol, CLI and SDK using a Noise-encrypted tunnel, in early preview. — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/bitwarden/agent-access
- CLAIM: OneCLI injects credentials at a gateway so agents never hold them, with Bitwarden and 1Password backends. — RISK: normal — SOURCES: github.com, news.ycombinator.com — PRIMARY: https://github.com/onecli/onecli
- CLAIM: Agent IAP is a Rust, MIT proxy with ask, allow and deny rules, 1Password-backed secrets and a hash-chained audit log. — RISK: normal — SOURCES: vpetersson.com, github.com — PRIMARY: https://vpetersson.com/2026/09/23/little-snitch-meets-1password-for-ai-agents/
- CLAIM: AP2 uses Open and Closed Checkout and Payment Mandates as verifiable credentials, and standardization is moving to FIDO. — RISK: normal — SOURCES: github.com — PRIMARY: https://raw.githubusercontent.com/google-agentic-commerce/AP2/main/docs/index.md
- CLAIM: ACP is an OpenAI and Stripe-maintained beta, Apache-2.0, with a 2026-04-17 spec adding cart, feed, orders, auth and MCP. — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/agentic-commerce-protocol/agentic-commerce-protocol
- CLAIM: MCP spec 2026-07-28 prefers Client ID Metadata Documents and deprecates Dynamic Client Registration. — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/basic/authorization/client-registration.mdx
- CLAIM: Visa TAP and ICC Connect, Mastercard Agent Pay and the ChatGPT Instant Checkout shutdown are as described in the Custena report. — RISK: high — SOURCES: github.com (a vendor-authored secondary source with a disclosed conflict of interest) — PRIMARY: none verified
- CLAIM: 1Password shipped a Claude integration around 2026-07-16. — RISK: high — SOURCES: news.ycombinator.com (a headline only; the WSJ article was not read) — PRIMARY: none verified
- CLAIM: Pickle Browser has policy-gated actions, but no open-source credential component was found. — RISK: normal — SOURCES: picklebrowser.com, news.ycombinator.com, github.com — PRIMARY: https://picklebrowser.com/

## EXPAND

- LEAD: Stripe agentic commerce docs (Shared Payment Tokens, delegate payment) — the ACP delegate-payment model is the likely template for a gateway-held payment credential — fetch docs.stripe.com/agentic-commerce directly.
- LEAD: Visa developer TAP docs and the Mastercard Agent Pay and Verifiable Intent spec — both were found only in a secondary source — fetch the primary pages and any open-source specs.
- LEAD: 1Password blog for Claude and its developer docs (Secure Agent Access, agentic autofill, `op` and SDK support) — I could not read the primary post — try blog.1password.com and developer.1password.com.
- LEAD: MCP `enterprise-managed-authorization` extension and `security-considerations.mdx` — I saw only their file listings — read the content for token-passthrough and confused-deputy rules.
- LEAD: "Pickle" disambiguation — confirm whether the brief means Pickle Browser (Nu Terra Labs), another repo, or a different project — ask the requester for a URL.