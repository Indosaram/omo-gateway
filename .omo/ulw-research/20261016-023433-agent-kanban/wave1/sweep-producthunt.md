# Product Hunt & Launch Sites Sweep: AI Agent Kanban & Task Boards (2025–2026)

Seen: Trello, Jira, Linear, Notion, Asana, ClickUp, monday, Plane, WeKan, Kanboard, Planka, Vikunja, Focalboard, Taiga, Huly, Vibe Kanban, Task Master, Backlog.md

### AgentKanban
- URL: https://agentkanban.io
- Kind: agent-native
- What: A dedicated Kanban board and context management system built for agent-assisted engineering, connecting cloud task boards to VS Code and CLI agents. It formalizes a Plan / TODO / Implement workflow and coordinates shared project context like AGENTS.md, memory notes, and Git worktrees.
- Agent integration: Official MCP server (supports Claude Code, Codex, Cursor), companion VS Code extension, REST API, webhooks.
- Mobile: responsive web (hosted web dashboard accessible via mobile browser); no native mobile app.
- Pricing: Free tier ($0 forever for 3 members, 3 boards, 30 tasks, 1 API key, 1 webhook, 250 API req/day); Pro tier at $10/seat/month (unlimited boards, tasks, members, API requests) as of October 2026.
- Self-host: no (cloud-hosted service for remote collaborative boards; local-only predecessor extension operates on local markdown files).
- License: proprietary cloud service; VS Code extension under Elastic License 2.0 (ELv2).
- Activity: Active 2026 release cycle; updated companion VS Code extension on Marketplace and Open VSX.
- Pros: First-class Plan/TODO/Implement workflow designed around agent failure modes; Seamless bridge between web board and VS Code; Git worktree automation per card; Built-in AGENTS.md and memory synchronization.
- Cons: Cloud service is proprietary with free-tier rate limits (250 req/day); Core board sync is not open-source self-hostable; No native iOS/Android application.
- Sources: https://agentkanban.io, https://agentkanban.io/pricing, https://github.com/appsoftwareltd/vscode-agent-kanban, https://www.appsoftware.com/blog/introducing-vs-code-agent-kanban-task-management-for-the-ai-assisted-developer
- Confidence: high

### QodFlow
- URL: https://www.qodflow.com
- Kind: agent-native
- What: An AI-native collaborative Kanban board where human teammates and AI agents work the exact same cards. Designed so that agents communicate their work status natively on the board, eliminating the need for separate progress check-ins.
- Agent integration: Native MCP server integration supporting Claude Code, Cursor, OpenCode, and any MCP-compliant agent client; REST API.
- Mobile: responsive web (features client QR-code scanning optimized for mobile browsers without requiring a login); no native mobile app.
- Pricing: Free permanent tier (up to 3 users, 10 active jobs, job assignment, priority flags, no credit card required); Premium/Diamond paid plans with per-stage SLAs and custom tags (launch discount 50% off with promo code) as of October 2026.
- Self-host: no (US-hosted cloud SaaS, TLS encrypted at rest).
- License: proprietary.
- Activity: Active commercial release in 2026; live demo and public production deployment.
- Pros: True human-agent parity on the same board cards; Native MCP endpoint requires zero-config setup; Client-facing QR status tracking without login; Clean SLA tracking and risk-color coding.
- Cons: Proprietary cloud-only architecture (no self-hosting); Strict 10 active job limit on the free tier; No native mobile app for offline/push management.
- Sources: https://www.qodflow.com, https://www.qodflow.com/pricing
- Confidence: high

