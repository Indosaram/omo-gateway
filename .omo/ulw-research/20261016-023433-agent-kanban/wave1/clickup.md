### ClickUp
- URL: https://clickup.com
- Kind: general-kanban-with-agent-integration
- What: A widely adopted all-in-one productivity and project management platform offering flexible Kanban boards, list views, and docs with deep customization for task status tracking. In late 2024 through 2026, ClickUp heavily expanded into agentic workflows via ClickUp Brain, autonomous AI agents, and first-party as well as community Model Context Protocol (MCP) integrations.
- Agent integration: Official remote MCP server (`https://mcp.clickup.com/mcp` supporting OAuth 2.1), full REST API v2/v3, webhooks, and popular community MCP servers (e.g. `hauptsacheNet/clickup-mcp` supporting API keys, and `taazkareem/clickup-mcp-server` supporting 150+ tools). Integrates with Claude Code, Cursor, Codex, Windsurf, Roo Code, VS Code, and custom agent runtimes to push status changes, query boards, create tasks, and manage comments.
- Mobile: Native iOS app (App Store: "ClickUp: Tasks, Chat, Docs, AI", requires iOS/iPadOS 17.4+, watchOS and visionOS support), native Android app (Google Play Store: `co.mangotechnologies.clickup`), plus responsive mobile web.
- Pricing: Free Forever ($0 for personal/small team use with 60MB storage and unlimited tasks); Paid workspace plans billed yearly: Core at $9/user/month (regular $12), Business at $17/user/month (regular $25), Enterprise (custom quote), and MAX at $100/user/month (regular $120). Official MCP usage includes 100 free calls/24h on Free Forever, 300/24h on Core, 1,000/24h on Business, 25,000/24h on Enterprise, and uncapped on MAX/Everything AI.
- Self-host: no (100% cloud SaaS hosted on AWS; enterprise localized data residency available in US, EU, and APAC data centers; no Docker or on-premise deployment).
- License: Proprietary
- Activity: Active commercial SaaS (v7.3.4 released Oct 2026 on iOS; Developer documentation updated October 2026; continuous platform feature rollouts).
- Pros: Extremely rich board and hierarchy views with customizable statuses, custom fields, and automations; First-party official remote MCP server (`mcp.clickup.com`) eliminates local setup overhead for standard OAuth clients; Mature ecosystem with both official remote MCP and local community MCP servers accepting simple API tokens; Comprehensive native mobile apps across iOS, iPadOS, Android, and watchOS; Granular tracking of tasks, comments, time-in-status, and subtask trees for AI agent monitoring.
- Cons: No self-hosting option (cloud-only AWS multi-tenant SaaS); Official MCP enforces strict rolling 24-hour rate limits unless subscribed to Enterprise or Everything AI/MAX tiers; Official MCP currently mandates OAuth 2.1 PKCE (disallowing direct static API keys on the official endpoint); Community MCP alternatives require separate maintenance or paid licenses (e.g., taazkareem transitioned to paid licenses); Complex UI/UX hierarchy can introduce steep configuration curves for automated board agents.
- Sources: https://clickup.com/pricing, https://developer.clickup.com/docs/connect-an-ai-assistant-to-clickups-mcp-server, https://developer.clickup.com/docs/mcp-tools, https://clickup.com/security, https://apps.apple.com/us/app/clickup-tasks-chat-docs-ai/id1535098836, https://play.google.com/store/apps/details?id=co.mangotechnologies.clickup
- Confidence: high

