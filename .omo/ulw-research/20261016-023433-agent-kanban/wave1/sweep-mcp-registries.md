# MCP Registries Kanban & Task Board Survey (Wave 1)

Date: 2026-10-16  
Angle: MCP Registries Sweep (Smithery.ai, Glama.ai, PulseMCP, mcp.so, GitHub Model Context Protocol Ecosystem)  
Focus: Kanban and task-board tools for monitoring and orchestrating AI agent work status  

Seen tools (omitted from deep profiling per brief): Trello, Jira, Linear, Notion, Asana, ClickUp, monday, Plane, WeKan, Kanboard, Planka, Vikunja, Focalboard, Taiga, Huly, Vibe Kanban, Task Master, Backlog.md.

---

### Kaneo
- URL: https://kaneo.app
- Kind: general-kanban-with-agent-integration
- What: An open-source, lightweight project management platform built with Hono, React, and TypeScript designed as a minimalist alternative to Jira and Linear. It incorporates a native built-in MCP server that enables coding agents to read board backlogs, create issues, and transition task statuses directly during development turns.
- Agent integration: Official built-in MCP server, REST API; supports Claude Code, Cursor, OpenAI Codex, Windsurf.
- Mobile: responsive web (no native iOS/Android apps; mobile browser responsive UI, cite: kaneo.app).
- Pricing: Free open-source self-hosting ($0 forever); Kaneo Cloud starts with a 14-day free trial, then $4.17/user/month (annual billing: $3.33/user/month) or $5/user/month as of 2026-10-16.
- Self-host: yes, Docker Compose and Node.js container/binary; no enterprise gating on self-hosted features.
- License: MIT
- Activity: 9,494 GitHub stars; latest release v2.46.2 on 2026-10-12; active daily commits.
- Pros: Clean Linear-inspired user interface; Zero-cost self-hosting with complete MIT license; Native MCP server built into the repository; Fast and lightweight Hono backend.
- Cons: Designed primarily for human software teams rather than autonomous agent-to-agent negotiation; No native mobile app for iOS or Android; Lacks agent collision prevention and distributed leasing mechanisms.
- Sources: https://kaneo.app, https://kaneo.app/pricing, https://github.com/usekaneo/kaneo
- Confidence: high