### is.team
- URL: https://is.team
- Kind: agent-native
- What: A spatial infinite canvas where AI agents join the board as real teammates alongside human staff. Users drag and arrange Kanban boards, documents, and notes, while agents subscribe and participate in card chat via MCP.
- Agent integration: MCP server connectivity for Claude, Cursor, and ChatGPT; HMAC-signed webhooks; GitHub, Slack, and Google Calendar integrations.
- Mobile: responsive web (infinite spatial canvas with touch navigation and mobile meta viewport); no native mobile app.
- Pricing: Free tier ($0/month, up to 3 seats, unlimited tasks); flat workspace pricing starting at Starter ($8/mo early adopter, $40/mo regular), Pro ($16/mo flat for 15 seats, $80/mo regular), Max ($39/mo, $195/mo regular) as of October 2026.
- Self-host: no (cloud-hosted multi-tenant SaaS, US servers, encrypted in transit).
- License: proprietary.
- Activity: Active 2026 launch on Product Hunt (`is-team`); frequent updates and active early adopter tier.
- Pros: Innovative spatial infinite canvas combined with Kanban boards; Flat workspace pricing instead of punitive per-seat licensing; AI agents chat directly inside task cards via MCP; Built-in voice rooms and real-time collaboration.
- Cons: Canvas-heavy spatial interface can feel cluttered on small mobile screens; Closed-source proprietary backend; No offline-first local mode.
- Sources: https://is.team, https://is.team/pricing
- Confidence: high

### Traycer AI
- URL: https://traycer.ai
- Kind: agent-native
- What: An agentic coding "nerve center" and Kanban-style phase board implementing spec-first development (plan → execute → verify → commit). It connects agents, memory, conversation logs, and artifacts into a centralized visual workspace.
- Agent integration: Direct integrations for Claude Code, Codex CLI, Cursor, and OpenCode; remote host execution; agent-to-agent communication protocol.
- Mobile: native iOS app and native Android app ("Get the mobile app" officially cited on homepage and pricing page).
- Pricing: Free tier ($0 free forever for individuals: BYO coding agent, AI planning & task tracking, unlimited agent sessions, remote hosts, mobile app, zero token markup); Sync for teams ($16/user/month billed yearly: adds cloud sync, cross-device switching, team collaboration) as of October 2026.
- Self-host: local desktop app (macOS Apple Silicon/Intel, Windows, Linux) storing project state locally; cloud sync available on team tier.
- License: proprietary.
- Activity: Active 2025-2026 releases; 1.4k+ community members and verified active releases.
- Pros: Complete cross-platform ecosystem with native iOS/Android mobile apps and desktop clients; Spec-first plan/execute/verify/commit phased board; BYO agent subscriptions with zero token markup; Agent-to-agent communication and remote host offloading.
- Cons: Team collaboration and cloud synchronization locked behind $16/user/mo plan; Proprietary binary; Not fully open-source backend.
- Sources: https://traycer.ai, https://traycer.ai/pricing
- Confidence: high

### PlateSpinner
- URL: https://github.com/moridinamael/platespinner
- Kind: agent-native
- What: An open-source, local-first Kanban board that orchestrates headless AI coding agents through a structured three-phase pipeline: Propose (read-only analysis), Plan (implementation plan generation), and Execute (write-enabled agent execution with live diffs). It includes a continuous autonomous "Autoclicker" mode with an AI judge.
- Agent integration: Headless CLI agent execution spawning Claude Code, Codex CLI, or Gemini CLI subprocesses; WebSocket live streaming; REST API; automated Git branch creation per task (`kanban/task-...`).
- Mobile: responsive web (Node.js web UI binds to `HOST=0.0.0.0` with `APP_API_TOKEN` bearer auth for mobile browser access on LAN); no native mobile app.
- Pricing: open-source free (100% free under MIT License).
- Self-host: yes (local Node.js/Express backend + Vite/React frontend, runs entirely locally on developer machine).
- License: MIT License (Copyright 2026 Matt Freeman).
- Activity: Active 2026 GitHub commits and Show HN launch; active repository maintenance.
- Pros: Strict three-stage pipeline (Propose → Plan → Execute) prevents runaway agent code destruction; "Autoclicker" mode allows supervised autonomous execution with budget caps; Built-in diff viewer and automated Git branch isolation; Fully open-source and free without telemetry.
- Cons: Relies on locally installed CLI agent binaries (Claude Code / Codex); Lacks built-in multi-developer cloud sync; No native mobile app.
- Sources: https://github.com/moridinamael/platespinner, https://raw.githubusercontent.com/moridinamael/platespinner/main/README.md, https://raw.githubusercontent.com/moridinamael/platespinner/main/LICENSE
- Confidence: high