### ClickUp Brain & Agents
- URL: https://clickup.com/brain
- Kind: agent-native
- What: ClickUp's built-in conversational and autonomous AI agent layer (Brain² and Super Agents) embedded directly across ClickUp boards, docs, and chat. It acts as an ambient intelligent coworker that automates status updates, generates sprint and board summaries, extracts action items, and executes routine project management workflows across teams.
- Agent integration: Native ClickUp ecosystem integration via `@Brain` mentions, ambient AI triggers, AI Automations, and external MCP connectors; can consume external MCP data (e.g. GitHub, Google Drive, Salesforce) and orchestrate workspace actions natively without requiring an external IDE agent.
- Mobile: Fully accessible within the official ClickUp native mobile apps for iOS and Android as well as the responsive web app.
- Pricing: Add-on and tiered pricing: Brain AI starts at $9/user/month billed annually ($14 monthly, 1,500 AI Super Credits per user pooled); Everything AI / MAX bundle is $28–$100/user/month depending on workspace plan; top-up Super Agent credits cost ~$0.001 to $0.01 per credit; Free Forever plans include a trial allotment of 500 total workspace AI credits.
- Self-host: no (Proprietary cloud AI layer orchestrated across multi-model frontier backends including OpenAI GPT, Anthropic Claude, and Google Gemini with enterprise zero-retention agreements).
- License: Proprietary
- Activity: Active commercial product (Brain² rolled out through 2025–2026 with continuous model additions and ISO 42001 certification).
- Pros: Seamless zero-setup integration into existing ClickUp Kanban boards and workspaces; Context engine leverages organization graph and full task history without manual prompt assembly; Multi-model support dynamically routes prompts to optimal frontier models (Claude Opus, GPT, Gemini); Enables non-technical human team members to interact with agent statuses and summaries without coding tools.
- Cons: Proprietary closed ecosystem tethered exclusively to ClickUp; Per-user monthly add-on fee required across entire workspace team seats; Credit burn for complex multi-step automations requires monitoring; Cannot be modified or self-hosted in private or air-gapped server environments.
- Sources: https://clickup.com/brain, https://clickup.com/brain/pricing, https://clickup.com/pricing, https://clickup.com/security
- Confidence: high

### ClickUp Community MCP Servers (hauptsacheNet & taazkareem)
- URL: https://github.com/hauptsacheNet/clickup-mcp
- Kind: general-kanban-with-agent-integration
- What: Community-developed open-source and commercial Model Context Protocol (MCP) servers providing STDIO and streamable HTTP transports for LLM coding agents. They bridge local developer tools (Claude Code, Cursor, Codex, Windsurf, n8n) directly to ClickUp's REST API using simple static API keys and team IDs without requiring OAuth flows.
- Agent integration: Direct MCP protocol over STDIO (`npx @hauptsache.net/clickup-mcp` or `@taazkareem/clickup-mcp-server`) and SSE/Streamable HTTP; natively compatible with Claude Code CLI, Cursor, OpenAI Codex (`config.toml`), Windsurf, Roo Code, Antigravity (`agy`), and n8n workflows; supports task reading/writing, markdown descriptions, local image attachment uploads, and board status manipulation.
- Mobile: none (Developer tool / CLI / desktop daemon bridging external agents to ClickUp; mobile interaction occurs through ClickUp's mobile apps or host apps).
- Pricing: `hauptsacheNet/clickup-mcp` is open-source free ($0); `taazkareem/clickup-mcp-server` is commercial/paid ($9/month, $59/year, or $89 lifetime license for 3 devices via Polar.sh).
- Self-host: yes (Run locally via Node.js/npx, binary bundle `.mcpb`, or Docker container `docker run -d -p 3231:3231 ghcr.io/taazkareem/clickup-mcp-server:latest`).
- License: MIT License (`hauptsacheNet/clickup-mcp`) / Proprietary commercial (`taazkareem/clickup-mcp-server`).
- Activity: 49 stars, last pushed 2026-09-11 (`hauptsacheNet/clickup-mcp`); 53 stars, last pushed 2026-10-11 (`taazkareem/clickup-mcp-server`).
- Pros: Allows static API key authentication, avoiding complex OAuth browser handshakes for terminal/CI agents; Enables local image file uploads directly from disk into task comments and descriptions; Offers optimized modes like `read-minimal` to save token context windows in coding agents; Docker and local NPX execution provide zero third-party cloud proxy dependency for the OSS version.
- Cons: Subject to ClickUp public API rate limits (100 req/min on standard plans); taazkareem version requires a paid commercial license; May lag behind official ClickUp schema changes and new v3 API features; Requires manual configuration in agent settings files (`mcp.json` or `config.toml`).
- Sources: https://github.com/hauptsacheNet/clickup-mcp, https://github.com/taazkareem/clickup-mcp-server, https://api.github.com/repos/hauptsacheNet/clickup-mcp, https://api.github.com/repos/taazkareem/clickup-mcp-server
- Confidence: high

## EXPAND
- LEAD: ClickUp App Center Custom MCP Connectors (Outbound MCP) — WHY: ClickUp allows workspaces to register external MCP servers (such as GitHub MCP or Atlassian MCP) into ClickUp Brain, turning ClickUp into an MCP client as well as a server — ANGLE: web search "ClickUp App Center Add Custom MCP Server"
