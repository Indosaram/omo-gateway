# Kanban MCP Servers: AI Agent Task & Work Status Survey

> Survey Date: 2026-10-16  
> Focus: MCP (Model Context Protocol) servers that expose Kanban boards to AI coding agents (Claude Code, Codex, Cursor, Gemini, OpenCode, etc.), including dedicated agent-native boards, middleware bridges to self-hosted OSS Kanban platforms (Planka, WeKan), and bridges to commercial SaaS Kanban tools (Trello).

---

### kanban-mcp (multidimensionalcats)
- URL: https://github.com/multidimensionalcats/kanban-mcp
- Kind: agent-native
- What: A Python-based database-backed Kanban board built natively for AI coding agents via MCP, featuring 40+ agent tools, SQLite/MySQL backends, local ONNX semantic search, and a human-facing web UI.
- Agent integration: MCP server over stdio; 40+ specialized tools covering issues, epics, todos, diary entries, decisions, and file links; supports Claude Code, Cursor, Codex, OpenCode, and any MCP client; includes session hooks to inject active tasks into agent context.
- Mobile: responsive web only (browser-based board at localhost:5000; no dedicated iOS/Android native app or PWA manifest cited).
- Pricing: free / open-source free (no paid tiers or seat limits as of 2026-10-16).
- Self-host: yes; local CLI/Python environment via pipx (`pipx install kanban-mcp`), SQLite default file or Docker Compose with MySQL/MariaDB; fully self-hosted with zero external cloud dependencies.
- License: MIT
- Activity: 83 GitHub stars; latest commit on 2026-03-17 (v0.2.0); active project.
- Pros: Built from the ground up for agent workflows rather than retrofitting a human tool; local ONNX vector search over tasks without API keys; 40+ granular agent tools including epics and dependency management; zero-dependency SQLite setup out of the box.
- Cons: Mobile experience is limited to localhost responsive web; no native multi-user permission controls in SQLite mode; semantic search requires ~140MB model download on initial query; smaller community compared to established human project managers.
- Sources: https://github.com/multidimensionalcats/kanban-mcp
- Confidence: high

---

### Planka MCP (gogogadgetbytes)
- URL: https://github.com/gogogadgetbytes/planka-mcp
- Kind: general-kanban-with-agent-integration
- What: An official-style Node.js MCP server bridging AI agents directly to Planka 2.0+ self-hosted Kanban boards via Planka's REST API.
- Agent integration: MCP server via npx (`@gogogadgetbytes/planka-mcp`) over stdio; 13 structured tools for card creation, hierarchy inspection, task checklist updates, comments, and card movement across lists; tested with Claude Code and Claude Desktop.
- Mobile: responsive web on Planka core (native mobile support introduced in Planka 2.5.0; no standalone mobile app for the MCP bridge itself; accessed via Planka web UI on mobile browsers).
- Pricing: free for the MCP bridge (MIT); upstream Planka has free open-source Community edition, Pro self-hosted from €36/month (billed annually, €7.20/user/month), and Hosted Pro from €42.50/month (billed annually, €8.50/user/month) as of 2026-10-16.
- Self-host: yes; Planka core is deployed via Docker / Docker Compose / Kubernetes; the MCP server runs locally via npx with `PLANKA_BASE_URL` and agent credentials.
- License: MIT (MCP server); Planka Community edition is under Fair Use / Community License with commercial Pro licenses.
- Activity: 11 GitHub stars; latest commit 2026-02-03 (v1.0.0); stable.
- Pros: Integrates with Planka's full-featured multi-user Kanban board and UI; type-safe implementation using Zod; simple zero-build execution with `npx @gogogadgetbytes/planka-mcp`; provides structured project hierarchy navigation.
- Cons: Requires an already running and reachable Planka instance; limited to Planka REST API capabilities (no real-time WebSocket/SSE event push to agents); agent actions require dedicated user credentials.
- Sources: https://github.com/gogogadgetbytes/planka-mcp, https://planka.app, https://planka.app/pricing, https://github.com/plankanban/planka
- Confidence: high

---

