# AI Agent Kanban & Task-Board Tools Survey: Awesome Lists Sweep

**Date**: 2026-10-16  
**Angle**: Awesome Lists (`awesome-claude-code`, `awesome-mcp-servers` [Product Management / Task sections], `awesome-selfhosted` [Task Management / Project Management sections], `awesome-ai-agents`)  
**Scope**: Discovery and profiling of less-known, agent-relevant, and agent-native kanban/task-board tools for monitoring AI agent work status.

Seen (obvious tools excluded from deep profiling): Trello, Jira, Linear, Notion, Asana, ClickUp, monday, Plane, WeKan, Kanboard, Planka, Vikunja, Focalboard, Taiga, Huly, Vibe Kanban, Task Master, Backlog.md.

---

### Nimbalyst
- URL: https://nimbalyst.com/
- Kind: agent-native
- What: An open-source visual workspace for building with Claude Code, Codex, and OpenCode that unifies sessions, git worktrees, task trackers, documents, diagrams, and code diffs. It features visual co-editing with coding agents alongside an integrated session and task kanban board.
- Agent integration: Native process orchestration, workspace API, and visual MCP client/server integration for Claude Code, OpenAI Codex CLI, OpenCode, and Gemini CLI; agents read and update project task trackers alongside human developers.
- Mobile: Native iOS companion app ("iOS companion: See which agents need you, reply by text or voice, swipe through diffs, queue the next task, and get a push when an agent needs help" - cite: GitHub README / nimbalyst.com); Android app companion cited in roadmap/README.
- Pricing: Free and open-source under MIT; zero per-seat fees or cloud subscriptions as of 2026-10-16.
- Self-host: Yes; local desktop application (packaged for macOS, Windows, Linux via GitHub releases) operating over local repository files and worktrees.
- License: MIT License
- Activity: 1,908 GitHub stars; last release v0.80.6 (2026-10-09); active commit pushed 2026-10-15; featured in awesome-claude-code.
- Pros: True visual WYSIWYG co-editing with red/green diff review; Native iOS companion app with push notifications and voice steering; Parallel multi-agent git worktree isolation per task card; Deep MCP integration rendering widgets instead of raw JSON.
- Cons: Desktop application footprint required on development workstation; Android companion still under development compared to the primary iOS release; Steeper learning curve than simple terminal log tailers.
- Sources: https://github.com/nimbalyst/nimbalyst, https://nimbalyst.com/, https://raw.githubusercontent.com/hesreallyhim/awesome-claude-code/main/README.md
- Confidence: high

---

### KaibanJS
- URL: https://www.kaibanjs.com/
- Kind: agent-native
- What: A JavaScript-native framework and visual board inspired by Kanban for building, managing, and observing multi-agent AI systems. It provides an interactive web UI ("Kaiban Board") to monitor AI agent states, task assignments, tool calls, and workflows in real time.
- Agent integration: Built-in JavaScript/TypeScript multi-agent runtime supporting OpenAI, Anthropic, Gemini, Groq, and local models; exposes React components (`<KaibanBoard />`) and event hooks for live agent state updates.
- Mobile: Responsive web dashboard built with React and Vite; no native iOS or Android app listed (cite: kaibanjs.com documentation and GitHub README).
- Pricing: Free and open-source under MIT; cloud interactive playground free as of 2026-10-16.
- Self-host: Yes; embeddable npm library (`kaibanjs`) running locally or deployed via Docker/Node.js web application.
- License: MIT License
- Activity: 1,480 GitHub stars; last release v0.24.1 (2026-05-01); commit pushed 2026-05-15; active Discord community.
- Pros: Native Kanban board metaphor purpose-built for AI agents and task lifecycles; JavaScript/TypeScript first (ideal for React/Node/Next.js developer stacks); Embeddable `<KaibanBoard />` component directly integratable into custom web apps; Human-in-the-loop task approval workflow.
- Cons: Focused on custom coded agents rather than out-of-the-box CLI harness for Claude Code/Codex; No dedicated native mobile app; Smaller ecosystem compared to Python-heavy frameworks like CrewAI/AutoGen.
- Sources: https://github.com/kaiban-ai/KaibanJS, https://www.kaibanjs.com/, https://raw.githubusercontent.com/e2b-dev/awesome-ai-agents/main/README.md
- Confidence: high

---