### Flux
- URL: https://paddo.dev/blog/flux-kanban-for-ai-agents/
- Kind: agent-native
- What: A fast, hackable, execution-agnostic Kanban board and task engine designed specifically for AI agents and human collaboration. Tasks live in a single JSON file synced via Git, exposed through an MCP server, CLI (`flux ready`), REST API, and web dashboard.
- Agent integration: Official MCP server for Claude Code and Cursor; CLI (`flux ready` returns priority-sorted unblocked tasks for piping); REST API; webhooks; companion TUI ("Momentum").
- Mobile: responsive web dashboard (`app.getflux.dev`); no native mobile app.
- Pricing: open-source free (MIT License); free hosted web app at app.getflux.dev.
- Self-host: yes (Docker image, Bun / Node.js runtime, local JSON or SQLite backend).
- License: MIT License.
- Activity: Released January 2026; active GitHub repository (`sirsjg/flux`) and blog updates.
- Pros: First-class MCP server and CLI-first piping design; Tasks stored in repository Git branch (zero vendor lock-in); Dependency graph ensures agents only work on unblocked tasks; Clean lightweight Preact/Bun architecture.
- Cons: Early-stage project without enterprise ACLs; Companion TUI and web app have simple visual styling; No native mobile app.
- Sources: https://paddo.dev/blog/flux-kanban-for-ai-agents/, https://github.com/sirsjg/flux, https://raw.githubusercontent.com/sirsjg/flux/main/README.md
- Confidence: high

### Nimbalyst
- URL: https://nimbalyst.com
- Kind: agent-native
- What: An open-source visual workspace and session Kanban board for orchestrating Claude Code, Codex, OpenCode, and Copilot. It enables running parallel coding sessions in isolated Git worktrees, visual WYSIWYG editing, and tracking plans, bugs, and tasks alongside agents.
- Agent integration: Headless and interactive agent runner for Claude Code, Codex, OpenCode, Copilot; full MCP client and server support with visual tool widgets; Git worktree isolation per task.
- Mobile: native iOS companion app ("See which agents need you, reply by text or voice, swipe through diffs, queue the next task, and get a push when an agent is waiting"); no native Android app.
- Pricing: open-source free (MIT License).
- Self-host: yes (desktop application for macOS, Windows, Linux; local plain files on disk).
- License: MIT License (Copyright 2024-2026 Nimbalyst Inc.).
- Activity: 1,908+ GitHub stars; active releases and commits pushed through October 2026.
- Pros: Rare tool offering a genuine native iOS companion app with push notifications and diff reviewing; Session Kanban board deeply linked to Git worktrees and diffs; Rich visual editors (Monaco, Mermaid, Excalidraw, Markdown WYSIWYG); Zero proprietary file formats (plain files in Git).
- Cons: High resource consumption when running multiple parallel agents locally; Desktop client required as primary host; Android companion app not yet available.
- Sources: https://nimbalyst.com, https://github.com/nimbalyst/nimbalyst, https://raw.githubusercontent.com/nimbalyst/nimbalyst/main/README.md
- Confidence: high

