### Notion Boards & Databases
- URL: https://www.notion.com
- Kind: general-kanban-with-agent-integration
- What: A flexible workspace and database system providing Kanban board views, tables, calendars, and timelines with customizable property schemas, relations, and automation triggers. Teams use it as a project and sprint tracker where cards represent tasks, documents, or agent runs.
- Agent integration: REST API (https://api.notion.com/v1, version 2026-03-11), official remote MCP (https://mcp.notion.com/mcp), Notion CLI (`ntn`), webhooks for real-time page/database updates, and official JS SDK (@notionhq/client). Integrates directly with Claude Code, Cursor, Codex, Devin, Hermes, and custom AI agents.
- Mobile: Native iOS app (Apple App Store: https://apps.apple.com/app/notion-notes-tasks-wikis/id1232780281); Native Android app (Google Play Store: https://play.google.com/store/apps/details?id=notion.id); responsive web application accessible across mobile browsers.
- Pricing: Free plan ($0, unlimited blocks for individuals, limited collaborative blocks for 2+ members, 5MB file upload); Plus plan ($10/user/month billed annually or $12 monthly, unlimited blocks, unlimited file uploads, 30-day version history); Business plan ($20–$25/user/month, SAML SSO, 90-day history, advanced security, includes Notion Agent core); Enterprise plan (custom pricing, SCIM provisioning, audit logs, unlimited history, HIPAA, zero data retention with LLMs). As of 2026-10-16.
- Self-host: no (strictly proprietary cloud-hosted multi-tenant SaaS; no on-premise binary, Docker container, or self-hosted server option available).
- License: Proprietary
- Activity: Active commercial SaaS. Official SDK (@notionhq/client) on GitHub has 5,673 stars with recent commits in October 2026; API actively updated with version 2026-03-11.
- Pros: Extremely customizable Kanban board views with multi-select status and formula properties; Rich documentation context allowing agent status cards to contain full markdown execution logs; Mature ecosystem with official hosted MCP server, webhooks, and modern 2026 REST APIs; Cross-platform native apps on iOS, Android, macOS, Windows, and web.
- Cons: Cannot be self-hosted on private infrastructure; Free tier limits collaborative block storage when multiple team members join; Cloud API rate limits (3 requests per second average burst limit) can bottleneck high-frequency agent state updates; Complex nested JSON schemas for page block manipulation compared to lightweight plain-text board tools.
- Sources: https://www.notion.com/pricing, https://www.notion.com/mobile, https://developers.notion.com/reference/intro, https://developers.notion.com/guides/get-started/overview.md, https://github.com/makenotion/notion-sdk-js
- Confidence: high

### Notion AI & Custom Agents
- URL: https://www.notion.com/product/ai
- Kind: agent-native
- What: Autonomous agent and AI teammate infrastructure embedded inside Notion workspaces. Features personal Notion Agent for chat/summaries and Custom Agents that run on schedules or webhook triggers to automate multi-step workflows across databases and connected tools like Slack and Google Drive.
- Agent integration: Notion Agent REST APIs (`/v1/agents/query`, `/v1/sessions`, `/v1/sessions/{id}/events/query`), Notion Workers Beta (serverless TypeScript tools with `@notionhq/workers`), Agent Skills API (download/upload `SKILL.md` compliant packages), and session management tools over remote MCP. Connects to Claude 3.5 Sonnet / Sonnet 5, OpenAI GPT models, and custom webhooks.
- Mobile: Accessible through the official native Notion iOS and Android mobile apps (integrated directly into mobile chat, workspace search, and page editing views); responsive mobile web.
- Pricing: Core Notion Agent (chat, autofill, meeting notes) is bundled into Business and Enterprise plans (Free/Plus workspaces have limited trial access). Custom Agents that run autonomously 24/7 run on Notion credits: free trial period through May 3, 2026, then priced at $10 per 1,000 monthly Notion credits starting May 4, 2026.
- Self-host: no (hosted on Notion cloud infrastructure with subprocessor LLMs like Anthropic and OpenAI; enterprise plans offer zero data retention agreements).
- License: Proprietary
- Activity: Active public beta / commercial release (2026). Public API for Custom Agents released with Notion API version 2026-03-11.
- Pros: Natively operates directly inside Notion Kanban boards and databases without external glue code; Autonomous execution on triggers or cron schedules even when human users are offline; Enterprise governance controls with credit limits, usage dashboards, and zero data retention; Compatible with standardized Agent Skills (`SKILL.md`) for cross-agent instruction sharing.
- Cons: Autonomous Custom Agents require paid Notion credits ($10/1,000 credits) starting May 2026; Proprietary closed-source agent engine cannot run local open-weight models directly inside Notion's engine; Tightly coupled to the Notion workspace boundary; Rate limits and credit ceilings can stall unattended agent pipelines.
- Sources: https://www.notion.com/product/ai, https://www.notion.com/pricing, https://developers.notion.com/guides/notion-agent-apis/overview.md, https://developers.notion.com/guides/notion-agent-apis/quickstart.md, https://developers.notion.com/guides/agent-skills/overview.md
- Confidence: high

### Notion MCP (Official Remote Server)
- URL: https://developers.notion.com/guides/mcp/overview.md
- Kind: agent-native
- What: Notion's official hosted Model Context Protocol (MCP) server running at `https://mcp.notion.com/mcp`. Exposes token-efficient tools for searching workspaces, reading and updating pages via enhanced Markdown, executing SQL queries against data sources, and managing Custom Agent sessions.
- Agent integration: Model Context Protocol (MCP) over Streamable HTTP (`https://mcp.notion.com/mcp`) and SSE fallback (`https://mcp.notion.com/sse`), with stdio bridging via `mcp-remote`. Directly supported and documented for Claude Code, Cursor, OpenAI Codex, Devin, Hermes, Pi, VS Code (GitHub Copilot), and fx. Authenticates via OAuth 2.0 PKCE or static Bearer MCP tokens (Business/Enterprise).
- Mobile: Client-dependent: accessible on mobile when used with mobile-capable MCP clients or web interfaces; Notion itself provides native iOS/Android apps for human viewing of MCP changes.
- Pricing: Server connection is included with Notion workspaces. Free/Plus plans support core tools with metered single-data-source SQL queries; Business and Enterprise plans unlock unlimited cross-data-source SQL, AI semantic search over connected apps, date sorting, and static bearer MCP tokens without interactive OAuth prompts.
- Self-host: no (hosted multi-tenant remote endpoint maintained by Notion at `https://mcp.notion.com/mcp`).
- License: Proprietary hosted service (tools interact with open-standard Model Context Protocol).
- Activity: Active production service, actively maintained as Notion's primary AI agent gateway in 2026. Regularly updated with new tools (`notion-search`, `notion-fetch`, `notion-query-data-sources`, `notion-create-pages`, `notion-upload-skill`).
- Pros: Zero local infrastructure required—connects directly via Streamable HTTP URL; Token-efficient enhanced Markdown page reading and editing compared to raw block JSON; Supports advanced SQLite querying (`notion-query-data-sources`) directly across database rows and Kanban boards; Native OAuth 2.0 and enterprise token management allow granular workspace access.
- Cons: Cannot be self-hosted or run air-gapped; Business/Enterprise plan required for bearer API tokens and multi-database SQL queries; Requires network connectivity to Notion's cloud endpoints; Subject to cloud latency and Notion API rate limits.
- Sources: https://developers.notion.com/guides/mcp/overview.md, https://developers.notion.com/guides/mcp/get-started-with-mcp.md, https://developers.notion.com/guides/mcp/mcp-supported-tools.md, https://developers.notion.com/guides/mcp/hosting-open-source-mcp.md
- Confidence: high

### notion-mcp-server (Official Open-Source Local Server)
- URL: https://github.com/makenotion/notion-mcp-server
- Kind: agent-native
- What: The original open-source local reference MCP server developed by Notion, distributed via npm as `@notionhq/notion-mcp-server` and Docker Hub as `mcp/notion`. Enables local agents to interact with Notion databases, data sources, and pages via stdio or streamable HTTP transport using internal integration tokens.
- Agent integration: Model Context Protocol (MCP) via local `stdio` or HTTP transport (`http://localhost:3000/mcp`), connecting to Claude Desktop, Cursor, Zed, GitHub Copilot CLI, and custom agents. Authenticates with static Notion internal integration tokens (`NOTION_TOKEN` / `ntn_...`). Supports multi-tenant token passthrough via `Notion-Token` header.
- Mobile: none (local server running on developer desktop or container; mobile clients can only access it if bridged via an exposed HTTP proxy).
- Pricing: Open-source free ($0, MIT license). Requires an active Notion workspace account (Free, Plus, Business, or Enterprise).
- Self-host: yes (can be run locally via `npx @notionhq/notion-mcp-server`, built as a Docker container with `docker run -i mcp/notion`, or deployed on custom private servers).
- License: MIT License
- Activity: Maintenance / Deprecated. Officially marked in 2026 as no longer actively maintained or supported by Notion in favor of Remote Notion MCP (`https://mcp.notion.com/mcp`). Repository has 4,667 GitHub stars, 634 forks, last pushed September 20, 2026, with 202 open issues.
- Pros: Can be self-hosted locally or inside private Docker environments; Supports headless local execution without browser OAuth prompts via `NOTION_TOKEN`; Includes streamable HTTP mode with bearer authentication and multi-tenant token passthrough; Exposes 22 tools including Markdown page read/update (`retrieve-page-markdown`, `update-page-markdown`) on API version 2026-03-11.
- Cons: Explicitly deprecated and not actively maintained by Notion developers; Lacks semantic AI search and cross-connected app retrieval available on remote MCP; Requires manual Notion integration setup and explicit database/page sharing permissions; JSON-heavy responses consume more context tokens than the optimized remote MCP server.
- Sources: https://github.com/makenotion/notion-mcp-server, https://github.com/makenotion/notion-mcp-server/blob/main/README.md, https://developers.notion.com/guides/mcp/hosting-open-source-mcp.md
- Confidence: high

### @suekou/mcp-notion-server (Community MCP Server)
- URL: https://github.com/suekou/mcp-notion-server
- Kind: agent-native
- What: A high-level, AI-friendly community Model Context Protocol server for the Notion API created by Kosuke Suenaga. Modernized for Notion API version 2026-03-11, it simplifies database/kanban querying and page manipulation with compact schema-aware tools and interactive MCP Apps.
- Agent integration: Model Context Protocol (MCP) over `stdio`, distributed via npm as `@suekou/mcp-notion-server`. Connects to Claude Desktop, Cursor, Windsurf, and other MCP-compliant hosts. Authenticates via internal integration token (`NOTION_API_TOKEN`). Exposes high-level tools (`notion_find`, `notion_read_page`, `notion_inspect_data_source`, `notion_query_data_source_by_values`, `notion_create_data_source_item_from_values`) and interactive UI apps (Data Source Explorer, Page Workbench).
- Mobile: none (desktop/local process; client dependent).
- Pricing: Open-source free ($0, MIT license).
- Self-host: yes (run locally via `npx @suekou/mcp-notion-server` or clone and build with Node.js 22+ and pnpm).
- License: MIT License
- Activity: Active open-source project. 920 GitHub stars, 181 forks; v2.0.2 released June 22, 2026; last repository push July 31, 2026.
- Pros: High-level value-based tools (`notion_create_data_source_item_from_values`) eliminate complex raw Notion JSON payloads; Schema validation against select/status options prevents bad database writes by AI agents; Supports compact response modes and Markdown conversion to preserve agent context windows; Includes interactive React-based MCP Apps for visual data exploration.
- Cons: Community-maintained project without official enterprise SLA or Notion support; Requires manual internal integration creation and token management; Only supports local stdio execution by default (no built-in remote multi-tenant HTTP server); Subject to upstream Notion API rate limits and permission restrictions.
- Sources: https://github.com/suekou/mcp-notion-server, https://raw.githubusercontent.com/suekou/mcp-notion-server/main/README.md, https://raw.githubusercontent.com/suekou/mcp-notion-server/main/docs/tools.md, https://api.github.com/repos/suekou/mcp-notion-server/releases
- Confidence: high

## EXPAND
- LEAD: Linear vs Notion for AI agent kanban — WHY: Linear has official webhook/agent integration and strict issue status state machines that often fit coding agents better than freeform Notion pages — ANGLE: web_search 'Linear MCP server official agent status kanban board 2026'
- LEAD: Focalboard / Mattermost Boards — WHY: Self-hosted open-source Notion alternative with Kanban views that can be deployed on-premise without cloud lock-in — ANGLE: web_search 'Focalboard self-hosted kanban agent API MCP 2026'
- LEAD: Planka / Taiga — WHY: Lightweight open-source Kanban boards with REST APIs for tracking autonomous background jobs — ANGLE: web_search 'Planka kanban docker self-host agent status webhook'