### Paca
- URL: https://paca-ai.org
- Kind: agent-native
- What: An AI-native, open-source project management platform where AI agents and human developers collaborate as equal teammates on the same Scrum boards, sprints, and backlogs. It serves as an agent-aware self-hosted alternative to Jira, Linear, and Trello.
- Agent integration: First-class MCP server (`paca-mcp`), REST API, webhooks, and WASM plugins; compatible with Claude Code, Cursor, Codex, and custom autonomous agents.
- Mobile: Responsive web application optimized for mobile browsers; PWA support; no official standalone native iOS/Android store app (cite: docs/mobile and GitHub README).
- Pricing: Free forever, 100% open-source under Apache-2.0 as of 2026-10-16.
- Self-host: Yes; single Docker container (`docker run -d -p 3000:3000 paca-ai/paca`) or Docker Compose / Kubernetes deployment.
- License: Apache-2.0
- Activity: 1,928 GitHub stars; last release v0.19.0 (2026-10-12); commit pushed 2026-10-15; featured in awesome-selfhosted.
- Pros: Treats AI agents as native Scrum team members with assigned cards and sprint velocity; Full MCP server allowing any agent to query backlogs and update card states; Self-contained single-container self-hosting; Zero per-seat fee or commercial upsell.
- Cons: Newer project still stabilizing advanced enterprise Scrum metrics and SSO; Lack of native mobile app store clients (relies on mobile browser/PWA); Requires configuring agent MCP credentials and prompts to adhere to Scrum conventions.
- Sources: https://github.com/Paca-AI/paca, https://paca-ai.org, https://raw.githubusercontent.com/awesome-selfhosted/awesome-selfhosted/master/README.md
- Confidence: high

---

### Vibeyard
- URL: https://github.com/elirantutia/vibeyard
- Kind: agent-native
- What: A cross-platform desktop IDE built specifically for AI coding agents that wraps Claude Code, Codex CLI, and Gemini CLI sessions with a swarm mode grid, project kanban board, and real-time session inspector. It provides visual task tracking, isolated multi-agent profiles, and P2P session sharing.
- Agent integration: Direct CLI and PTY wrapper for Claude Code, OpenAI Codex CLI, and Google Gemini CLI; agents execute in dedicated pseudo-terminals linked to visual kanban task cards.
- Mobile: None; desktop-only application for macOS, Windows, and Linux (cite: GitHub README).
- Pricing: Free and open-source under MIT; zero subscription fees as of 2026-10-16.
- Self-host: Yes; local desktop application (packaged dmg/exe/AppImage) with local-only storage and optional WebRTC P2P sharing.
- License: MIT License
- Activity: 1,387 GitHub stars; last release v0.3.8 (2026-09-03); active community; featured in awesome-claude-code.
- Pros: Integrated Kanban board directly mapped to parallel terminal sessions; Swarm mode grid to monitor multiple agent runs at a glance; Isolated Claude profiles preventing auth collisions; P2P encrypted session sharing via WebRTC without central servers.
- Cons: Desktop only with no mobile companion app or responsive web UI; Tightly coupled to specific CLI coding agents (Claude Code, Codex, Gemini); Customization options for board columns are relatively rigid.
- Sources: https://github.com/elirantutia/vibeyard, https://raw.githubusercontent.com/hesreallyhim/awesome-claude-code/main/README.md
- Confidence: high

---

### Claude Code Agent Monitor (CCAM)
- URL: https://hoangsonww.github.io/Claude-Code-Agent-Monitor/
- Kind: agent-native
- What: A real-time local-first control room and monitoring dashboard for Claude Code, Cursor, and Codex that visualizes agent hierarchies, tool calls, token spend, and workflow progress across a live Kanban status board. It operates via native Claude hooks and local SQLite storage.
- Agent integration: Claude Code native hooks, Cursor transcript discovery, companion MCP server, REST API, WebSocket live streaming, and OpenAPI 3.0 endpoints.
- Mobile: Responsive web dashboard accessible via mobile browser on local network; web-push notifications via VAPID (cite: README features and badges).
- Pricing: Free and open source under MIT; no paid tiers as of 2026-10-16.
- Self-host: Yes; local Node.js/Express service, Docker container (`docker run -p 3001:3001`), or packaged native desktop app (Electron for macOS/Windows).
- License: MIT License
- Activity: 1,080 GitHub stars; last release v2.3.0 (2026-10-10); commit pushed 2026-10-11; featured in awesome-claude-code.
- Pros: Real-time Kanban status board driven directly by agent lifecycle hooks; Deep observability into subagent hierarchies and token/cost burn; Native companion MCP server and VS Code extension; Multilingual documentation (EN, CN, VN, KO, ES).
- Cons: Primarily an observability and monitoring dashboard rather than an end-to-end task creation/sprint planning suite; No native iOS/Android app store client (uses responsive web + web push); Setup requires registering Claude Code hooks or running background server.
- Sources: https://github.com/hoangsonww/Claude-Code-Agent-Monitor, https://hoangsonww.github.io/Claude-Code-Agent-Monitor/, https://raw.githubusercontent.com/hesreallyhim/awesome-claude-code/main/README.md
- Confidence: high