### Kandev
- URL: https://kandev.ai
- Kind: agent-native
- What: An open-source control plane and Kanban workbench for agentic software development. It orchestrates parallel tasks across Git worktrees, manages customizable review gates, and provides an integrated workspace with file trees, terminal, browser preview, and git diffs.
- Agent integration: Multi-provider agent execution (supports Anthropic, OpenAI, local models, CLI harnesses); `agentctl` task environment helpers; Git worktree orchestration.
- Mobile: responsive web explicitly adapted for mobile ("Use Kandev from a Phone" via Tailscale/VPN in official docs, with mobile task controls and drawer layouts); no native mobile app.
- Pricing: open-source free (GNU AGPLv3).
- Self-host: yes (distributed as a single self-contained native Go binary with embedded web UI; Docker, Homebrew, systemd service; zero cloud dependencies).
- License: GNU Affero General Public License v3.0 (AGPL-3.0).
- Activity: 954+ GitHub stars; active commits pushed October 2026.
- Pros: Single self-contained Go binary with embedded web assets (zero runtime Node.js requirement); Opinionated review-first gates and Git worktree isolation; Explicit mobile phone workflow guide via private network/VPN; Multi-provider support with zero telemetry.
- Cons: AGPL-3.0 copyleft license; Requires private VPN (Tailscale/Cloudflare Tunnel) for secure mobile access; Steeper learning curve for configuring custom review workflows.
- Sources: https://kandev.ai, https://github.com/kdlbs/kandev, https://raw.githubusercontent.com/kdlbs/kandev/main/README.md, https://raw.githubusercontent.com/kdlbs/kandev/main/docs/public/mobile-remote-access.md
- Confidence: high

### Ouijit
- URL: https://ouijit.com
- Kind: agent-native
- What: A desktop task and terminal manager ("Integrated Divination Environment") for running coding agents in parallel. Each task card on the Kanban board gets its own Git worktree, dedicated terminal, lifecycle hooks that trigger agent CLIs on card movements, and a session-aware CLI.
- Agent integration: CLI agent execution (Claude Code, Codex, Aider, custom scripts); lifecycle hooks triggered on Kanban state transitions (To Do → In Progress → In Review → Done); session-aware CLI (`ouijit task`); per-terminal sandboxing.
- Mobile: none (desktop application for macOS Apple Silicon/Intel and Linux x64; explicitly requires desktop download); no native mobile app.
- Pricing: open-source free (AGPL-3.0).
- Self-host: yes (local desktop application; local-only files, zero telemetry, no account required).
- License: GNU Affero General Public License v3.0 (AGPL-3.0).
- Activity: Active 2026 release cycle with downloadable DMGs and Linux packages.
- Pros: Direct coupling between Kanban card status transitions and automated agent CLI lifecycle hooks; Dedicated Git worktree and sandboxed terminal per card; Interactive diff review inside the desktop app; 100% offline-first with no telemetry.
- Cons: Desktop-only application (no web interface, no Windows build currently); No mobile support; Designed primarily for single-developer local workflows.
- Sources: https://ouijit.com, https://github.com/ouijit/ouijit, https://raw.githubusercontent.com/ouijit/ouijit/main/README.md
- Confidence: high

### AgentRQ
- URL: https://agentrq.com
- Kind: agent-native
- What: A real-time, human-in-the-loop task manager and conversational control plane for AI agents. It provides a visual task board (list and Kanban views), task scheduling, event triggers, and workflow graphs, enabling agents to pull tasks, update progress, and request approvals via MCP and ACP.
- Agent integration: Official MCP server (`https://mcp.agentrq.com/mcp` and dedicated workspace MCP endpoints); ACP (Agent Client Protocol); Chrome Extension; supports Claude Code, Gemini CLI, Cursor.
- Mobile: responsive web ("open on any device, as a list or a Kanban" cited in documentation); Chrome Extension; no native mobile app.
- Pricing: free self-hosted open-source (AGPL-3.0); free cloud tier ("Start Free — No Credit Card") as of October 2026.
- Self-host: yes (self-hosted via Docker and Node.js backend; fully independent control plane).
- License: GNU Affero General Public License v3.0 (AGPL-3.0).
- Activity: 1,158+ GitHub stars; active commits pushed October 2026.
- Pros: Full supervisor-and-colony MCP architecture with workspace isolation; Interactive human-in-the-loop permission escalation for sensitive actions; Drag-and-drop workflow graph linking events to workspaces; Built-in on-device speech-to-text (in-browser Whisper) and auto-titling.
- Cons: AGPL-3.0 license; Complex multi-workspace architecture may be heavy for single-agent tasks; No native mobile application.
- Sources: https://agentrq.com, https://agentrq.com/pricing, https://github.com/agentrq/agentrq, https://raw.githubusercontent.com/agentrq/agentrq/main/README.md
- Confidence: high

