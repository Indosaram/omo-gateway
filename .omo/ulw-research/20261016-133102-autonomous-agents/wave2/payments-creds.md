<!-- task=st_01a14522 -->
st_01a14522 [completed] model mahoquot/claude-sonnet-5-5
Ask: research credential and payment delegation models for a self-hosted gateway, covering the listed sources. Wanted: a sourced report with a route log per source. For you: 7 of 8 source groups were read at primary level. Two are thin: Visa TAP (README only) and Mastercard Agent Pay (marketing page only). The 1Password page named "Secure Agent Access" does not exist under that name. Now: done. Next: none.

## Findings

**Individual availability (the answer to "what is available to individuals")**
- Stripe's Link Agent Wallet is the only individual-facing payment path I found. The agent creates a spend request for an amount, the user approves it in the Link app, and Link returns a one-time card, shared payment token (SPT) or Link Pay Token. It is available to US and Canadian customers. [https://docs.stripe.com/agentic-commerce/agents/link-agent-wallet/use-link-wallet-pay-online.md] PRIMARY, route: `.md` suffix via direct curl
- The same wallet's overview says it authenticates with a per-customer OAuth token (`LINK_ACCESS_TOKEN`), not a Stripe API key. Financial insights are US-only. [https://docs.stripe.com/agentic-commerce/link-cli.md] PRIMARY, route: `.md` suffix
- `link.com/agents` lists "granular agent controls" under "Coming soon". [https://link.com/agents] PRIMARY, route: direct curl
- SPT issuance (`/v1/shared_payment/issued_tokens`) needs a Stripe account, and the agent issues the token to a seller's Stripe profile. A personal gateway can only use it through Link. [https://docs.stripe.com/agentic-commerce/concepts/shared-payment-tokens.md?agent-seller=agent] PRIMARY, route: `.md` with the variant query
- Visa TAP is a merchant-verification protocol. I found no individual-facing path. [https://github.com/visa/trusted-agent-protocol] PRIMARY
- Mastercard Agent Pay says "only registered agents can transact". That implies agent registration, so I found nothing individual-facing. The page is marketing only. [https://www.mastercard.com/us/en/business/artificial-intelligence/mastercard-agent-pay.html] PRIMARY, route: r.jina.ai

**Stripe SPT**
- The SPT is a scoped grant with `usage_limits{currency,max_amount,expires_at}` bound to a seller profile. [SPT agent page above] PRIMARY
- Its states are `active`, `requires_action` and `deactivated`. `deactivated` is terminal, and the agent can revoke at any time. The webhooks are `requires_action`, `active`, `used` and `deactivated`. [same] PRIMARY
- The seller-side resolve API returns an fpan, dpan or agentic_token with optional RSA-encrypted fields. A token can be resolved only once, and a second resolve returns 400. [https://docs.stripe.com/agentic-commerce/concepts/shared-payment-tokens.md?agent-seller=seller] PRIMARY, route: `.md` with the variant query
- Stripe may use network tokens from Mastercard Agent Pay and Visa Intelligent Commerce behind SPTs. [same] PRIMARY

**ACP delegate_payment**
- `POST /agentic_commerce/delegate_payment` is a single endpoint that returns a vault token. The allowance is `reason=one_time`, `max_amount`, `currency`, `checkout_session_id`, `merchant_id` and `expires_at`. Card data is passed in the request, so the token issuer must be PCI-scoped. [https://github.com/agentic-commerce-protocol/agentic-commerce-protocol/blob/7fdd78df677a94dce04c770644b0fbbb1401272b/rfcs/rfc.delegate_payment.md] PRIMARY (draft RFC), route: raw GitHub via the gh commit SHA
- Idempotency rules: the key is scoped to identity plus path. Same key with a different body returns 422, an in-flight duplicate returns 409, 5xx responses are not cached, and keys are kept for at least 24 hours. [same] PRIMARY

**Google AP2** (docs are in the GitHub repo; `ap2-protocol.org/specification/` returned 404)
- Delegation has two models: a User Credential (OpenID4VP `transaction_data` with `type: "delegate"`) and a Trusted Agent Provider. [https://github.com/google-agentic-commerce/AP2/blob/e1ea56db72a6385bce3e5c1112b3a56ce60acb43/docs/ap2/agent_authorization.md] PRIMARY, route: raw GitHub at the gh-resolved SHA
- Mandates are open or closed. An open mandate carries `constraints` and a `cnf` key for the agent. The agent closes it with a key-binding JWT. Unknown constraints must fail evaluation. [same] PRIMARY
- The verifier returns a signed Mandate Receipt, and the agent then narrows the open mandate based on it. [same] PRIMARY
- Payment constraints: `agent_recurrence`, `allowed_payees`, `allowed_payment_instruments`, `allowed_pisps`, `amount_range`, `budget`, `reference`, `execution_date`. Budget requires tracked cumulative spend. [https://github.com/google-agentic-commerce/AP2/blob/e1ea56db72a6385bce3e5c1112b3a56ce60acb43/docs/ap2/payment_mandate.md] PRIMARY
- AP2's threat model assumes prompt injection cannot be prevented and treats LLMs as potential attackers. The payment credential is released only after a final Payment Mandate is verified, and receipts must be integrity-protected from the LLM. [https://github.com/google-agentic-commerce/AP2/blob/e1ea56db72a6385bce3e5c1112b3a56ce60acb43/docs/ap2/security_and_privacy_considerations.md] PRIMARY

**Visa TAP**
- It uses RFC 9421 signatures with timestamp, session id, key id and algorithm id, bound to the merchant domain and operation. An agent registry holds public keys. The sample repo has no spend limits or token issuance. [https://github.com/visa/trusted-agent-protocol/blob/16d59bdf3f8a542bc538d0962edbb80ea30a02af/README.md] PRIMARY (README only; last push 2025-10-28)

**Mastercard Agent Pay**
- The marketing page names four pillars: know your agent, interface standards, verifiable intent, and Agent Pay for Machines. It does not describe a technical flow. `developer.mastercard.com` is a JavaScript SPA and returned no content. [URL above] PRIMARY (marketing)

**1Password** (no page called "Secure Agent Access" exists in `llms.txt`; the closest are below)
- The NVIDIA OpenShell integration gives the agent placeholders only. The gateway substitutes real secrets outside the sandbox for approved host and port only, with a TTL. Expiry stops substitution while the sandbox keeps running, and writes are opt-in. [https://www.1password.dev/environments/nvidia-openshell.md] PRIMARY (beta), route: `www.1password.dev/*.md` (`developer.1password.com` served an empty SPA shell)
- The Environments MCP server runs locally, returns names but never values, and prompts for approval per Environment until the app locks. [https://www.1password.dev/environments/mcp-server.md] PRIMARY
- `op run --environment` injects secrets into the MCP server process instead of storing them in `mcp.json`. [https://www.1password.dev/get-started/secure-ai-access.md] PRIMARY

**MCP**
- Security best practices, draft: token passthrough is forbidden; servers must accept only tokens issued for them. Handles must be bound to the authenticated user. Scopes should be progressive, with no omnibus scopes. SSRF blocking is required for OAuth discovery fetches, including private ranges and redirect hops. [https://github.com/modelcontextprotocol/modelcontextprotocol/blob/86f6e0c675afea9a8f233e09b6faf566b126086c/docs/docs/draft/tutorials/security/security_best_practices.mdx] PRIMARY
- Authorization security considerations: RFC 8707 `resource`, PKCE S256, short-lived tokens with refresh rotation, and per-client consent before forwarding to a third party. [https://github.com/modelcontextprotocol/modelcontextprotocol/blob/86f6e0c675afea9a8f233e09b6faf566b126086c/docs/specification/draft/basic/authorization/security-considerations.mdx] PRIMARY
- Enterprise-managed authorization (stable): the client exchanges an ID token for an ID-JAG (`audience` is the resource authorization server, `resource` is the MCP server, about 300s lifetime), then presents it as a `jwt-bearer` grant. The issued token must be audience-restricted. The identity provider sees only issuance, not the MCP traffic. [https://github.com/modelcontextprotocol/ext-auth/blob/main/specification/stable/enterprise-managed-authorization.mdx] PRIMARY, but fetched from `main`, so it is not SHA-pinned
- The extension requires an enterprise identity provider, so it is not useful for an individual. [https://github.com/modelcontextprotocol/modelcontextprotocol/blob/86f6e0c675afea9a8f233e09b6faf566b126086c/docs/extensions/auth/enterprise-managed-authorization.mdx] PRIMARY

**Routes that failed**
- Direct curl on `docs.stripe.com` returned a 450KB HTML shell.
- r.jina.ai hit per-IP rate limits (429) on repeat calls.
- `ap2-protocol.org/specification` returned 404.
- `developer.1password.com` and `developer.mastercard.com` are SPAs.
- I did not need hn.algolia, Reddit or web.archive.org after the above. The web.archive.org attempt for Stripe overflowed the output buffer.

## Design patterns worth copying into a self-hosted Rust gateway

1. **Lease state machine, from the Stripe SPT.** Use `enum LeaseState { Active, RequiresAction(NextAction), Deactivated(Reason) }`. Deactivation is terminal. Reasons are `Consumed | Expired | Revoked | Resolved`. Revoke is a user or admin command. Emit `used` and `deactivated` events to an audit log.
2. **Limits struct bound to a destination.** `Lease { id, principal, destination: HostPort|SellerId, max_amount_minor: u64, currency, expires_at, single_use: bool }`. Single-use means one resolve. A second resolve is an error (Stripe returns 400).
3. **Agent never holds the secret, from 1Password and OpenShell.** Hand the agent opaque placeholders. A gateway egress layer substitutes the secret only when the request matches the lease's host, port and method class. Writes are opt-in per lease. After the TTL, substitution stops and the call fails closed.
4. **Open and closed mandates, from AP2.** Store the user-approved "open" grant with `constraints: Vec<Constraint>`. Each concrete action creates a "closed" record that must satisfy every constraint. Make the `Constraint` enum non-exhaustive and fail closed on unknown variants. Start with `AllowedPayees`, `AmountRange`, `Budget`, `ExecutionWindow` and `Recurrence`.
5. **Persistent ledger.** Budget and recurrence need cumulative spend. Keep a `(mandate_id, closed_id, amount, state)` table in the gateway's store. Commit the idempotency record and the ledger entry in one transaction, following the ACP RFC.
6. **Receipts outside LLM reach.** After each action, write a gateway-signed receipt (hash of the closed action plus a result). The LLM sees only a status, never the receipt keys. Do not issue an overlapping closed action until the previous one has a receipt (the AP2 double-spend rule).
7. **Human approval card, from Link.** Create `SpendRequest{ id, amount, merchant, context(min 100 chars), line_items }` and set the status to `PendingApproval`. Approval comes from a trusted surface the agent cannot drive (a local TUI, a push notification or a passkey prompt). Use a 30-minute expiry, `Approved -> Submitted -> Succeeded|Failed`, and `Denied` as terminal with no retry. Branch on `next_action.resolution` rather than on the error type.
8. **Approval rules to hard-code.**
   - Auto-approve only below a per-transaction cap, inside a daily cap and a thirty-day cap, for an allow-listed payee. Link exposes `per_transaction`, `daily` and `thirty_day` caps.
   - Always prompt for amount increases or new payees. Require re-approval for any increase, and treat the new total as a fresh request.
   - Use a tiered scope model from the MCP guidance: a read-only baseline, with elevation challenges on first use of a risky tool.
9. **Idempotency.** Scope keys to principal plus path. Return the original response on an identical replay, 422 for a different body under the same key, and 409 while a request is in flight. Do not cache 5xx. Retain keys for at least 24 hours.
10. **Inbound token hygiene, from MCP.** Validate `aud` on every token. Never forward a client token upstream; mint a separate upstream credential. Bind handles to `<principal>:<handle>`. Apply an SSRF guard (private ranges, no automatic redirect following) to any URL fetched during discovery.
11. **Card handling.** Write PAN output to a `0600` file, not stdout, as Link recommends. Never log full PANs. Prefer network tokens or SPTs over raw cards.

## CLAIMS
- CLAIM: Stripe SPT is the only first-party individual route to agent payments, and it requires Link (US and Canada) — RISK: low, but it may change; the docs use a preview API version — SOURCES: the Link overview and spend-request pages — PRIMARY: yes
- CLAIM: SPT issuance via the Stripe API is business-only — RISK: medium; I inferred this from the Stripe account requirement and did not find it stated outright — SOURCES: the SPT agent page — PRIMARY: yes
- CLAIM: AP2 treats the LLM as an attacker and releases credentials only after a final Payment Mandate is verified — RISK: low — SOURCES: AP2 security doc at the pinned SHA — PRIMARY: yes
- CLAIM: AP2 constraints evaluate against tracked cumulative state, so the gateway must keep a ledger — RISK: low — SOURCES: AP2 payment mandate doc — PRIMARY: yes
- CLAIM: ACP delegate_payment passes raw card data, so the issuer needs PCI scope — RISK: medium; this is an RFC draft marked "Status: Draft" — SOURCES: the ACP RFC at the pinned SHA — PRIMARY: yes
- CLAIM: Visa TAP gives merchants a way to verify agents, not to delegate spending — RISK: medium; I read only the README — SOURCES: the TAP README — PRIMARY: yes
- CLAIM: Mastercard Agent Pay is registration-gated and needs an issuer or PSP — RISK: high; the page is marketing only and "registered agents" is the only gating statement I found — SOURCES: the Mastercard page — PRIMARY: marketing only
- CLAIM: 1Password's OpenShell integration is placeholder substitution with a TTL and host allow-list — RISK: low, but the feature is beta and supports Mac and Linux only — SOURCES: the OpenShell page — PRIMARY: yes
- CLAIM: 1Password's MCP server never returns secret values — RISK: low — SOURCES: the MCP server page — PRIMARY: yes
- CLAIM: MCP enterprise-managed authorization needs an enterprise identity provider and is out of reach for an individual — RISK: low (inference from the spec) — SOURCES: the ext-auth spec — PRIMARY: yes, not SHA-pinned

## EXPAND
- The "Secure Agent Access" name did not appear in `llms.txt`. Search `llms-full.txt` in chunks, since range requests were blocked.
- Fetch the Link OAuth page (`.../link-agent-wallet/oauth.md`) and the `stripe/link-cli` repo. They show how a personal gateway could hold the token and the approval flow.
- Fetch the Mastercard Agent Pay developer documentation through a rendering route. Plain fetch cannot read the SPA.
- Read the Visa TAP signature-input specification and the RFC 9421 profile.
- Pin the ext-auth spec to a commit SHA.
- Read the draft IETF ID-JAG spec if enterprise-style delegation matters.