---

### Routa
- URL: https://phodal.github.io/routa/
- Kind: agent-native
- What: A workspace-first multi-agent coordination platform for software delivery with shared specifications, visual Kanban orchestration, and comprehensive agent protocol support. It aligns autonomous agents across task decomposition, coding, review, and evidence collection.
- Agent integration: Multi-protocol agent gateway supporting MCP (Model Context Protocol), ACP (Agent Communication Protocol), A2A (Agent-to-Agent), AG-UI, REST, and SSE streams; supports Claude Code, Codex, and local LLMs.
- Mobile: Responsive web UI built on Next.js; no native mobile app listed (cite: GitHub README).
- Pricing: Free and open source under MIT; no commercial gate as of 2026-10-16.
- Self-host: Yes; self-hostable via Docker (`docker compose up`) or local desktop app built with Rust (Axum) and Next.js / Tauri.
- License: MIT License
- Activity: 1,828 GitHub stars; last release v0.19.0 (2026-06-22); commit pushed 2026-10-14; actively maintained.
- Pros: Rich multi-agent Kanban orchestration with built-in CRAFTER implementor/reviewer roles; Industry-standard protocol support (MCP, ACP, A2A); Strong trace and evidence collection model for auditability; Dual runtime flexibility (web service and desktop app).
- Cons: High architectural complexity due to multi-protocol surface; Requires familiarization with Routa specification conventions; Native mobile apps unavailable.
- Sources: https://github.com/phodal/routa, https://phodal.github.io/routa/, https://github.com/phodal/routa/blob/main/README.md
- Confidence: high

---

### Tirith
- URL: https://eabz.github.io/tirith/
- Kind: agent-native
- What: A lightweight coordination server and task board daemon for parallel coding agents that prevents agents from colliding through TTL-based file claims, interface contracts, and task dependency tracking over MCP.
- Agent integration: Dedicated Model Context Protocol (MCP) server over stdio or SSE/HTTP; integrates with Claude Code, Cursor, Codex, Pi, and any MCP client.
- Mobile: None; headless MCP daemon and CLI with web dashboard endpoints (cite: docs and README).
- Pricing: Free and open source under MIT; published to crates.io (`tirith-mcp`) at zero cost as of 2026-10-16.
- Self-host: Yes; single static Rust binary (`cargo install tirith-mcp` or GitHub release binaries) running on local host or container.
- License: MIT License
- Activity: 4 GitHub stars; last release v1.3.0 (2026-10-10); active repository in awesome-mcp-servers.
- Pros: Purpose-built for preventing parallel agent race conditions on shared repositories; Fast, zero-overhead Rust implementation; Built-in task board with dependency graph and auto-unblocking; Scoped memory notes linked to repository paths.
- Cons: Minimalist UI compared to full-blown web suites (focuses on MCP tools and simple status endpoints); Very recent project with smaller community; Requires agents configured to respect claims and contract conventions.
- Sources: https://github.com/eabz/tirith, https://eabz.github.io/tirith/, https://raw.githubusercontent.com/punkpeye/awesome-mcp-servers/main/README.md
- Confidence: high

---