### It's a Plan (ItsaPlan)
- URL: https://itsaplan.dev
- Kind: agent-native
- What: An open-source, self-hosted project management and issue tracking system (alternative to Linear, Jira, and Plane) where AI agents are treated as first-class teammates with roles, permissions, and assignee slots on the board alongside humans.
- Agent integration: Built-in native MCP server, REST API, webhooks; agents take issue assignments, update statuses, leave comments, and execute automated schedules.
- Mobile: responsive web (tested with mobile viewport meta; optimized for mobile browser access); no native mobile app.
- Pricing: open-source free (AGPL-3.0); no per-seat fees or enterprise paywalls.
- Self-host: yes (Docker Compose, Coolify, Railway 1-click deployment; requires TLS reverse proxy).
- License: GNU Affero General Public License v3.0 (AGPL-3.0).
- Activity: 983+ GitHub stars; launched on Product Hunt (`it-s-a-plan`); active commits in October 2026.
- Pros: Complete Linear/Plane alternative with full Kanban, table, timeline, and calendar views; Agents possess first-class assignee slots, roles, and permissions; Native MCP server built directly into the core platform; One-click deployment templates on Docker, Coolify, and Railway.
- Cons: Self-hosting strictly enforces a TLS-terminating reverse proxy (session cookies rejected over HTTP); AGPL-3.0 license; No native mobile app.
- Sources: https://itsaplan.dev, https://github.com/croffasia/itsaplan, https://raw.githubusercontent.com/croffasia/itsaplan/main/README.md, https://raw.githubusercontent.com/croffasia/itsaplan/main/docs/self-hosting.md
- Confidence: high

### Kaiban Board (KaibanJS)
- URL: https://www.kaibanjs.com
- Kind: agent-native
- What: A visual Kanban board UI and JavaScript-native multi-agent orchestration framework designed to visualize, manage, and inspect AI agent teams. It renders agent states, subtasks, thought processes, and handoffs as cards moving across Kanban lanes.
- Agent integration: JavaScript SDK (`kaibanjs`), embeddable React component (`kaiban-board` on npm), integrates with LangChain.js, OpenAI, Anthropic, and Mistral LLMs.
- Mobile: responsive web (cloud playground and embeddable web component adapt to mobile screens); no native mobile app.
- Pricing: open-source free (MIT License); free interactive web playground.
- Self-host: yes (npm package `kaiban-board` embeddable in any Next.js/Vite/React app or Docker container).
- License: MIT License.
- Activity: 1,480+ stars on KaibanJS; active maintenance in 2025-2026.
- Pros: True Kanban visual paradigm adapted specifically for agent state debugging; JavaScript/TypeScript-first ecosystem with easy React embedding; Transparent real-time observation of agent thought chains and handoffs; Permissive MIT license.
- Cons: Geared toward application developers embedding agent workflows rather than an end-user project management tool; Does not manage Git worktrees or terminal sessions; No native mobile application.
- Sources: https://www.kaibanjs.com, https://github.com/kaiban-ai/KaibanJS, https://github.com/kaiban-ai/kaiban-board
- Confidence: high