### Kanban MCP (bradrisse)
- URL: https://github.com/bradrisse/kanban-mcp
- Kind: general-kanban-with-agent-integration
- What: A TypeScript MCP middleware bridge connecting Cursor and Claude to Planka Kanban boards, with bundled Docker Compose for spinning up a complete Planka environment.
- Agent integration: MCP server over stdio; tools for projects, boards, lists, cards, checklist tasks, card moving, and time tracking; explicitly documented for Cursor (`.cursor/mcp.json`) and Claude Desktop.
- Mobile: responsive web through Planka's web interface (no dedicated native mobile apps).
- Pricing: free / open-source free (upstream Planka Community edition is free self-hosted).
- Self-host: yes; includes Docker Compose configuration to start Planka and PostgreSQL locally, with the Node.js MCP server connecting over localhost:3333.
- License: MIT
- Activity: 61 GitHub stars; latest commit 2025-03-26; stable baseline.
- Pros: Bundles ready-to-run Docker Compose for quick local Planka testing; explicit setup guides and `.cursor/mcp.json` templates; covers time tracking and card task checklists.
- Cons: Maintenance slower than newer Planka 2.x specific bridges; requires running Planka container stack with PostgreSQL; no prebuilt npm package (requires local clone and build).
- Sources: https://github.com/bradrisse/kanban-mcp, https://planka.app
- Confidence: high

---

### MCP Server Trello (delorenj)
- URL: https://github.com/delorenj/mcp-server-trello
- Kind: general-kanban-with-agent-integration
- What: A high-performance TypeScript/Bun-powered MCP server providing AI agents with 57 comprehensive tools to manage Atlassian Trello boards, cards, checklists, and workflows.
- Agent integration: MCP server registered on official Model Context Protocol Registry (`io.github.delorenj/mcp-server-trello`); runs via `bunx`/`npx @delorenj/mcp-server-trello`; provides 57 tools including native acceptance criteria extraction, checklist dependency queries, card attachments, and card/list watchers; verified for Claude Code, Cursor, and Claude Desktop; includes BMAD agent skill package.
- Mobile: native iOS app and native Android app available for Trello (via Apple App Store and Google Play Store); responsive web and desktop apps available.
- Pricing: MCP bridge is 100% free (MIT); Trello pricing: Free plan ($0 up to 10 collaborators/workspace), Standard ($5/user/month billed annually or $6 monthly), Premium ($10/user/month billed annually or $12.50 monthly), Enterprise ($17.50/user/month billed annually) as of 2026-10-16.
- Self-host: no for Trello itself (cloud SaaS owned by Atlassian); yes for the MCP server execution (runs locally on user machine via Bun/Node).
- License: MIT
- Activity: 446 GitHub stars; latest release v2.0.0-beta.0 (2026-08-04), latest commit 2026-10-15; highly active and widely used.
- Pros: Most mature and feature-complete Kanban MCP server with 57 tools; built-in API rate limiting handling Trello's 300 req/10s token limits; official MCP Registry listing; native iOS and Android apps through Trello platform; includes specialized acceptance-criteria intelligence tools.
- Cons: Relies on external proprietary SaaS (Trello/Atlassian); requires Trello API key and manual token generation; subject to Trello free tier limits (10 collaborators per workspace).
- Sources: https://github.com/delorenj/mcp-server-trello, https://trello.com/pricing, https://registry.modelcontextprotocol.io
- Confidence: high

---

### Trello MCP Server (m0xai)
- URL: https://github.com/m0xai/trello-mcp-server
- Kind: general-kanban-with-agent-integration
- What: A lightweight Python-based MCP server built with `mcp[cli]` and `httpx` to manage Trello boards, lists, and cards from AI host applications.
- Agent integration: MCP server over stdio using Python 3.12+ and `uv`; basic board operations, list creation/retrieval, and card CRUD; compatible with Claude Desktop and Claude Code.
- Mobile: native iOS and Android apps available via the official Trello mobile apps; responsive web.
- Pricing: MCP server is free; Trello is free up to 10 collaborators, Standard at $5/user/month, Premium at $10/user/month as of 2026-10-16.
- Self-host: no for Trello core (SaaS); MCP bridge runs locally via `uv` or Python virtual environment.
- License: MIT (per repository classification; no explicit root LICENSE file committed).
- Activity: 54 GitHub stars; latest commit on 2026-06-18; stable maintenance.
- Pros: Simple Python implementation for environments where Node/Bun is undesirable; straightforward configuration via uv; clean minimal tool footprint.
- Cons: Significantly fewer specialized tools than delorenj's 57-tool implementation; lacks advanced workflow features like acceptance criteria parsing or automated rate-limiting backoff.
- Sources: https://github.com/m0xai/trello-mcp-server, https://trello.com/pricing
- Confidence: high