### Saga MCP
- URL: https://www.npmjs.com/package/saga-mcp
- Kind: agent-native
- What: A Jira-like, SQLite-backed project and task tracking MCP server designed to give coding agents persistent memory of plans, epics, tasks, dependencies, and decisions across sessions. It includes a built-in one-call dashboard and local web kanban view.
- Agent integration: 41 MCP tools covering project management, task breakdown, status transitions, and dependency management; works out-of-the-box with Claude Code (`.mcp.json`), Cursor, and Claude Desktop.
- Mobile: Responsive local web dashboard accessible via browser; no native mobile app (cite: npmjs package docs).
- Pricing: Free and open source under MIT; available on npm (`npx -y saga-mcp`) with zero fees as of 2026-10-16.
- Self-host: Yes; local-first SQLite file stored in repository (`.saga/`) with zero cloud dependencies; runnable via npx.
- License: MIT License
- Activity: 40 GitHub stars; last release v1.19.0 (2026-09-29); commit pushed 2026-10-07; featured in awesome-mcp-servers.
- Pros: 41 granular MCP tools letting agents self-manage hierarchical backlogs and epics; Zero configuration and no external database needed (single SQLite file in project); Auto-blocking and dependency resolution prevents agents from starting premature work; One-call dashboard summary for quick agent context initialization.
- Cons: Local single-project orientation makes cross-team enterprise rollups manual; No native mobile app; Relies on LLM reasoning to invoke appropriate management tools consistently.
- Sources: https://github.com/spranab/saga-mcp, https://www.npmjs.com/package/saga-mcp, https://raw.githubusercontent.com/punkpeye/awesome-mcp-servers/main/README.md
- Confidence: high

---

### WorkBuddy Board
- URL: https://board-widget-download.app.workbuddy.host/
- Kind: agent-native
- What: A desktop task board and floating widget designed for MCP agents that displays a 4-column kanban board constantly visible on the desktop, automatically highlighting stalled tasks and logging append-only agent events.
- Agent integration: 15 MCP tools enabling agents to create, move, update, and inspect task cards; standalone MCP Python server and one-click MCP registration.
- Mobile: None; native Windows desktop floating widget application (cite: GitHub README).
- Pricing: Free and open-source under MIT as of 2026-10-16.
- Self-host: Yes; standalone single executable (`BoardWidget.exe`) with embedded local SQLite/JSON storage and zero external dependencies.
- License: MIT License
- Activity: 1 GitHub star; last release v1.1.0 (2026-10-08); listed in awesome-mcp-servers.
- Pros: Always-on-top desktop widget eliminates need to switch windows or check web browsers; Automated yellow highlighting for stalled agent tasks; Zero external dependencies (runs as a single executable); 15 tailored MCP tools for agent board manipulation.
- Cons: Windows-first executable packaging (macOS/Linux support requires running Python source directly); No mobile interface; Niche user base with single-developer maintenance.
- Sources: https://github.com/b52tot/workbuddy-board, https://board-widget-download.app.workbuddy.host/, https://raw.githubusercontent.com/punkpeye/awesome-mcp-servers/main/README.md
- Confidence: high

---

### Agent 007
- URL: https://github.com/bill10/agent-007
- Kind: agent-native
- What: A self-hosted virtual office and job board with a pixel-art UI where an AI manager agent ("Billion") breaks down goals, posts task cards to a board, dispatches Claude Code and Codex worker agents to isolated git worktrees, and reviews/merges pull requests.
- Agent integration: Direct headless CLI orchestration of Claude Code and OpenAI Codex; agents interact with the job board via internal dispatch APIs and worktree hooks.
- Mobile: Responsive web app accessible via mobile browser; features voice-call interaction ("Talk to Billion") from mobile phones (cite: GitHub README and video demo).
- Pricing: Free and open source under MIT as of 2026-10-16.
- Self-host: Yes; self-hosted Node.js / Python daemon running locally or on Linux/Windows servers.
- License: MIT License
- Activity: 5 GitHub stars; last release v0.69.4.4 (2026-10-11); actively developed; featured in awesome-claude-code.
- Pros: End-to-end autonomous manager-worker hierarchy with visual job board; Git worktree isolation per task prevents branch conflicts; Unique mobile voice-call interface to review progress on the go; Retro pixel-art office visualizer providing intuitive spatial status.
- Cons: High resource consumption running multiple CLI agents and worktrees simultaneously; Early-stage project with small contributor base; Voice call features require additional setup/API keys.
- Sources: https://github.com/bill10/agent-007, https://raw.githubusercontent.com/hesreallyhim/awesome-claude-code/main/README.md
- Confidence: high

---