### Cline Kanban
- URL: https://github.com/cline/kanban
- Kind: agent-native
- What: A zero-config local web application launched with `npx kanban` from any Git repository. It replaces traditional IDE agent interactions by running multiple CLI coding agents in parallel across automatic Git worktrees, displaying progress on task cards with auto-commit and dependency chains.
- Agent integration: Headless CLI agent execution (automatically detects Claude Code, Cline CLI, Codex, etc.); auto-creates dedicated Git worktrees and terminals per card; injects board-management prompts.
- Mobile: responsive web (local web server accessible over local network IP via mobile browser); no native mobile app.
- Pricing: open-source free (Apache-2.0).
- Self-host: yes (runs locally with zero setup via `npx kanban` or `npm i -g kanban`).
- License: Apache License 2.0.
- Activity: 1,360+ GitHub stars; active research preview in October 2026.
- Pros: Zero-configuration instant launch (`npx kanban` from any repo); Automatically creates and manages isolated Git worktrees for every task card; Autonomous dependency chains linking cards together; Built-in diff viewer and auto-commit capabilities.
- Cons: Experimental research preview requiring elevated permissions for agents; Local-only single-user tool with no multi-user sync; No native mobile app.
- Sources: https://github.com/cline/kanban, https://raw.githubusercontent.com/cline/kanban/main/README.md
- Confidence: high

### Veritas Kanban
- URL: https://sstb.ai
- Kind: agent-native
- What: A local-first task management and Kanban board with optional AI agent orchestration layers. It provides an honest, transparent view of project status, allowing users to start with a visual board and selectively layer in CLI tools, MCP servers, OpenClaw agents, and Squad Chat webhooks.
- Agent integration: Official MCP server (`veritas-kanban-mcp`), CLI, OpenClaw integration, Squad Chat webhooks, REST API.
- Mobile: responsive web (accessible locally or over LAN via `localhost:3000`); no native mobile app.
- Pricing: open-source free (MIT License).
- Self-host: yes (pnpm / Node.js + SQLite/Postgres; fully self-hostable on Docker or local hardware).
- License: MIT License (Copyright 2026 Digital Meld).
- Activity: 841+ GitHub stars; version 6.2.1 released October 2026.
- Pros: Modular architecture (visual board first, optional MCP/agent orchestration layers); Native OpenClaw and MCP integration; Comprehensive governance and workflow gating options; MIT license.
- Cons: Setting up advanced agent webhooks requires manual configuration; Relatively small community compared to major PM platforms; No native mobile app.
- Sources: https://sstb.ai, https://github.com/BradGroux/veritas-kanban, https://raw.githubusercontent.com/BradGroux/veritas-kanban/main/README.md
- Confidence: high

### Agent Teams AI
- URL: https://agentteams.live
- Kind: agent-native
- What: An open-source desktop workspace where users manage autonomous AI agent teams as "the boss". Agents collaborate on tasks, exchange messages, conduct peer reviews on code, and report live progress onto a shared Kanban board.
- Agent integration: Direct integrations for 300+ models and 200+ LLM providers; supports Claude Code, Codex, OpenCode, Cursor, Grok, Copilot; multi-agent peer communication and automated review workflows.
- Mobile: responsive web UI for remote web dashboard; desktop app for macOS, Windows, Linux; no native mobile app.
- Pricing: open-source free (AGPL-3.0); includes free out-of-the-box models with no API key or credit card required.
- Self-host: yes (desktop application or self-hosted web server).
- License: GNU Affero General Public License v3.0 (AGPL-3.0).
- Activity: 2,265+ GitHub stars; active release cycle pushed through October 2026.
- Pros: Enables multi-agent peer review and agent-to-agent communication on a Kanban board; Ships with free no-auth models out of the box for zero-barrier onboarding; Massive provider support (300+ models); Visual task detail inspection with diffs.
- Cons: AGPL-3.0 copyleft license; Multi-agent concurrency can quickly drain local CPU/RAM or API limits; No native mobile app.
- Sources: https://agentteams.live, https://github.com/777genius/agent-teams-ai, https://raw.githubusercontent.com/777genius/agent-teams-ai/main/README.md
- Confidence: high