### EasyKanban
- URL: https://easykanb.app
- Kind: general-kanban-with-agent-integration
- What: A calm, distraction-free online kanban board built for focused work, featuring a remote Streamable HTTP MCP server and OAuth 2.1 authentication. It allows assistants like Claude, ChatGPT, Cursor, and Codex to retrieve to-do summaries and manage cards without requiring manual API key generation.
- Agent integration: Official remote MCP server endpoint (https://easykanb.app/api/mcp), Streamable HTTP transport, OAuth 2.1 (PKCE + Dynamic Client Registration); supports Claude Web/Desktop, Claude Code (`claude mcp add --transport http easykanban`), ChatGPT Connectors, Cursor, VS Code.
- Mobile: responsive web (mobile browser optimized, no standalone native App Store or Play Store app, cite: easykanb.app).
- Pricing: Free tier (€0/month) includes full read-only board querying (`list_boards`, `get_board`, `list_cards`, `get_todo_overview`); EasyKanban Pro is €6/month to unlock write tools (`create_card`, `update_card`, `move_card`, `archive_card`, `delete_card`) as of 2026-10-16.
- Self-host: no (cloud-hosted SaaS).
- License: Proprietary
- Activity: Active production SaaS; listed and verified on Smithery with OAuth 2.1 integration as of 2026-10-16.
- Pros: Zero local installation required via Streamable HTTP remote endpoint; Frictionless OAuth 2.1 PKCE login without hardcoded secrets; Generous free tier for agent task inspection; Native support across Claude Code and ChatGPT Connectors.
- Cons: Proprietary cloud-only service with no self-hosting; Agent card creation/mutation locked behind €6/month Pro tier; No native mobile app; Lacks agent-swarm specific features like task locking.
- Sources: https://easykanb.app, https://easykanb.app/mcp, https://easykanb.app/#pricing, https://api.smithery.ai/servers/e-goetmann/easykanban
- Confidence: high

### Nodiom Cloud
- URL: https://nodiom.md
- Kind: agent-native
- What: A dedicated shared state layer and kanban board built specifically for autonomous AI agents operating across different machines, models, or vendors. Agents claim tasks without collisions, hand work off to peer agents, and pause for human approvals with transparent author attribution and Discord/Slack alerts.
- Agent integration: Official MCP server (synexiom-labs/nodiom-cloud) providing 17 tools (`board_create`, `board_read`, `board_add_card`, `board_move_card`, `board_update_card`, `board_delete_card`, etc.), REST API; supports Claude Code, Cursor, Codex, custom agent harnesses.
- Mobile: responsive web with Discord, Slack, and email webhook review alerts for mobile decision approvals (cite: nodiom.md).
- Pricing: Free tier ($0) includes 10,000 writes/month, unlimited agents, unlimited boards, 3 workspaces, and unlimited free reads; Pro tier is $49/month for unlimited workspaces and high-throughput agent fleets as of 2026-10-16.
- Self-host: yes (core Nodiom engine is open-source MIT on GitHub; cloud service adds managed multi-agent sync and review routing).
- License: MIT (core engine) / Proprietary (cloud platform)
- Activity: Active private beta and verified Smithery server deployment as of 2026-10-16.
- Pros: Purpose-built for multi-agent swarm collision prevention; Clear attribution tracking which model made each write; Plain Markdown backing based on MIT open-source engine; Integrated Slack/Discord approval notifications.
- Cons: Relatively new entrant with a small ecosystem; Jump from free tier to $49/month Pro tier is steep for small hobbyists; No standalone native iOS or Android mobile application.
- Sources: https://nodiom.md, https://nodiom.md/pricing, https://api.smithery.ai/servers/synexiom-labs/nodiom-cloud
- Confidence: high

### Nimbalyst
- URL: https://nimbalyst.com
- Kind: agent-native
- What: An open-source visual workspace, task tracker, and kanban board tailored for running coding agents such as Claude Code, Codex, and OpenCode in parallel. It combines visual kanban task management with live Markdown/Excalidraw/Mermaid editors, isolated git worktrees, and deep MCP support.
- Agent integration: Native MCP server integration, local CLI agent runners (Claude Code, Codex, OpenCode), git worktree synchronizer.
- Mobile: native iOS app available on the Apple App Store companion app; responsive web (cite: nimbalyst.com, GitHub README).
- Pricing: Free open-source for individuals ($0); Teams tier is $20 per user per month; Enterprise is $200 per user per year as of 2026-10-16.
- Self-host: yes, native desktop app (macOS, Windows, Linux) storing files in local git worktrees; team server is self-hostable.
- License: MIT
- Activity: 1,908 GitHub stars; latest release v0.80.6 on 2026-10-09; actively maintained.
- Pros: Local-first architecture with native desktop apps and an official iOS companion app; Visual WYSIWYG editing of agent artifacts alongside kanban cards; Automatic git worktree creation and isolation per task; Fully open-source under MIT license.
- Cons: Heavier desktop installation compared to browser-only web boards; Real-time multi-user team synchronization requires a paid tier or custom server; iOS companion app focuses on monitoring rather than autonomous execution.
- Sources: https://nimbalyst.com, https://nimbalyst.com/pricing, https://github.com/nimbalyst/nimbalyst
- Confidence: high

### craaft
- URL: https://craaft.io
- Kind: general-kanban-with-agent-integration
- What: A collaborative, real-time kanban board platform featuring an official MCP server with 74 fine-grained tools. It allows software development teams and AI coding agents to drag, create, update, and review cards simultaneously with instant live board updates.
- Agent integration: Official MCP server (craaft/craaft) exposing 74 tools (e.g. `add_column`, `create_card`, `bulk_move_cards`, `archive_card`, `create_webhook`), REST API, webhooks; supports Claude Desktop, Claude Code, Cursor, Windsurf.
- Mobile: responsive web (optimized for mobile browsers and touch devices, cite: craaft.io).
- Pricing: Free tier ($0 forever) supports up to 3 projects, unlimited cards, live updates, and complete MCP/API access; Pro plan is $14/month for unlimited projects and team roles as of 2026-10-16.
- Self-host: no (cloud-hosted SaaS).
- License: Proprietary
- Activity: Active production SaaS; listed on Smithery and Glama; tools updated in 2026.
- Pros: Comprehensive MCP tool surface with 74 endpoints covering cards, milestones, checklists, and webhooks; Real-time live board syncing as agents execute moves; Generous free tier with full MCP access; Built-in importer from Trello, Jira, and Asana.
- Cons: Closed source with no self-hosting option; No native iOS or Android mobile application; Lacks agent-specific safety constraints or timeout circuit breakers.
- Sources: https://craaft.io, https://craaft.io/mcp, https://craaft.io/pricing, https://api.smithery.ai/servers/craaft/craaft
- Confidence: high

### Hamloy
- URL: https://hamloy.com
- Kind: agent-native
- What: An agent coordination task board and question inbox for Claude Code, Codex, Cursor, ChatGPT, and human teams. Agents record active tasks, log completed or ruled-out solutions, and prompt humans with structured choice dialogs when blocking decisions arise.
- Agent integration: Official MCP server (hamloy/hamloy) providing tools (`claim`, `tick`, `assume`, `answer`, `message`, `lists`, `view`), single-click browser OAuth authentication without pasting tokens.
- Mobile: responsive web with PWA / mobile web application manifest (`apple-mobile-web-app-capable`) allowing decision answering on phones (cite: hamloy.com HTML head).
- Pricing: Free plan ($0) for one project and solo users; Pro plan is €8/month (or €80/year) for unlimited projects and team collaboration as of 2026-10-16.
- Self-host: no (cloud-hosted SaaS).
- License: Proprietary
- Activity: Active SaaS; verified and listed on Smithery with ongoing updates throughout 2025 and 2026.
- Pros: Unique 'question and options' mechanism specifically designed for agent-to-human decision unblocking; Coordination-over-orchestration philosophy keeps agents autonomous; Transparent agent assumption and completion logs; Smooth mobile decision-making interface.
- Cons: Closed source SaaS; Free tier is limited to a single project; Less customizable column structure compared to traditional kanban tools.
- Sources: https://hamloy.com, https://hamloy.com/pricing, https://api.smithery.ai/servers/hamloy/hamloy
- Confidence: high

### Tasklix
- URL: https://tasklix.dev
- Kind: agent-native
- What: A shared fleet coordination board and task queue designed for autonomous AI agent fleets to read and write over MCP. Features automated task claiming, sibling agent handoffs, human approval gates before consequential actions, and fleet credit/execution metering.
- Agent integration: Official MCP server (hello-n43p/tasklix) offering 29 tools (`claim_task`, `poll_tasks`, `update_task_status`, `list_fleet`, `list_capabilities`, `add_task_attachment`), REST API; supports Claude Code, Cursor, Codex swarms.
- Mobile: responsive web (optimized mobile browser view for checking fleet status, cite: tasklix.dev).
- Pricing: Free tier ($0) includes 500 one-time welcome credits, up to 5 registered agents, and 1 project; Starter tier is $9.99/month (1,000 monthly credits, 10 agents, 3 projects); Growth tier is $49.99/month (5,000 monthly credits, unlimited agents & projects) as of 2026-10-16.
- Self-host: no (cloud-hosted platform).
- License: Proprietary
- Activity: Active SaaS listed on Smithery; active adoption for multi-agent fleet operations in 2026.
- Pros: Built-in agent fleet registry with capability discovery; Consequential action gating requiring explicit human approval; Automated task queueing and peer agent handoffs; Built-in execution audit logs.
- Cons: Credit-based pricing model can be hard to forecast with autonomous agent loops; Closed source SaaS; No native mobile apps for iOS or Android.
- Sources: https://tasklix.dev, https://tasklix.dev/pricing, https://api.smithery.ai/servers/hello-n43p/tasklix
- Confidence: high

### amux
- URL: https://amux.io
- Kind: agent-native
- What: An open-source control plane and task board for AI coding agents written in Rust. It enables running parallel Claude Code, Codex, and Gemini CLI workers side by side with a shared atomic task board, origin-stamped inter-agent messaging, model switching, and self-healing recovery.
- Agent integration: Native CLI, tmux session multiplexer, built-in MCP server, REST API; directly supervises and controls Claude Code, Codex, Gemini CLI.
- Mobile: native iOS app on the Apple App Store (`amux — Agent Multiplexer`, id6760410435); responsive web dashboard (cite: amux.io, Apple App Store).
- Pricing: Open source free ($0); optional commercial managed Concierge / Done-for-you service available for businesses as of 2026-10-16.
- Self-host: yes, single statically linked Rust binary for macOS and Linux; zero external database dependencies.
- License: MIT + Commons Clause
- Activity: 534 GitHub stars; latest release v0.9.108; active commits through 2026-10-16.
- Pros: Single Rust binary with minimal footprint and zero external database setup; Official native iOS app on App Store for supervising agent workers from a phone; Integrated tmux process supervision and self-healing crash recovery; Origin-stamped inter-agent messaging.
- Cons: Commons Clause limits commercial resale of hosted instances; CLI/terminal-heavy initial setup; Community size is smaller than traditional project management tools.
- Sources: https://amux.io, https://github.com/mixpeek/amux, https://apps.apple.com/us/app/amux-agent-multiplexer/id6760410435
- Confidence: high

### Claude-Code-Agent-Monitor
- URL: https://hoangsonww.github.io/Claude-Code-Agent-Monitor/
- Kind: agent-native
- What: A real-time local-first monitoring dashboard and Kanban status board for Claude Code, Cursor, and OpenAI Codex sessions. Built with SQLite3, Express, React, Vite, and WebSockets, it tracks live tool calls, token cost analytics, agent hierarchies, and subagent orchestration states.
- Agent integration: Local hooks/WebSocket bridge, MCP server integration, SQLite event logger; supports Claude Code, Cursor, OpenAI Codex.
- Mobile: responsive web accessible across the local network via mobile browser; native desktop packages for macOS and Windows (cite: GitHub README).
- Pricing: 100% Free and open source ($0).
- Self-host: yes, local Node.js/Express server or Docker container with SQLite database.
- License: MIT
- Activity: 1,080 GitHub stars; latest release v2.3.0 on 2026-10-10; actively maintained.
- Pros: Complete local privacy with zero cloud telemetry; Live WebSocket streaming of agent status changes and tool calls; Dedicated Kanban board view for tracking multi-agent tasks; Packaged native desktop apps for macOS and Windows.
- Cons: No native mobile apps (responsive web only); Requires running a local background Node process; Focused specifically on Claude Code/Codex sessions rather than general team project planning.
- Sources: https://hoangsonww.github.io/Claude-Code-Agent-Monitor/, https://github.com/hoangsonww/Claude-Code-Agent-Monitor
- Confidence: high

### scrumboy
- URL: https://scrumboy.com
- Kind: general-kanban-with-agent-integration
- What: A lightweight, self-hosted agile project management system with customizable kanban boards, workload flow analytics, and a built-in MCP server for AI agent automation. Built with Go and SQLite, supporting instant shareable boards and real-time updates.
- Agent integration: Official built-in MCP server (`mcp-server` package), REST API; supports Claude, Cursor, and MCP-compatible coding agents.
- Mobile: PWA (Progressive Web App) with offline caching and mobile-friendly touch board interactions (cite: scrumboy.com, GitHub repository topics).
- Pricing: 100% Free open-source ($0); no paid tiers or commercial feature restrictions.
- Self-host: yes, single Go binary or Docker container with SQLite database; includes OIDC authentication.
- License: AGPL-3.0
- Activity: 445 GitHub stars; latest release v3.38.8 on 2026-10-16; very active daily commit frequency.
- Pros: Instant anonymous board creation with no account wall; PWA mobile app support; Single Go binary self-hosting with embedded SQLite; Built-in MCP server for agent read and write automation.
- Cons: AGPL-3.0 license may restrict proprietary enterprise deployments; Utilitarian/retro pixel styling might not suit corporate aesthetics; Lacks agent-specific task leasing or collision prevention.
- Sources: https://scrumboy.com, https://github.com/markrai/scrumboy
- Confidence: high

### Agent Teams AI
- URL: https://agentteams.live
- Kind: agent-native
- What: An autonomous multi-agent orchestration desktop platform centered around a real-time Kanban board where agents independently claim tasks, message peers, and review each other's code. Supports over 300 LLMs and providers with zero-auth free model fallbacks.
- Agent integration: Built-in agent runners (Claude Code, Codex, OpenCode, Cursor CLI, Kiro, Grok), native MCP server, agent-to-agent protocol.
- Mobile: responsive web cockpit accessible over the local network; primary client is an Electron desktop app for macOS, Windows, Linux (cite: agentteams.live).
- Pricing: 100% Free open-source ($0).
- Self-host: yes, run locally as an Electron application or self-hosted Node.js server.
- License: AGPL-3.0
- Activity: 2,265 GitHub stars; latest release v2.17.10 on 2026-10-09; active development.
- Pros: Full multi-agent team orchestration centered on a visual kanban board; Cross-agent peer review loop before human verification; Support for 300+ models across 200+ providers; Native Electron client.
- Cons: Heavy local resource consumption when running multiple parallel agents; AGPL-3.0 license; No native mobile app for iOS or Android.
- Sources: https://agentteams.live, https://github.com/777genius/agent-teams-ai
- Confidence: high

### devBoard
- URL: https://getdevboard.app
- Kind: general-kanban-with-agent-integration
- What: A developer-focused project tracker and kanban board built for solo developers and small teams, offering native apps across iOS, Android, and macOS. Provides an official MCP server with 29 tools for Claude Code, Cursor, and VS Code.
- Agent integration: Official remote MCP server (https://api.getdevboard.app/mcp, 29 tools including `start_work`, `finish_work`, `log_time`, `move_item`, `create_release`), personal bearer token auth; supports Claude Code, Cursor, VS Code, Claude Desktop.
- Mobile: native iOS app (iPhone), native Android app, and native macOS desktop app (cite: getdevboard.app/mcp, App Store and Play Store listings).
- Pricing: Free while in early access as of 2026-10-16.
- Self-host: no (cloud-synchronized backend).
- License: Proprietary
- Activity: Active early access beta listed on Smithery; frequent client and MCP server releases in 2026.
- Pros: Native mobile apps on both iOS and Android along with a native macOS client; 29 specialized MCP tools including timer, time logging, and git releases; Free during early access; Fast native UI.
- Cons: Closed source SaaS; Self-hosting not available; Future commercial pricing terms not yet published.
- Sources: https://getdevboard.app, https://getdevboard.app/mcp, https://api.smithery.ai/servers/darko-bacic/devboard
- Confidence: high

### multidimensionalcats/kanban-mcp
- URL: https://github.com/multidimensionalcats/kanban-mcp
- Kind: agent-native
- What: A standalone database-backed kanban board and MCP server designed specifically for AI coding agents. Features a built-in lightweight web UI for humans, persistent SQLite/MySQL storage, and 40+ MCP tools for managing features, epics, blockers, and semantic search.
- Agent integration: Native MCP server (stdio and HTTP transports), 40+ agent tools for item hierarchies, decisions, and tag updates; supports Claude Code, Cursor, Windsurf, Codex.
- Mobile: responsive web UI accessible via local network browser (cite: GitHub README).
- Pricing: 100% Free and open source ($0).
- Self-host: yes, local Node.js/npx execution or Docker container with SQLite (default) or MariaDB/MySQL.
- License: MIT
- Activity: 83 GitHub stars; active updates through late 2026; listed on Glama.ai.
- Pros: 40+ purpose-built MCP tools covering items, decisions, epics, and work diaries; Embedded local ONNX semantic search; Zero external server dependencies with SQLite; Web UI for human inspection.
- Cons: Independent solo OSS project; No native mobile app; Lacks built-in multi-tenant access control.
- Sources: https://github.com/multidimensionalcats/kanban-mcp, https://glama.ai/mcp/servers/multidimensionalcats/kanban-mcp
- Confidence: high

### Kanera
- URL: https://www.kanera.app
- Kind: general-kanban-with-agent-integration
- What: A modern multi-board project management platform developed by Happen Software that connects kanban boards, portfolio overviews, and AI agents. Its official MCP server allows agents to create, update, and search cards and automations across workspaces.
- Agent integration: Official MCP server (kanera/kaneramcp) with 45+ tools across workspaces, boards, cards, and automations; supports Claude, ChatGPT, Codex, Cursor, Copilot, OpenCode, Gemini CLI.
- Mobile: responsive web (optimized for mobile web browsers, cite: kanera.app).
- Pricing: Free plan available; Pro plan is $5/user/month with a 30-day free trial as of 2026-10-16.
- Self-host: no (cloud-hosted service).
- License: Proprietary
- Activity: Active commercial product with verified Smithery MCP deployment; updated October 2026.
- Pros: Comprehensive multi-board portfolio views; Broad AI client support (Claude, Codex, Cursor, Gemini); Rich MCP toolset (45+ tools including bulk operations and automations); Competitive pricing ($5/user/mo).
- Cons: Proprietary cloud-only; No native iOS/Android mobile apps; Requires team accounts for advanced features.
- Sources: https://www.kanera.app, https://api.smithery.ai/servers/kanera/kaneramcp
- Confidence: high

### BoardMark
- URL: https://www.boardmark.ai
- Kind: general-kanban-with-agent-integration
- What: An agile issue and kanban board platform where every issue, sprint, and comment is backed by Markdown files. Its official MCP server enables AI agents to plan sprints, update delivery pipelines, and document work with the user's exact permissions.
- Agent integration: Official MCP server (i-c-develop/boardmark, 50+ tools), personal token and OAuth support; listed on Official MCP Registry, Smithery, Glama, and mcp.so; supports Claude, ChatGPT, Cursor.
- Mobile: responsive web (supports English and Thai languages, cite: boardmark.ai).
- Pricing: Free plan (฿0) for up to 2 users and 2 projects; Pro is ฿249/seat/month (~$7 USD, first 2 seats free); Premium is ฿329/seat/month (~$9.50 USD) as of 2026-10-16.
- Self-host: no (cloud-hosted platform by I C Develop Co., Ltd.).
- License: Proprietary
- Activity: Active production deployment; multi-registry presence (Official MCP registry, Smithery, Glama, mcp.so).
- Pros: Markdown-native issue and document architecture; Explicit AI agent attribution on every card update; Dual-language support (English/Thai); 2 free seats on Pro plan.
- Cons: Proprietary cloud SaaS; No native mobile applications; Pricing billed in Thai Baht (may cause FX overhead for global teams).
- Sources: https://www.boardmark.ai, https://www.boardmark.ai/pricing, https://api.smithery.ai/servers/i-c-develop/boardmark
- Confidence: high

### Kanban-md
- URL: https://github.com/antopolskiy/kanban-md
- Kind: agent-native
- What: A lightweight file-based kanban board CLI and TUI designed for autonomous AI agent loops. Stores tasks as human-readable Markdown files and bundles agent skills to enable multi-agent workflows directly from the terminal without cloud services.
- Agent integration: CLI commands, bundled AI agent skills (`skills/` directory for Claude Code/Codex), file-based watch events; direct terminal invocation.
- Mobile: none (pure CLI/TUI tool; can be accessed via mobile SSH/Termius, cite: GitHub README).
- Pricing: 100% Free and open source ($0).
- Self-host: yes, single Go binary running on local files; zero cloud or network dependencies.
- License: MIT
- Activity: 224 GitHub stars; latest release v0.39.0 on 2026-10-05; actively maintained.
- Pros: Git-native and file-based (zero database or external server needed); High-speed terminal TUI with interactive shortcuts; Built-in skills for autonomous agent loops; MIT licensed.
- Cons: No graphical web dashboard (TUI/CLI only); No native mobile app; Lacks multi-device real-time sync unless synced via git.
- Sources: https://github.com/antopolskiy/kanban-md
- Confidence: high

### Routa
- URL: https://phodal.github.io/routa/
- Kind: agent-native
- What: A workspace-first multi-agent coordination platform for software development featuring shared specifications, visual Kanban orchestration, and unified MCP, ACP (Agent Communication Protocol), and A2A support across web and desktop.
- Agent integration: Native MCP server, ACP, A2A (Agent-to-Agent) protocol, CLI (`routa-cli` on crates.io and npm); supports Claude Code, Cursor, OpenCode, Codex.
- Mobile: responsive web (mobile browser accessible, cite: GitHub README).
- Pricing: 100% Free and open source ($0).
- Self-host: yes, Docker Compose or Rust Axum + Next.js build; fully self-hostable.
- License: MIT
- Activity: 1,828 GitHub stars; latest release v0.19.0 on 2026-06-22; active commits in late 2026.
- Pros: Comprehensive multi-agent protocols (MCP + ACP + A2A); Built-in spec-driven development workflows tied to kanban cards; High performance Rust Axum backend with Next.js frontend; MIT licensed.
- Cons: Setup requires building or running multi-container stack; Desktop/web oriented with no native mobile app; Complex architecture for simple solo-agent tracking.
- Sources: https://phodal.github.io/routa/, https://github.com/phodal/routa
- Confidence: high

### OpenClaw Nerve
- URL: https://github.com/daggerhashimoto/openclaw-nerve
- Kind: agent-native
- What: A real-time web cockpit and mission control dashboard for OpenClaw agents, providing an automated kanban task board, voice conversations, workspace/file controls, sub-agent session monitoring, and usage visibility.
- Agent integration: Direct OpenClaw agent runtime integration, WebSocket event stream, MCP server bridge; supports OpenClaw, Claude, OpenAI agents.
- Mobile: responsive web UI accessible on tablets and mobile browsers (cite: GitHub README).
- Pricing: 100% Free and open source ($0).
- Self-host: yes, self-hosted web server with local configuration and Docker support.
- License: MIT
- Activity: 867 GitHub stars; active commits through 2026-10-13.
- Pros: Real-time visual mission control with live kanban task transitions; Voice interaction and audio status announcements; Sub-agent session tracing and token usage charts; MIT licensed.
- Cons: Closely coupled to the OpenClaw agent ecosystem; No native mobile app; Requires local Node/Python runtime setup.
- Sources: https://github.com/daggerhashimoto/openclaw-nerve
- Confidence: high

---

## EXPAND
- LEAD: Breakaway (breakaway-app/breakaway, npm package breakaway) — WHY: Mentioned in npm registry search as a task board for coding agents backed by Cloudflare Workers and Taskwarrior sync, but DNS for breakaway.sh was temporarily unresolved — ANGLE: Search GitHub and npm archives for breakaway-app/breakaway setup and MCP bridge
- LEAD: WrongStack Kanban MCP (@wrongstack/kanban-mcp) — WHY: Registered on npm with tiered read-only, writable, destructive, and live-watch capability tiers — ANGLE: Inspect @wrongstack/kanban-mcp package manifest on npmjs.org
- LEAD: Kanbaruu (kanbaruu.com) — WHY: Registered on Smithery with git two-way markdown sync, but main domain was temporarily unreachable — ANGLE: Search GitHub and Smithery archives for admin-6t83/kanbaruu repository mirrors