---

### WeKan MCP Server (namar0x0309)
- URL: https://github.com/namar0x0309/wekan-mcp
- Kind: general-kanban-with-agent-integration
- What: A Node.js/TypeScript Model Context Protocol server providing AI agents with direct programmatic access to self-hosted WeKan Kanban boards, swimlanes, and cards.
- Agent integration: MCP server via stdio; tools include `listBoards`, `listLists`, `listSwimlanes`, `listCards`, `createCard` (with due dates, labels, members, swimlanes), `moveCard`, and `commentCard`; includes automated PowerShell and Bash token generation scripts.
- Mobile: responsive web on WeKan (progressive web app support in modern browsers; unofficial mobile wrappers; no official native iOS/Android app from WeKan core).
- Pricing: free / open-source free; WeKan is 100% FLOSS under MIT; optional commercial priority support is available directly from WeKan's Finnish maintainer (Lauri Ojansivu) starting from €50/year SaaS or €100/incident as of 2026-10-16.
- Self-host: yes; WeKan is self-hosted via Docker, Snap, Source, or Cloudron with MongoDB; the MCP server runs locally via Node.js connecting to the WeKan REST API.
- License: ISC (package.json) / MIT for upstream WeKan.
- Activity: 6 GitHub stars; latest commit 2026-06-28; stable.
- Pros: Connects to a completely free, telemetry-free, 100% open-source Kanban system; supports WeKan's swimlanes (2D board layout); includes helper scripts for automated API token extraction.
- Cons: Small community adoption for the MCP bridge; limited to standard card/list CRUD; lacks advanced agent collaboration features (e.g. atomic card claiming or task dependencies).
- Sources: https://github.com/namar0x0309/wekan-mcp, https://wekan.fi, https://wekan.fi/commercial-support/, https://github.com/wekan/wekan
- Confidence: high

---

### WeKan MCP Server (omriza5)
- URL: https://github.com/omriza5/wekan-mcp-server
- Kind: general-kanban-with-agent-integration
- What: A Python/FastAPI microservice MCP server that provides AI chat interfaces like Claude Desktop with read access to WeKan users, boards, and cards.
- Agent integration: MCP server over HTTP/FastAPI (Uvicorn) or stdio; tools to query WeKan instance users, list boards, and fetch cards for a board; works with Claude Desktop and HTTP-capable MCP clients.
- Mobile: responsive web through WeKan web client.
- Pricing: free / open-source free (WeKan is MIT licensed).
- Self-host: yes; connects directly to MongoDB (`MONGO_URL=mongodb://localhost:27017/wekan`) or runs in Docker alongside WeKan.
- License: MIT
- Activity: 0 GitHub stars; latest commit 2025-10-02; proof-of-concept.
- Pros: Direct Python/FastAPI architecture with direct MongoDB database connection option; Dockerfile and docker-compose included.
- Cons: Read-only functionality (no card creation, updating, or movement tools implemented); dormant development; zero community adoption.
- Sources: https://github.com/omriza5/wekan-mcp-server, https://wekan.fi
- Confidence: medium (verified code files and commit history directly).

---

### Flux (sirsjg)
- URL: https://github.com/sirsjg/flux
- Kind: agent-native
- What: An open, hackable, Git-native and execution-agnostic Kanban board engine designed for humans and AI agents, where tasks live in a single JSON file synced via Git and are accessible via MCP, CLI, REST, and web UI.
- Agent integration: Built-in MCP server (`packages/mcp`) supporting stdio and Streamable HTTP; 25+ tools covering projects, epics, tasks, priorities (P0/P1/P2), dependencies, agent memory comments, and file blob attachments; includes dedicated Claude Code CLI plugin (`claude mcp add flux`) and Codex support.
- Mobile: responsive web UI (Preact-powered frontend running on localhost:3000; works on mobile web browsers; no native iOS/Android apps).
- Pricing: free / open-source free (MIT license, self-hosted, no paid subscriptions).
- Self-host: yes; single JSON file backing, Docker quickstart (`curl -fsSL ... | bash`), or standalone Bun/Node execution; git-native sync via `flux push`/`flux pull` on a `flux-data` branch.
- License: MIT
- Activity: 104 GitHub stars; latest release v0.6.0 (2026-08-27), latest commit 2026-09-09; active and modern.
- Pros: Git-native persistence prevents database sprawl and works seamlessly in CI/CD and offline repos; explicit AI-native design with agent memory comments, acceptance criteria, and task blocker dependencies; dual stdio and Streamable HTTP MCP support; clean CLI tool with full MCP parity.
- Cons: File-based JSON model may experience concurrency merge conflicts on large multi-agent teams without frequent git sync; no native mobile notifications or push alerts.
- Sources: https://github.com/sirsjg/flux
- Confidence: high