### Claude Code Agent Monitor
- URL: https://hoangsonww.github.io/Claude-Code-Agent-Monitor
- Kind: agent-native
- What: A real-time monitoring dashboard and Kanban status board for Claude Code, Cursor, and Codex sessions. It visualizes agent conversations, tool calls, agent hierarchies, and token costs while allowing developers to operate and resume local agents directly from the board.
- Agent integration: Deep integration with Claude Code and Codex via local process monitoring, MCP 1.0 support, WebSockets, Express REST API, Commander CLI.
- Mobile: responsive web (responsive React dashboard with Web Push VAPID notifications for mobile web alerts); no native mobile app.
- Pricing: open-source free (MIT License).
- Self-host: yes (Node.js/Express + React/Vite + SQLite3; runs locally or packaged as a desktop app).
- License: MIT License (Copyright 2026 Son Nguyen).
- Activity: 1,080+ GitHub stars; active maintenance and commits in October 2026.
- Pros: Complete observability cockpit combining Kanban boards with token cost/usage analytics; Live agent reattachment and resumption from the UI; Web Push notifications alert when an agent needs user feedback; MIT license.
- Cons: Focused on monitoring and session control rather than general long-term backlog planning; Single-user / local host focus; No native mobile app.
- Sources: https://hoangsonww.github.io/Claude-Code-Agent-Monitor, https://github.com/hoangsonww/Claude-Code-Agent-Monitor, https://raw.githubusercontent.com/hoangsonww/Claude-Code-Agent-Monitor/master/README.md
- Confidence: high

### Dispatch Board
- URL: https://www.dispatchboard.ai
- Kind: agent-native
- What: A collaborative Kanban platform designed to make AI agents work like human team members on familiar boards. Tasks can be dragged and assigned either to humans or AI agents (e.g., GPT-4/Claude), which automatically execute the task without complicated prompting.
- Agent integration: Cloud agent runners executing assigned cards; REST API; webhooks.
- Mobile: responsive web (tested in mobile web browsers); no native mobile app.
- Pricing: free beta / waitlist signup currently ("Sign up for launch updates", free preview during launch phase) as of October 2026.
- Self-host: no (cloud-hosted multi-tenant SaaS).
- License: proprietary.
- Activity: Active launch site and beta marketing in 2025-2026.
- Pros: Familiar Trello-like Kanban interface with zero learning curve; Seamless human vs AI assignee dropdowns on any card; No complex DSL or prompt engineering needed for non-technical users; Synchronized progress tracking.
- Cons: Currently in beta / waitlist phase; Proprietary cloud-only architecture; Limited technical details on local CLI agent (Claude Code/Codex) bindings.
- Sources: https://www.dispatchboard.ai
- Confidence: medium (beta platform with limited public API documentation)

### Moo Tasks
- URL: https://mootasks.dev
- Kind: agent-native
- What: A modern Kanban-style task management dashboard with an integrated Model Context Protocol (MCP) server. Humans manage tasks via a drag-and-drop web UI, while AI agents connect via MCP to discover, accept, update, and create tasks programmatically.
- Agent integration: Integrated official MCP server; REST API; supports Claude Code, Cursor, OpenCode, and any MCP client.
- Mobile: responsive web (Nuxt 4 / Tailwind CSS v4 responsive mobile layout); no native mobile app.
- Pricing: open-source free (MIT License).
- Self-host: yes (Docker Compose, Nuxt 4 + Node.js + MySQL/SQLite with Drizzle ORM).
- License: MIT License (Copyright 2026 Moo Tasks).
- Activity: Active 2026 GitHub commits (`dizlexic/moo-tasks`) and Show HN launch.
- Pros: Purpose-built from the ground up for the MCP protocol; Clean, modern Nuxt 4 / Tailwind CSS drag-and-drop UI; Simple Docker self-hosting; MIT license.
- Cons: Early-stage project without deep enterprise workflow rules; Lacks Git worktree orchestration; No native mobile app.
- Sources: https://mootasks.dev, https://github.com/dizlexic/moo-tasks, https://raw.githubusercontent.com/dizlexic/moo-tasks/main/README.md
- Confidence: high

