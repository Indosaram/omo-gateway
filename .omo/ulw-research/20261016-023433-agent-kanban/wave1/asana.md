### Asana
- URL: https://asana.com
- Kind: general-kanban-with-agent-integration
- What: A widely adopted enterprise collaborative work management platform offering Kanban boards, list, timeline, and Gantt views for tracking multi-step team workflows. Its Work Graph model connects tasks, projects, portfolios, and goals across organizations.
- Agent integration: Official REST API v1.0 (OAuth 2.0 and Personal Access Tokens) with webhooks and audit log APIs; community MCP servers (notably `@roychri/mcp-server-asana` and `@pipeworx/mcp-asana`) supporting Claude Code, Cursor, Codex, and Claude Desktop via stdio/HTTP; third-party CLI utilities (such as `timwehrle/asana`); built-in proprietary "AI Studio" and "AI Teammates" for native autonomous workflows inside Asana.
- Mobile: Native iOS app (Apple App Store: https://apps.apple.com/us/app/asana-work-in-one-place/id489969512) and native Android app (Google Play Store: https://play.google.com/store/apps/details?id=com.asana.app), plus responsive web interface.
- Pricing: Personal plan is free for up to 2 users (unlimited tasks, projects, boards); Starter plan is $10.99/user/month (annual) or $13.49/user/month (monthly); Advanced plan is $24.99/user/month (annual) or $30.49/user/month (monthly); Enterprise & Enterprise+ require sales quotes. AI Studio Basic included on paid plans (50k-200k monthly credits), with AI requests priced at $0.50/request (prepaid packs) or $0.60/request (on-demand), as seen 2026-10-16.
- Self-host: no — strictly multi-tenant cloud SaaS hosted on AWS; official FAQ explicitly confirms no on-premises version is offered or planned.
- License: Proprietary.
- Activity: Active commercial SaaS (publicly traded NYSE: ASAN); regular bi-weekly web/cloud updates; AI Studio and AI Teammates major rollout in 2024–2026.
- Pros: Mature enterprise-grade Kanban and project management with rich custom fields; Native AI Studio and AI Teammates embedded directly into Work Graph; Comprehensive REST API and webhooks for status automation; Well-maintained native iOS and Android apps; Broad enterprise security certifications (SOC 2 Type II, HIPAA, SAML/SCIM).
- Cons: No self-hosting option available; Per-seat pricing quickly escalates ($10.99–$24.99/user/month) plus additional fees for high AI request volume; Free tier restricted to maximum 2 users; No first-party Asana-maintained MCP server repository (relies on community-maintained MCP servers); Heavy enterprise feature bloat for solo or lean agent setups.
- Sources: https://asana.com, https://asana.com/pricing, https://asana.com/product/ai/ai-studio, https://asana.com/product/ai/ai-teammates, https://developers.asana.com/docs/overview, https://apps.apple.com/us/app/asana-work-in-one-place/id489969512, https://play.google.com/store/apps/details?id=com.asana.app, https://help.asana.com/s/article/ai-studio-smart-workflows?language=en_US
- Confidence: high (all pricing, mobile app availability, self-host stance, and API specifications verified from official Asana documentation)

### Asana AI Studio & AI Teammates
- URL: https://asana.com/product/ai/ai-studio
- Kind: agent-native
- What: Asana's native no-code agentic automation builder and autonomous AI agents integrated directly into Asana boards. It lets users define rule/prompt-based triggers, classification, routing, and actions that execute tasks directly on boards alongside human team members.
- Agent integration: Direct in-app integration via Asana Work Graph using LLM backends (OpenAI and Anthropic models with zero customer data retention); automated project rules and webhooks; external agent coordination possible via Asana REST API triggers and status comments.
- Mobile: Accessible through the native Asana iOS and Android mobile apps (status summaries, AI Dash task views, and notification feeds) and responsive web.
- Pricing: Requires an Asana paid plan (Starter, Advanced, or Enterprise); includes monthly credit allotments (50k credits on Starter, 75k on Advanced, 200k on Enterprise) plus 5 AI Teammate requests/user/month; additional AI usage billed at $0.50 per request prepaid or $0.60 per request on-demand (2026-10-16).
- Self-host: no — cloud-only managed SaaS running on Asana infrastructure with OpenAI/Anthropic cloud inference.
- License: Proprietary.
- Activity: Active production feature (announced late 2024, expanded through 2025–2026 with 30+ prebuilt AI Teammates and Asana Dash).
- Pros: Seamless zero-code setup for non-technical team members; Deep bidirectional integration with project boards, custom fields, and task assignments; Backed by enterprise security guardrails and non-training commitments with frontier LLM providers; Native multi-agent collaboration with shared memory and project context.
- Cons: Closed proprietary ecosystem not directly usable as an arbitrary local agent runtime; High variable costs per AI request ($0.50–$0.60/request); Inflexible execution environment compared to developer agents running code/CLI (Claude Code, Cursor); Locked into Asana cloud.
- Sources: https://asana.com/product/ai/ai-studio, https://asana.com/product/ai/ai-teammates, https://asana.com/pricing, https://help.asana.com/s/article/ai-studio, https://investors.asana.com/news-releases/news-release-details/asana-announces-ai-studio-no-code-builder-designing-and
- Confidence: high (verified via Asana AI Studio launch specifications, admin guides, and published pricing models)

### mcp-server-asana (Community MCP Server by roychri)
- URL: https://github.com/roychri/mcp-server-asana
- Kind: general-kanban-with-agent-integration
- What: An open-source Model Context Protocol (MCP) server implementation enabling external AI coding agents and desktop clients to read, create, update, and manage Asana tasks and projects over MCP stdio protocol.
- Agent integration: MCP (Model Context Protocol) over stdio; integrates directly into Claude Desktop, Claude Code, Cursor, Codex, and other MCP-compliant agent environments using Asana Personal Access Tokens (`ASANA_ACCESS_TOKEN`); provides dedicated tools for listing workspaces, searching tasks, creating tasks, modifying task sections (columns on Kanban boards), adding comments, and toggling optional read-only mode (`READ_ONLY_MODE=true`).
- Mobile: none (desktop/server CLI middleware; depends on client environment).
- Pricing: open-source free (free under MIT license; user pays only for underlying Asana account and LLM tokens).
- Self-host: yes via local Node.js (`npx @roychri/mcp-server-asana` or clone repository and run locally); runs completely on local workstation.
- License: MIT License.
- Activity: 149 GitHub stars; created December 2024, actively maintained with commits in October 2026; distributed via npm as `@roychri/mcp-server-asana`.
- Pros: Full MCP standard compliance for seamless Claude Code/Cursor integration; Supports board section manipulation for Kanban column status movement; Built-in safety switch (`READ_ONLY_MODE=true`) to prevent accidental board modifications; Lightweight zero-dependency binary execution via npx.
- Cons: Community-maintained without official Asana SLA or warranty; Requires generating and managing Personal Access Tokens manually; Lacks advanced Asana enterprise capabilities like custom field formula parsing or portfolio-level orchestration; Standard stdio transport requires local runner.
- Sources: https://github.com/roychri/mcp-server-asana, https://www.npmjs.com/package/@roychri/mcp-server-asana
- Confidence: high (verified repository code, npm package metadata, license, and tool list)

## EXPAND
- LEAD: Official Asana Developer MCP Server — WHY: Asana announced agentic work management initiatives in 2025/2026, but whether an official first-party MCP server has been published to Model Context Protocol's official registry or developer portal remains unconfirmed outside community servers — ANGLE: search Asana Developer relations forum and github.com/asana for 'model context protocol' or 'mcp'