---

### Collattice (MrBildo)
- URL: https://github.com/MrBildo/collattice
- Kind: agent-native
- What: A lightweight, self-hosted Kanban board purpose-built for human-agent collaboration as a single executable with embedded SQLite, real-time Server-Sent Events (SSE), and an integrated Streamable HTTP MCP server.
- Agent integration: Built-in Streamable HTTP MCP endpoint directly embedded in the single binary; AI agents (Claude Code, Cursor, Codex) create cards, move lanes, comment, tag labels, archive, search, and manage attachments; changes instantly stream to human browsers via SSE.
- Mobile: responsive web (modern responsive browser layout with dark/light mode; no native mobile app).
- Pricing: free / open-source free (MIT license; zero paid tiers).
- Self-host: yes; single precompiled binary (.NET 10 / Node 22 runtime embedded) with embedded SQLite database; zero external database or container requirements; one-line curl/powershell installer.
- License: MIT
- Activity: 8 GitHub stars; latest release v3.2.3 (2026-10-01), latest commit 2026-10-01; actively maintained.
- Pros: True single-binary zero-config deployment without Docker or database setup; real-time SSE updates allow human users to watch AI agents move cards live without page refreshes; built-in outbound webhooks with HMAC signing; rich Markdown and Mermaid diagram rendering.
- Cons: Relatively new project with a small user base; lacks complex multi-project portfolio management; single-instance architecture not designed for distributed enterprise clusters.
- Sources: https://github.com/MrBildo/collattice
- Confidence: high

---

### LokalBoards (florian-strasser)
- URL: https://github.com/florian-strasser/LokalBoards
- Kind: general-kanban-with-agent-integration
- What: An open-source, self-hosted Nuxt/Vue Kanban board system with real-time multiplayer Socket.IO updates, enterprise SSO, and an integrated MCP server (`/mcp`) with atomic card claiming for AI agents.
- Agent integration: Native built-in MCP server accessible at `/mcp` with per-user/per-agent API keys or OAuth; agents can search, filter, create, move, comment, and assign cards; features dedicated 'AI agent' user accounts with bot badges and atomic card claiming to prevent human/agent work collisions.
- Mobile: responsive web (Nuxt-powered responsive web interface with multi-language support; no native mobile store app).
- Pricing: free / open-source free (MIT license; includes full SSO and all features with no paid enterprise edition).
- Self-host: yes; self-hosted on Node.js 22+ with MySQL/MariaDB database; Docker support available.
- License: MIT
- Activity: 17 GitHub stars; latest release v0.40.2 (2026-10-06), latest commit 2026-10-05; actively maintained.
- Pros: Atomic card claiming specifically solves race conditions between human and AI agent workers; enterprise SSO (OIDC/SAML) included for free; real-time Socket.IO multiplayer presence shows which human or agent currently has a card open; granular webhook triggers on board events.
- Cons: Requires MySQL database setup; higher operational footprint than single-binary SQLite alternatives; mobile experience is web-only.
- Sources: https://github.com/florian-strasser/LokalBoards
- Confidence: high

---