### Shrimp Task Manager
- URL: https://cjo4m06.github.io/mcp-shrimp-task-manager
- Kind: agent-native
- What: An MCP task manager built for AI agents that emphasizes chain-of-thought, reflection, and task decomposition. It provides persistent memory across sessions and includes an optional lightweight web GUI for monitoring task progress and dependencies.
- Agent integration: Official MCP server (`shrimp-task-manager` on Smithery and Glama); supports Claude Code, Cline, Claude Desktop, Cursor.
- Mobile: responsive web (lightweight optional web GUI enabled via `ENABLE_GUI=true`); no native mobile app.
- Pricing: open-source free (MIT License).
- Self-host: yes (Node.js application storing data locally in JSON files; local execution).
- License: MIT License (Copyright 2025 siage).
- Activity: 2,146+ GitHub stars (`cjo4m06/mcp-shrimp-task-manager`); active maintenance and multi-language documentation.
- Pros: Deep task decomposition and dependency management for complex reasoning workflows; Preserves context and memory across token compaction and new sessions; Wide MCP client compatibility (Claude Code, Cline, Cursor); Permissive MIT license.
- Cons: GUI is an optional secondary interface rather than a full-featured collaborative board; Local JSON-based storage without multi-user concurrent editing; No native mobile app.
- Sources: https://cjo4m06.github.io/mcp-shrimp-task-manager, https://github.com/cjo4m06/mcp-shrimp-task-manager, https://raw.githubusercontent.com/cjo4m06/mcp-shrimp-task-manager/main/README.md
- Confidence: high

### Yao Agents
- URL: https://yaoagents.com
- Kind: agent-native
- What: An open-source, self-hosted platform connecting agents, devices, and workspaces into a unified system. It features a conversation-driven task board where user instructions become Kanban tasks that agents execute across local computers.
- Agent integration: Open API (SSE and WebSockets), DeepSeek Harness, Jev (TypeSafe AI) decision engine, desktop and mobile client connectivity.
- Mobile: native Android app (Beta APK downloadable at `https://yaoagents.com/download/yaoagents`) + responsive web UI + desktop app.
- Pricing: open-source free core (modified Apache 2.0); Enterprise tier available for commercial fleets.
- Self-host: yes (single binary Go application with embedded engine, Docker, desktop apps).
- License: Modified Apache License 2.0 (Yao Engine).
- Activity: 8,130+ GitHub stars (`YaoApp/yao`); active commits in October 2026.
- Pros: Rare tool offering a native Android application alongside desktop and web; Distributed multi-device execution (agents run across your own computers); Built-in Kanban task board derived from conversation; Integrates DeepSeek Harness and Jev typed decision engine.
- Cons: Modified Apache 2.0 license has commercial redistribution terms; Broad platform scope can be complex to configure compared to single-purpose task boards; iOS native app not yet released (Android only).
- Sources: https://yaoagents.com, https://github.com/YaoApp/yao, https://raw.githubusercontent.com/YaoApp/yao/main/README.md
- Confidence: high

## EXPAND
- LEAD: Height 2.0 (Autonomous Project Management) — WHY: Height overhauled its core product into an autonomous project manager where AI agents triage incoming requests, update task statuses based on git activity, and manage sprints. — ANGLE: web search and changelog audit of height.app
- LEAD: Tara AI Sprint Agent Workflows — WHY: Tara AI specifically introduced autonomous sprint assistants and GitHub-linked agent kanban sync in late 2024–2025. — ANGLE: "site:tara.ai agent sprint kanban"
- LEAD: CCPM (automazeio/ccpm) & Spec-Driven GitHub Project Boards — WHY: CCPM has 8,400+ stars implementing PRD-to-issue workflows directly on GitHub Projects/Issues with parallel Git worktrees for Claude Code and Codex. — ANGLE: "site:github.com automazeio ccpm PRD epic issues"