### hubd
- URL: https://hubd.net
- Kind: agent-native
- What: A plain-file-based project tracker and coordination hub for mixed teams of humans and AI agents. It provides shared project cards, daily journals, agent work queues, and read-only kanban views with zero external dependencies.
- Agent integration: Native MCP server (`hubd mcp`), CLI tool (`hubd`), and filesystem hooks; agents read/write markdown and YAML files directly.
- Mobile: Responsive web view and plain markdown mobile compatibility; no dedicated native mobile app (cite: hubd.net docs).
- Pricing: Free and open-source under MIT; zero cloud fees as of 2026-10-16.
- Self-host: Yes; local CLI binary and filesystem storage (git-trackable directory); optional lightweight web server.
- License: MIT License
- Activity: 1 GitHub star; last release v0.9.54 (2026-10-06); commit pushed 2026-10-14; featured in awesome-mcp-servers.
- Pros: Boring-technology philosophy using plain text files that can be committed to git; Decoupled from agent runners (composes with Claude Code, Cursor, Aider, etc.); Role-based queues where agents wake on incoming tasks; Extremely fast and zero-infrastructure footprint.
- Cons: Read-only kanban presentation (interactive drag-and-drop requires editing files or CLI); Early release with minimal stars; Requires strict prompt discipline for agents to follow file schemas.
- Sources: https://github.com/bzdOS/hubd, https://hubd.net, https://raw.githubusercontent.com/punkpeye/awesome-mcp-servers/main/README.md
- Confidence: high

---

### It's a Plan (itsaplan)
- URL: https://itsaplan.dev/
- Kind: agent-native
- What: An open-source, self-hosted project management and issue tracking platform (alternative to Linear, Jira, and Plane) where AI agents work alongside humans as first-class teammates with dedicated roles, schedules, and full MCP connectivity.
- Agent integration: Official MCP server, REST API, and webhooks; agents can be scheduled or triggered to triage issues, write code, update tickets, and participate in sprint planning.
- Mobile: Fully responsive web interface optimized for mobile browsers and tablets; PWA installation supported (cite: itsaplan.dev / docs/self-hosting.md).
- Pricing: Free and open source (AGPL-3.0); optional self-hosted 1-click cloud templates (Railway); bring-your-own-keys for LLMs with no per-seat fees as of 2026-10-16.
- Self-host: Yes; Docker Compose (`docker compose up`), Docker images, and Railway deployment template.
- License: GNU Affero General Public License v3.0 (AGPL-3.0)
- Activity: 983 GitHub stars; last release v1.5.0 (2026-10-13); commit pushed 2026-10-15; rapid growth.
- Pros: Modern, polished Linear-like UX with native AI agent teammates; Comprehensive MCP server exposing full issue and board capabilities to agents; Built-in agent schedules, timelines, and automated ticket triaging; Self-hostable with one Docker Compose command.
- Cons: AGPL-3.0 copyleft license may restrict proprietary modifications; Rapid development means schema migrations between versions require care; Native iOS/Android app store client not yet released.
- Sources: https://github.com/croffasia/itsaplan, https://itsaplan.dev/, https://github.com/croffasia/itsaplan/blob/main/README.md
- Confidence: high

---

### Kandev
- URL: https://kandev.ai
- Kind: agent-native
- What: A specialized AI Kanban and parallel development environment designed for orchestrating multiple AI coding agents, reviewing diffs, and shipping code. It supports multiple LLM providers and offers full workflow customization without telemetry.
- Agent integration: Custom agent orchestration engine supporting Claude Code, OpenAI, Anthropic, Ollama, and custom scripts via `agentctl` CLI; provides REST API and WebSocket events.
- Mobile: Mobile remote-access guide provided using secure tunnels/WireGuard to access responsive web UI on mobile devices (cite: `docs/public/mobile-remote-access.md`).
- Pricing: Free and open-source under AGPL-3.0; zero telemetry; no subscription fee as of 2026-10-16.
- Self-host: Yes; single standalone binary (Rust/Tauri backend), Docker container, or systemd service.
- License: AGPL-3.0
- Activity: 954 GitHub stars; last release v0.97.0 (2026-10-04); commit pushed 2026-10-16; very active commit velocity.
- Pros: Purpose-built for parallel agent task execution and visual diff review; Pipeline and Kanban views tailored to agent development stages; Multi-provider support (Anthropic, OpenAI, local models) with zero telemetry; Single binary deployment without requiring Node.js on the server.
- Cons: AGPL-3.0 license; Focuses heavily on developer coding workflows rather than general business project management; Requires technical setup to expose remote mobile access.
- Sources: https://github.com/kdlbs/kandev, https://kandev.ai, https://github.com/kdlbs/kandev/blob/main/README.md
- Confidence: high

