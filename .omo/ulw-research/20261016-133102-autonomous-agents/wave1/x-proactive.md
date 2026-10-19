<!-- task=st_01a14509 -->
st_01a14509 [completed] model mahoquot/claude-sonnet-5-5
Ask: research proactive/ambient agents (interrupt policy, LangChain ambient agents and Agent Inbox, Google proactive-agent papers, autonomy levels, HITL approval UX). Wanted: a sourced report.
For you: report below. Ledger: 5/5 research tracks covered; one gap on Google work (see EXPAND). Now: report delivered. Next: none.

I did not have a web_search tool. I used fetches of primary pages instead: the LangChain blog, GitHub raw files, the arXiv API and the HN Algolia API. I did not retrieve LangChain's current docs page on interrupts.

## Identity
- **Ambient agents**: a term coined by LangChain, Inc. (LangGraph/LangSmith). The founding post is "Introducing ambient agents", published 2025-01-14, last modified 2026-04-17. It sits at https://www.langchain.com/blog/introducing-ambient-agents (the older URL is blog.langchain.dev). Confidence: high.
- **Agent Inbox**: the open-source UI at https://github.com/langchain-ai/agent-inbox. It is described as "An inbox UX for interacting with human-in-the-loop agents". The last push was 2026-09-18 and `main` is at f1616f3e7998e7e4fd574545b9558554d88d9c49. A hosted version is at dev.agentinbox.ai. Confidence: high.
- **Reference app**: https://github.com/langchain-ai/executive-ai-assistant (EAIA), an email assistant. Confidence: high.
- **Levels of autonomy**: "Levels of Autonomy for AI Agents" by K. J. Kevin Feng, David W. McDonald and Amy X. Zhang (arXiv 2506.12469, June 2025). It was published in the Knight First Amendment Institute at Columbia's "AI and Democratic Freedoms" series. Confidence: high.
- **Google proactive-agent work**: no single canonical "Google proactive agents" paper turned up. The closest hit is the Google DeepMind paper "Proactive Agents for Multi-Turn Text-to-Image Generation Under Uncertainty" (ICML 2025). Confidence that this is the one you mean: low.
- **ProactiveBench**: it is not from Google. It comes from Tsinghua and affiliates (Yaxi Lu et al., arXiv 2410.12361). Confidence: medium, because I judged this from the author list alone.