### Taskboard (tcarac)
- URL: https://github.com/tcarac/taskboard
- Kind: agent-native
- What: A local-first, self-hosted project management tool featuring a Kanban web UI, full CLI, embedded terminal for running AI agents, and a built-in 22-tool MCP server.
- Agent integration: Built-in MCP server (`taskboard mcp`); 22 tools covering projects, tickets, teams, priority levels, labels, subtasks, and dependencies ('blocked by'); directly integrates with Claude Code CLI and Claude Desktop; includes an embedded web terminal to execute AI coding agents (Claude Code, opencode) from the Kanban UI.
- Mobile: responsive web UI on localhost (no native mobile app).
- Pricing: free / open-source free (MIT license).
- Self-host: yes; single Go binary backed by embedded SQLite; installable via Homebrew (`brew install taskboard`) or source compilation with zero Docker requirement.
- License: MIT
- Activity: 37 GitHub stars; latest release v0.6.0 (2026-02-16); stable.
- Pros: Embedded terminal in the Kanban web UI allows launching Claude Code or coding agents directly from the board; single binary with zero external dependencies via Homebrew; full CLI and MCP tool parity; built-in task dependency handling.
- Cons: Designed primarily for local single-developer workflows rather than distributed team collaboration; no remote mobile push or native app.
- Sources: https://github.com/tcarac/taskboard
- Confidence: high

---

### Moo Tasks (dizlexic)
- URL: https://github.com/dizlexic/moo-tasks
- Kind: agent-native
- What: A multi-user, multi-board Kanban task management dashboard built with Nuxt 4 and Drizzle ORM featuring per-board MCP server endpoints for AI agent task discovery and execution.
- Agent integration: Per-board MCP server endpoints with Bearer token authentication and Public/Private toggles; agents discover, accept, update, and create tasks; includes configurable agent instructions, linked child correction tasks, and AI changelog generation.
- Mobile: responsive web (Nuxt 4 / Tailwind CSS v4 responsive layout; no native mobile app).
- Pricing: free / open-source free (MIT license).
- Self-host: yes; Docker Compose deployment (`docker compose up -d`) with MySQL database, Adminer, and Mailpit.
- License: MIT
- Activity: 45 GitHub stars; latest commit on 2026-06-21; active.
- Pros: Multi-user collaboration with invite system and member roles; specialized 'Correction Tasks' mechanism allowing humans to request specific fixes via linked child tasks; per-board MCP endpoint security with customizable system instructions for agents.
- Cons: Requires MySQL and Docker infrastructure; polling-based real-time sync rather than native WebSocket/SSE; mobile experience is browser-only.
- Sources: https://github.com/dizlexic/moo-tasks
- Confidence: high

---

### kanban-mcp (eyalzh)
- URL: https://github.com/eyalzh/kanban-mcp
- Kind: agent-native
- What: A lightweight Node.js/TypeScript MCP server that gives AI agents structured Kanban-based task management memory for multi-session coding workflows, backed by SQLite with an optional web UI.
- Agent integration: MCP server over stdio; tools to create boards, define columns, manage tasks, enforce WIP limits, and persist state across AI sessions; supports predefined prompts for planning and task resumption in Claude Desktop.
- Mobile: responsive web via local web server (http://localhost:8221; no native mobile app).
- Pricing: free / open-source free (MIT license).
- Self-host: yes; local Node.js process or Docker container with SQLite database stored in any local directory.
- License: MIT
- Activity: 41 GitHub stars; latest commit on 2025-07-13; stable baseline.
- Pros: Implements Kanban WIP (Work-In-Progress) limits for agent tasks; includes companion web UI for human observation; lightweight embedded SQLite storage.
- Cons: Older repository without recent updates; UI requires building and running a separate web server package; basic toolset compared to newer 2026 alternatives.
- Sources: https://github.com/eyalzh/kanban-mcp
- Confidence: high

---

## EXPAND
- LEAD: Routa (phodal/routa) — WHY: 1,828 GitHub stars workspace-first multi-agent coordination platform featuring Kanban orchestration and unified MCP/ACP/A2A protocols for parallel development across web and desktop. — ANGLE: Search GitHub for 'phodal/routa' architecture, MCP tool specifications, and mobile/web deployment models.
- LEAD: Open-Sunsama (ShadowWalker2014/open-sunsama) — WHY: 99 stars open-source daily planner with Kanban boards, time blocking, one-click MCP connector for Claude and ChatGPT, and mobile apps. — ANGLE: Search for 'ShadowWalker2014/open-sunsama' mobile app store availability and MCP schema.
- LEAD: Claude Code Studio (Lexus2016/claude-code-studio) — WHY: 144 stars dedicated web workspace for Claude Code CLI featuring live Kanban task boards, remote mobile access via Telegram/Web, and multi-agent orchestration. — ANGLE: Search repository 'Lexus2016/claude-code-studio' for Kanban MCP synchronization mechanisms.