---

### 4ga Boards
- URL: https://4gaboards.com
- Kind: general-kanban
- What: A straightforward, lightweight realtime kanban board system for intuitive task tracking featuring collapsible todo lists, multitasking tools, and an elegant dark mode.
- Agent integration: REST API and webhook integrations; can be manipulated by AI agents via HTTP API tokens and external automation scripts.
- Mobile: Responsive web application accessible on mobile devices; no dedicated native mobile app (cite: 4gaboards.com).
- Pricing: Free open-source self-hosted version; optional managed hosting available (pricing varies by tier, commercial cloud available as of 2026-10-16).
- Self-host: Yes; Docker and Docker Compose (`docker compose up`) with Node.js and MongoDB backend.
- License: MIT License
- Activity: 728 GitHub stars; last release v3.3.13 (2026-07-15); commit pushed 2026-10-15; listed in awesome-selfhosted.
- Pros: Clean, distraction-free UI with dark mode and collapsible task lists; Realtime multi-user synchronization via WebSockets; Easy Docker Compose setup with MIT license; Lightweight resource footprint compared to Jira/Taiga.
- Cons: General-purpose kanban without built-in agent-specific abstractions (claims, agent roles, token tracking); No official MCP server published out-of-the-box; No native app store mobile applications.
- Sources: https://github.com/RARgames/4gaBoards, https://4gaboards.com, https://raw.githubusercontent.com/awesome-selfhosted/awesome-selfhosted/master/README.md
- Confidence: high

---

### Kaneo
- URL: https://kaneo.app/
- Kind: general-kanban
- What: An open-source, minimalist project management platform focused on simplicity and speed ("All you need. Nothing you don't"). It provides streamlined Kanban boards, task lists, and issue tracking without enterprise bloat.
- Agent integration: Full REST API (`https://kaneo.app/docs/api-reference`), GitHub/Gitea integrations, project webhooks; compatible with agents invoking HTTP endpoints or API wrappers.
- Mobile: Responsive web application; PWA support; no official iOS/Android store app (cite: kaneo.app docs).
- Pricing: Free and open-source under MIT; cloud hosted version offered with free tier as of 2026-10-16.
- Self-host: Yes; Docker, Docker Compose, and Kubernetes deployment options available.
- License: MIT License
- Activity: 9,494 GitHub stars; last release v2.46.2 (2026-10-12); commit pushed 2026-10-15; extremely popular in awesome-selfhosted.
- Pros: Minimalist, blazingly fast interface that eliminates project management bloat; Open REST API and webhooks allow straightforward AI agent automation; Strong open-source community (>9.4k stars) and MIT license; Clean Docker and Kubernetes deployment.
- Cons: General-purpose tool lacking agent-native concepts like LLM token metrics or execution traces; No official MCP server (requires custom REST API calls); Lacks native mobile app store clients.
- Sources: https://github.com/usekaneo/kaneo, https://kaneo.app/, https://raw.githubusercontent.com/awesome-selfhosted/awesome-selfhosted/master/README.md
- Confidence: high

---

### Super Productivity
- URL: https://super-productivity.com
- Kind: general-kanban-with-agent-integration
- What: An advanced todo list, timeboxing, and personal kanban app that integrates with Jira, GitHub, GitLab, and OpenProject, offering rich time tracking and productivity analytics.
- Agent integration: Local Sync Server REST API, WebSockets, CLI integration, and plugin architecture; community scripts and agent tools can create/update tasks and track time.
- Mobile: Native Android app on Google Play & F-Droid, iOS app on Apple App Store, and responsive PWA (cite: GitHub README & super-productivity.com).
- Pricing: 100% Free and open-source forever under MIT; no subscriptions or ads as of 2026-10-16.
- Self-host: Yes; local desktop application (macOS, Linux, Windows), Docker container for sync server, or browser web app.
- License: MIT License
- Activity: 22,859 GitHub stars; last release v19.2.0 (2026-10-11); commit pushed 2026-10-15; mature and highly active; listed in awesome-selfhosted.
- Pros: Native mobile apps across Android (Play Store, F-Droid) and iOS (App Store); Rich timeboxing, Pomodoro, and time-tracking features; Excellent integration with external trackers (Jira, GitHub, GitLab); 100% free, MIT licensed, with optional private sync server.
- Cons: Personal productivity focus rather than multi-user team agile board; Kanban view is a mode within a broader todo/planner workflow; Requires REST/Plugin bridge for deep AI agent coordination.
- Sources: https://github.com/super-productivity/super-productivity, https://super-productivity.com, https://raw.githubusercontent.com/awesome-selfhosted/awesome-selfhosted/master/README.md
- Confidence: high