## Findings
**LangChain ambient agents**
- An ambient agent listens to an event stream and acts on it, possibly on several events at once. It is not triggered solely by human messages. [https://www.langchain.com/blog/introducing-ambient-agents]
- LangChain says ambient agents are not necessarily fully autonomous. The key design question is when and how they interact with humans. [https://www.langchain.com/blog/introducing-ambient-agents]
- It names three interrupt types:
  - Notify: flag an important event without acting.
  - Question: ask the human to unblock the agent instead of guessing.
  - Review: approve, edit, or give feedback on a dangerous action, such as an outbound email. [https://www.langchain.com/blog/introducing-ambient-agents]
- The stated benefits of human-in-the-loop are lower stakes, user trust (it mimics human communication), and a feedback channel for long-term memory. [https://www.langchain.com/blog/introducing-ambient-agents]
- They started with Slack and moved away because notifications pile up into a backlog that is hard to navigate and constrains how you can respond. The Agent Inbox is modeled on an email inbox plus a ticketing system, showing all open lines of communication. Items are sorted by time, with priority sorting and assignment listed as future work. [https://www.langchain.com/blog/introducing-ambient-agents]
- The infrastructure LangChain names as necessary is checkpoint persistence (so the agent can pause), an `interrupt` primitive, a long-term memory store, and cron for polling event sources. [https://www.langchain.com/blog/introducing-ambient-agents]

**Agent Inbox schema**
- An interrupt carries `action_request {action, args}`, a `config` of four booleans (`allow_ignore`, `allow_respond`, `allow_edit`, `allow_accept`) and a markdown `description`. The response types are `accept | edit | response | ignore`. [https://github.com/langchain-ai/agent-inbox/blob/f1616f3e7998e7e4fd574545b9558554d88d9c49/README.md]
- The inbox always returns a list holding a single `HumanResponse`. `ignore` returns null args. `edit` returns the edited `ActionRequest`. [https://github.com/langchain-ai/agent-inbox/blob/f1616f3e7998e7e4fd574545b9558554d88d9c49/README.md]

**EAIA triage policy**
- Policy is user-written config in three buckets: `triage_no` (ignore), `triage_notify` (tell the user but do not draft), and `triage_email` (draft a response). Ingestion is a cron-driven job with `--minutes-since`, `--rerun` and `--early` flags. Reflection logic updates memory. [https://github.com/langchain-ai/executive-ai-assistant/blob/main/README.md]

**Autonomy levels (Feng et al.)**
- Autonomy is a deliberate design decision, separate from capability and environment. There are five levels, defined by the role the user takes: operator, collaborator, consultant, approver, observer. [https://arxiv.org/abs/2506.12469]
- The authors propose "autonomy certificates" to govern agent behavior, and sketch early ideas for evaluating autonomy. [https://arxiv.org/abs/2506.12469]

**When to interrupt: 2026 research**
- PRISM frames proactivity as cost-sensitive selective intervention. The agent intervenes only when the calibrated probability of user acceptance exceeds a threshold derived from the asymmetric costs of a false alarm and of missed help. The gate is decoupled from the response policy so it can be tuned and audited. It reports 22.78% fewer false alarms on ProactiveBench. [https://arxiv.org/abs/2602.01532]
- A survey on proactive service agents formalizes the choice among stay silent, ask, assist and act as a constrained decision process. It counts interruption, misunderstanding, overreach and privacy as costs. It concludes that reliable proactivity needs calibrated incremental value, verifiable authorization and recoverable execution. It also says offline classification scores do not predict deployment benefit. [https://arxiv.org/abs/2609.03727]
- Gating can be cheap. A small temporal-graph controller beats LLM triggers at about 11 ms per event and avoids paying inference when the agent stays silent. [https://arxiv.org/abs/2605.30152]
- A two-stage design, with a lightweight gate in front of a heavier reasoner, cuts false triggers. [https://arxiv.org/abs/2606.03236]
- Framing the decision as a single yes/no token and reading its probability improved macro-F1 by 0.249 over free-form generation. [https://arxiv.org/abs/2609.07099]
- Interrupt/silent decisions need de-duplication and irrelevance suppression, not just onset detection. [https://arxiv.org/abs/2609.21183]
- Earlier work on proactive agents that ask for help only when stuck, in states that need human recovery, comes from Chelsea Finn's group. [https://arxiv.org/abs/2210.10765]

**Google**
- Google DeepMind's proactive T2I agents ask clarifying questions when uncertain. They show their uncertainty as an editable "belief graph". At least 90% of human subjects found them helpful. [https://arxiv.org/abs/2412.06771]
- Speculation: the pattern of surfacing the agent's uncertainty as an editable artifact maps onto the `edit` action in Agent Inbox. No Google source states this.

**Security**
- A 2026 paper argues that scope boundaries must be verified while the agent runs, using executable scope contracts, egress control, stop conditions and re-authorization. It says the Gemini incident's causal mechanism is provisional, because the public record is only statements and journalism. [https://arxiv.org/abs/2610.12463]

## Implications for the gateway
1. **Typed interrupt envelope across channels.** Make every owner-facing message one of `notify | question | review`. Render `review` with accept / edit / respond / ignore buttons in Discord, and as keyword replies (`ok`, `edit: ...`, `no`) in iMessage and KakaoTalk, where buttons are unavailable. Persist pending interrupts in the daemon so a restart does not lose them. Allowed actions per interrupt should be a config bitset like `HumanInterruptConfig`.
2. **Local inbox ledger.** Keep a durable "open items" table with status, age and priority, like the Agent Inbox. Support `/inbox` to list pending items and a daily digest. This avoids the Slack failure mode LangChain describes, where unanswered pings become an unmanageable backlog.
3. **Triage tiers per source.** Mirror EAIA's `no / notify / act` buckets as a per-source policy file for inbox ingestion, which the owner can edit by message. Use a cheap pre-gate (rules plus a small model) before invoking an expensive agent, so silence costs nothing.
4. **Cost-asymmetric notification gate.** Score each candidate notification with an acceptance probability. Apply a threshold set by the false-alarm cost for the channel and time (for example, a high threshold overnight and a low one for a failed deploy). Log every suppressed item so the gate can be audited and recalibrated. Add de-duplication and rate limits per lane.
5. **Autonomy dial per lane.** Treat Feng et al.'s levels as a per-lane setting. For example, coding agents run as approver (they ask before push or destructive commands), cron housekeeping runs as observer (a digest only), and unknown senders run as consultant. Persist the level so it can be shown and changed in chat.
6. **Recoverable execution.** Pair `review` approvals with an undo or dry-run path, such as drafts, a worktree branch, and staged sends. Bind approvals to a hash of the exact action and args, so an edit invalidates the old approval. This follows the "verifiable authorization, recoverable execution" requirement.
7. **Question-first on ambiguity.** When an omo run lacks a preference, park the lane and send a `question` instead of guessing. Store the answers as memory entries, which become future policy and reduce later interrupts.
8. **Runtime scope contracts.** Give each lane an explicit allowlist (paths, hosts, accounts) that is enforced outside the agent, with automatic stop conditions that escalate to a `review`.

## CLAIMS
- CLAIM: LangChain defined ambient agents as event-stream-driven and possibly concurrent, and defined notify/question/review as the three HITL interrupt types. — RISK: normal — SOURCES: langchain.com — PRIMARY: https://www.langchain.com/blog/introducing-ambient-agents
- CLAIM: Agent Inbox supports accept/edit/respond/ignore, configurable per interrupt. — RISK: normal — SOURCES: github.com — PRIMARY: https://github.com/langchain-ai/agent-inbox/blob/f1616f3e7998e7e4fd574545b9558554d88d9c49/README.md
- CLAIM: Feng et al. define five autonomy levels by user role (operator, collaborator, consultant, approver, observer). — RISK: normal — SOURCES: arxiv.org — PRIMARY: https://arxiv.org/abs/2506.12469
- CLAIM: Decision-theoretic gating with asymmetric costs reduces proactive false alarms (PRISM, 22.78% on ProactiveBench). — RISK: normal — SOURCES: arxiv.org — PRIMARY: https://arxiv.org/abs/2602.01532
- CLAIM: The quoted gate and benchmark numbers come from each paper's own abstract and have not been independently replicated. — RISK: normal — SOURCES: arxiv.org — PRIMARY: https://arxiv.org/abs/2602.01532
- CLAIM: There is a canonical Google "proactive agents" paper. — RISK: high — SOURCES: arxiv.org — PRIMARY: none (unverified; only the DeepMind T2I paper at https://arxiv.org/abs/2412.06771 was found)

## EXPAND
- LEAD: Google/DeepMind proactive-assistant work beyond T2I (Gemini/Assistant proactive features, Project Astra, "proactive" in Pixel/Android) — WHY: the assigned Google angle is thin and may be a product, not a paper — ANGLE: blog.google and research.google.com searches, which I could not run without a search tool.
- LEAD: Current LangGraph/LangChain docs for human-in-the-loop interrupts (approve/edit/reject decisions) — WHY: the Agent Inbox README may be superseded by newer middleware APIs — ANGLE: docs.langchain.com human-in-the-loop page (my fetch of it failed).
- LEAD: Feng et al. follow-ups, such as autonomy certificates and an eval methodology — WHY: concrete lane-policy formats — ANGLE: Semantic Scholar citations of arXiv 2506.12469.