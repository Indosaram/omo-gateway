<!-- task=st_01a14507 -->
st_01a14507 [completed] model mahoquot/claude-sonnet-5-5
Ask: research the "a2a" topic (agent-to-agent protocols and personal-agent coordination) and deliver a sourced report - wanted: the report in the requested format. For you: A2A, MCP, ANP and Entra Agent ID are verified from primary sources. Pickle's agent-to-agent behavior is not verified, because its site is a JavaScript app that curl can't read. Now: report complete. Next: none.

# Report: a2a (agent-to-agent protocols and personal-agent identity)

I used no web_search tool. All evidence comes from `gh` and `curl` against GitHub, Microsoft Learn, pickle.com and HN Algolia. Reddit returned only CSS and was unusable. HN has no relevant Pickle agent-to-agent discussion.

## Identity

| Item | Details | Confidence |
|---|---|---|
| **A2A (Agent2Agent)** | Open protocol, Apache-2.0, repo `a2aproject/A2A` (https://github.com/a2aproject/A2A). v1.0.0 on 2026-03-12, v1.0.1 on 2026-05-28. The repo was first created under Google; I did not verify that origin or the current governance. | High on version and license; medium on governance |
| **MCP** | Model Context Protocol. Stable revision `2026-07-28` released 2026-07-28; the previous stable revision was `2025-11-25`. https://github.com/modelcontextprotocol/modelcontextprotocol | High |
| **ANP (Agent Network Protocol)** | Open-source community project (ANP Community), Apache-2.0, current spec set 1.2. https://github.com/agent-network-protocol/AgentNetworkProtocol, site https://agent-network-protocol.com/ | High |
| **Microsoft Entra Agent ID** | Microsoft product that extends Entra to AI agents. Docs last updated 2026-08-13. https://learn.microsoft.com/en-us/entra/agent-id/what-is-microsoft-entra-agent-id | High |
| **Pickle** | https://pickle.com is a Korean-default landing page titled "Pickle: your own AI agent" (the tagline translates as "an AI agent that does tasks on your behalf"). HN history links it to "Pickle 1: The first soul computer", AR glasses, a "Memory OS" and "Glass". The page source mentions "Critter", invite links (`/i/:token`) and a Korea/Japan market split. | Medium on identity; **low** on any agent-to-agent feature |

## Findings

**A2A**
- A2A positions itself as complementary to MCP. MCP connects an agent to tools and data. A2A connects agents to each other and supports multi-turn negotiation and clarification. [https://raw.githubusercontent.com/a2aproject/A2A/main/docs/topics/what-is-a2a.md]
- Discovery is by Agent Card, served at `/.well-known/agent-card.json`. The card carries identity, endpoint, capabilities, security schemes and skills. Other strategies are curated registries and direct configuration. The spec defines no standard registry API. [https://raw.githubusercontent.com/a2aproject/A2A/main/docs/topics/agent-discovery.md]
- A2A has "authenticated extended agent cards" for sensitive detail. It recommends out-of-band dynamic credentials over static secrets in the card. [https://raw.githubusercontent.com/a2aproject/A2A/main/docs/topics/agent-discovery.md]
- The request flow is: fetch the card, authenticate (OIDC/OAuth), then call `SendMessage` or `SendStreamingMessage`. Tasks move through states such as submitted, working and completed. Updates arrive by streaming or push notification. [https://raw.githubusercontent.com/a2aproject/A2A/main/docs/topics/what-is-a2a.md]
- Identity is not carried in the payload. It is established at the HTTP layer through TLS, bearer tokens or API keys. Authorization can be per skill, using OAuth scopes. A server that needs extra credentials mid-task signals this, and the client obtains them out of band. [https://raw.githubusercontent.com/a2aproject/A2A/main/docs/topics/enterprise-ready.md]
- v1.0.0 made breaking changes. It split the protocol definition from the transport bindings, added `tasks/list` with pagination, and modernized OAuth (removed implicit and password flows, added device code and PKCE). It also added gRPC multi-tenancy, merged the two push-config types, and removed the redundant `final` field. [https://github.com/a2aproject/A2A/releases/tag/v1.0.0]
- The `a2aproject/A2A` repo has about 26k stars and was pushed 2026-10-16, so it is actively maintained. [https://github.com/a2aproject/A2A]

**MCP**
- The 2026-07-28 revision made MCP stateless. It removed the `initialize` handshake and `Mcp-Session-Id`. Each request now carries its protocol version and client capabilities in `_meta`, and servers must implement `server/discover`. [https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/docs/specification/2026-07-28/changelog.mdx]
- Tasks moved out of core into an official extension, `io.modelcontextprotocol/tasks`. It uses polling via `tasks/get`, plus `tasks/update` for client-to-server input. [same changelog]
- Server-initiated requests (`sampling`, `elicitation`, `roots`) were replaced by multi-round-trip requests. The server returns `input_required` and the client retries with `inputResponses`. [same changelog]
- Roots, Sampling and Logging are deprecated, with a minimum 12-month deprecation window. HTTP+SSE is deprecated. [same changelog]
- Dynamic Client Registration is deprecated in favor of Client ID Metadata Documents. Clients must bind credentials to the issuing authorization server. [same changelog]
- Results now carry `ttlMs` and `cacheScope`, and OpenTelemetry `traceparent` is documented for `_meta`. [same changelog]

**ANP**
- ANP is a broader stack than A2A. It covers DID-based identity (`did:wba`, native `did:web`), naming (WNS handles), agent description and discovery, and end-to-end encrypted direct and group messaging. [https://raw.githubusercontent.com/agent-network-protocol/AgentNetworkProtocol/main/README.md]
- Maturity is mixed. Identity, description and discovery are "Released v1.2". The meta-protocol negotiation layer (ANP-06), DID-based authorization (ANP-05) and AP2 payments (ANP-10) are drafts. Group end-to-end encryption is a candidate. [same README]
- The README states that publishing the specs does not mean SDKs or products implement them, and that interoperability is still being validated. [same README]
- Reference implementations are AgentConnect (SDK) and AWiki, which includes a Rust CLI, `awiki-cli-rs2`. [same README]

**Microsoft Entra Agent ID**
- Entra Agent ID gives each agent a distinct identity, separate from workforce and workload identities. Agents can act autonomously with their own rights, or on behalf of a user via delegated access. They can also authenticate incoming calls from other agents using Entra tokens. [https://learn.microsoft.com/en-us/entra/agent-id/what-are-agent-identities]
- Identities are designed for short-lived agents, created in bulk and retired without leaving orphaned credentials. Each agent records a human sponsor. [same page]
- An agent identity can optionally be paired one-to-one with an "agent user account" for systems that expect a human user. [same page]
- The platform supports OAuth 2.0, MCP and A2A. Third-party agents (AWS Bedrock, n8n) can be governed through an Auth SDK sidecar or workload identity federation. [https://learn.microsoft.com/en-us/entra/agent-id/what-is-microsoft-entra-agent-id]
- Agent ID is available to all Entra customers. Conditional Access, Identity Protection and governance for agents require Microsoft Agent 365, which is bundled in M365 E7 or sold as an add-on. [same page]

**Pickle (limited)**
- The pickle.com `<head>` comments reference invite-link pages, a Korea/Japan market variant and a "Critter act in real time" article. The site body is client-rendered, so product capabilities, including any agent-to-agent behavior, could not be read. [https://pickle.com/]
- HN has no discussion of Pickle agents talking to each other. Only submissions of pickle.com itself turned up ("Pickle 1", "Memory OS", "Glass"), all with 0-6 points. [https://hn.algolia.com/api/v1/search_by_date?query=pickle.com]
- GitHub repository search for Pickle plus AI agent returned unrelated hits (security tools, "Big Pickle" OpenCode model). [https://github.com/search?q=pickle+ai+agent]
- The invite-link route suggests, as speculation only, a social or referral graph between Pickle users. Nothing found shows agents negotiating with each other. [https://pickle.com/]

## Implications for a self-hosted personal agent gateway

1. **Publish an Agent Card at `/.well-known/agent-card.json`, off by default.** Expose the owner's omo lanes as A2A skills (for example "schedule", "code-task", "inbox-triage"). Serve only a public minimal card, and put the detailed skill list behind the authenticated extended card. [A2A discovery doc]
2. **Make it an A2A server first, then an A2A client.** Map A2A tasks onto lane jobs and the A2A task states onto your lane lifecycle. Use SSE or push notification for long coding runs. The Rust daemon can reuse the same task store for Discord, iMessage and KakaoTalk replies.
3. **Use a per-peer trust allowlist with skill scopes.** A2A expects OAuth scopes per skill. Do the same: a peer token maps to a lane and a skill set. Every outbound action beyond read-only (spending, sending messages as the owner) goes to the owner's channel for approval. This follows the A2A rule that agents act as gatekeepers. [enterprise-ready doc]
4. **Treat A2A input as untrusted inbox content.** Route peer messages through the same ingestion and prompt-injection filtering as email and iMessage. Never let a peer message reach an omo agent with write tools without a gate. (This is design advice, not drawn from a source.)
5. **Mirror Entra's sponsor and delegation model locally.** Each spawned omo agent gets a short-lived identity (a scoped token or DID) with a recorded owner and a TTL. It acts either on its own scope or as the owner with explicit delegation. Log every action under that identity. [Entra agent identities doc]
6. **Expose gateway tools via MCP 2026-07-28 and keep it stateless.** Implement `server/discover`, pass the version in `_meta`, use the tasks extension for long jobs, and expect multi-round-trip `input_required` flows. Turn `input_required` into a Discord or iMessage question to the owner. Don't build on Sampling, Roots or the SSE transport, which are deprecated.
7. **Propagate W3C trace context (`traceparent`) from inbound A2A and MCP calls into lanes and cron jobs.** This gives one audit trail across inbox, lane and peer. [MCP changelog; A2A enterprise doc]
8. **Defer ANP.** Its identity layer (did:wba) is the only part worth watching. The authorization, negotiation and group end-to-end encryption pieces are drafts. Don't commit to it until SDK interoperability is shown. [ANP README]

## CLAIMS
- CLAIM: A2A v1.0.0 shipped 2026-03-12 and v1.0.1 on 2026-05-28 — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/a2aproject/A2A/releases
- CLAIM: MCP's stable `2026-07-28` revision removed the session handshake, added `server/discover`, and moved tasks to an extension — RISK: normal — SOURCES: github.com, modelcontextprotocol.io — PRIMARY: https://modelcontextprotocol.io/specification/2026-07-28/changelog
- CLAIM: MCP deprecated Sampling, Roots, Logging, HTTP+SSE and Dynamic Client Registration — RISK: normal — SOURCES: github.com — PRIMARY: https://modelcontextprotocol.io/specification/2026-07-28/changelog
- CLAIM: A2A carries no identity in the payload and relies on HTTP-layer authentication with per-skill scopes — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/a2aproject/A2A/blob/main/docs/topics/enterprise-ready.md
- CLAIM: ANP's authorization, meta-protocol negotiation and AP2 payments are drafts, and group end-to-end encryption is only a candidate — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/agent-network-protocol/AgentNetworkProtocol
- CLAIM: Entra Agent ID supports OAuth, MCP and A2A, and its security features require Agent 365 licensing — RISK: normal — SOURCES: learn.microsoft.com — PRIMARY: https://learn.microsoft.com/en-us/entra/agent-id/what-is-microsoft-entra-agent-id
- CLAIM: Pickle (pickle.com) has a shipping agent-to-agent feature — RISK: **high** — SOURCES: pickle.com only — PRIMARY: none. Unverified; the site could not be read.
- CLAIM: A2A originated at Google, and the Linux Foundation now governs it — RISK: normal — SOURCES: none fetched — PRIMARY: none. Background knowledge only; I did not verify it.

## EXPAND
- LEAD: Pickle's actual product behavior (agent-to-agent, invite graph, "Critter") — WHY: the SPA hid all content, so the central claim in the brief is unverified — ANGLE: a headless-browser render of pickle.com, its app-store listings (KR/JP), the `/memory` and `/glass` pages, and press coverage.
- LEAD: the A2A v1.0 spec text (`docs/specification.md`) and the Rust SDK status — WHY: the Rust daemon would implement the Agent Card schema, task states and push-config types, and I read only the topic docs — ANGLE: read the spec and check `a2a-rs` or the SDK list in the repo README.
- LEAD: MCP `io.modelcontextprotocol/tasks` extension and the MRTR pattern pages — WHY: they define how long-running lane jobs and owner questions would map — ANGLE: fetch the pages under `/specification/2026-07-28/basic/patterns/mrtr` and `/docs/extensions`.
- LEAD: ANP `did:wba` and ANP-09 messaging profiles — WHY: they are the only candidate for a decentralized, owner-controlled agent identity — ANGLE: read ANP-02/03 and test `awiki-cli-rs2` for interoperability.
- LEAD: Entra Agent ID on-behalf-of and sponsor flow details — WHY: a model for delegation and audit — ANGLE: read `what-is-agent-id-platform` and the agent OBO docs.

**Remaining uncertainty:** the Pickle identity and its agent features, and A2A governance.