---

### Tasks.md
- URL: https://github.com/BaldissaraMatheus/Tasks.md
- Kind: general-kanban
- What: A self-hosted, Markdown-file-based task management board with a modern web interface where cards are stored as plain markdown files on disk.
- Agent integration: File-based integration; AI agents can directly read, write, and manipulate markdown card files in the storage directory, plus webhooks.
- Mobile: Responsive web interface and installable PWA (cite: GitHub README).
- Pricing: Free and open-source under MIT as of 2026-10-16.
- Self-host: Yes; single Docker container (`baldissaramatheus/tasks.md`) with file mounts.
- License: MIT License
- Activity: 2,203 GitHub stars; last commit pushed 2026-03-08; active in awesome-selfhosted.
- Pros: Plain Markdown files on disk allow agents to edit tasks with standard file tools; Single Docker container deployment; PWA support for mobile home screens; Clean theme options (Nord, Catppuccin, Adwaita).
- Cons: General kanban without native MCP server or agent-specific telemetry; Slower release cadence than newer agent boards; File sync race conditions if multiple agents edit the same card simultaneously without locking.
- Sources: https://github.com/BaldissaraMatheus/Tasks.md, https://raw.githubusercontent.com/awesome-selfhosted/awesome-selfhosted/master/README.md
- Confidence: high

---

### Framedeck MCP
- URL: https://framedeck.app
- Kind: general-kanban-with-agent-integration
- What: An official MCP server for Framedeck, a Kanban-based content production pipeline platform for video creators (YouTube, TikTok, podcasts) that connects AI assistants to production stages, cards, checklists, and comments.
- Agent integration: Official MCP server (`npx -y framedeck-mcp`), REST API; compatible with Claude Code, Claude Desktop, ChatGPT, Gemini, and Copilot.
- Mobile: Responsive web application on framedeck.app; no native mobile app listed (cite: framedeck.app / GitHub README).
- Pricing: Free MCP package on npm; Framedeck SaaS offers free tier with paid plans for professional teams as of 2026-10-16.
- Self-host: No (Framedeck is cloud SaaS; MCP connector runs locally as stdio bridge).
- License: MIT License (for MCP connector); Proprietary (Framedeck core).
- Activity: 1 GitHub star; commit pushed 2026-04-08; featured in awesome-mcp-servers Product Management section.
- Pros: Purpose-built for media and content production workflows on Kanban stages; Official MCP server supporting Claude Code and major AI clients; Comprehensive card management with checklists and stage transitions; Bridges creative direction prompts directly to production boards.
- Cons: Core platform is closed-source cloud SaaS (not self-hostable); Tailored to media/video production rather than software engineering sprints; Small open-source connector footprint.
- Sources: https://github.com/Lukaris/framedeck-mcp, https://framedeck.app, https://raw.githubusercontent.com/punkpeye/awesome-mcp-servers/main/README.md
- Confidence: high

---

## EXPAND
- LEAD: yetanotherorchestrator.app & Chorus (Chorus-AIDLC) — WHY: Discovered during agent orchestration landscape review as emerging multi-agent team boards, but they focus on conversational prompt choreography and AI-DLC cycles rather than standalone visual Kanban status boards — ANGLE: Search GitHub for "AI-DLC" and multi-agent team choreography boards.
- LEAD: CLI Agent Remote Notifiers (anotifier, ntfy, Happy Coder) — WHY: While not traditional task boards, these tools solve the mobile awareness problem for running agents by streaming real-time status transitions and approval prompts to mobile push notifications and voice calls — ANGLE: Search awesome-claude-code for "Remote Control, Notifications & Voice I/O".
- LEAD: Local-First Markdown Kanban Extensions (obsidian-pm, dotpm) — WHY: Local-first Markdown kanban boards embedded in personal knowledge bases offer local REST APIs and direct filesystem access that AI agents can inspect and manipulate without cloud overhead — ANGLE: Search awesome-selfhosted and Obsidian community plugins for "local-first agent task tracking".
