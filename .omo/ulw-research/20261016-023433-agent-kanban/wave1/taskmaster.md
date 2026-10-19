### Taskmaster AI (eyaltoledano/claude-task-master)
- URL: https://github.com/eyaltoledano/claude-task-master (Commercial platform: https://tryhamster.com/product/taskmaster)
- Kind: agent-native
- What: A CLI-first and MCP-native task management system designed specifically for AI coding agents to parse PRDs into structured tasks, analyze complexity, manage dependencies, and track task state locally or in sync with team workspaces.
- Agent integration: Official Model Context Protocol (MCP) server (`task-master-ai` via npx/npm with 7 core, 15 standard, or 36 full tools), CLI commands (`task-master` / `tm`), file-based tasks JSON (`.taskmaster/tasks/tasks.json` with cross-process locking), and IDE slash commands/rules; natively supports Claude Code, Cursor, Windsurf, VS Code, Roo Code, Cline, Codex CLI, OpenCode, Gemini CLI, Zed, Trae, and Kiro.
- Mobile: none (No native iOS or Android apps exist on the App Store or Google Play Store; the open-source CLI/MCP engine operates purely locally inside terminal/IDE environments, and the companion Hamster web app is a responsive desktop-first web interface with no dedicated mobile PWA/app listing as of October 2026).
- Pricing: Free & open-source for standalone local use (BYOK / Bring Your Own API Key, or free local models via Ollama / zero-key Claude Code CLI / Codex CLI OAuth). Connected Hamster team plans: Free tier ($0 for 10 briefs/mo, 1 creator, 1 repo, unlimited viewers), Team tier ($40/creator/month or $400/creator/year with unlimited briefs/repos, cloud agents, Linear/Jira sync), and Enterprise ($200/creator/month with SSO/SOC2).
- Self-host: yes (Local file-based self-hosting via npm/npx `task-master-ai` or Docker sandbox; stores tasks, tags, and progress locally in project `.taskmaster/` directories without external server requirements; commercial Hamster Studio cloud sync features remain proprietary hosted SaaS).
- License: MIT License with Commons Clause Condition v1.0 (permits internal/commercial usage, modifications, and redistribution, but explicitly forbids selling Task Master itself or offering it as a competing commercial hosted service).
- Activity: Active OSS project; ~28,231 GitHub stars, 2,619 forks, latest release v0.43.1 in 2026, over 2M npm downloads.
- Pros: Deeply native MCP protocol support across 13+ modern AI IDEs and CLI agents (Claude Code, Cursor, Codex, Windsurf); Built-in PRD parser, automatic dependency graph resolution, complexity scoring (1-10), and autonomous TDD execution (`task-master loop` / `autopilot`); Safe local multi-process coordination with file locking and atomic writes preventing write collisions when multiple agent windows run concurrently.
- Cons: Commons Clause license restriction bars third-party commercial re-hosting and SaaS resale; Purely local CLI/MCP by default with visual Kanban board relegated to VS Code extension or proprietary Hamster cloud tier; No native mobile applications for inspecting agent progress on iOS/Android.
- Sources: https://github.com/eyaltoledano/claude-task-master, https://tryhamster.com/pricing, https://tryhamster.com/product/taskmaster, https://tryhamster.com/product/studio, https://raw.githubusercontent.com/eyaltoledano/claude-task-master/main/LICENSE, https://marketplace.visualstudio.com/items?itemName=Hamster.task-master-hamster, https://api.github.com/repos/eyaltoledano/claude-task-master
- Confidence: high (all licensing, pricing tiers, MCP schemas, agent compatibility, release versions, and mobile availability verified directly against official repository and site documents).

### Taskmaster Kanban VS Code Extension (Hamster.task-master-hamster)
- URL: https://marketplace.visualstudio.com/items?itemName=Hamster.task-master-hamster
- Kind: agent-native
- What: The official visual Kanban board extension for VS Code and Cursor that surfaces local `.taskmaster` tasks into drag-and-drop columns with real-time bidirectional MCP synchronization.
- Agent integration: Direct background connection to the local `task-master-ai` MCP server over stdio; synchronizes state changes with `.taskmaster/tasks/tasks.json` in real time so actions taken by coding agents (Claude Code, Cursor, Codex) immediately update the UI board and vice versa.
- Mobile: none (Desktop-only IDE extension for VS Code and Cursor; can only be accessed remotely via VS Code Server / code-server web browser instances).
- Pricing: Free (Open-source extension distributed freely via the Visual Studio Marketplace).
- License: MIT License.
- Self-host: yes (Runs entirely on developer local machines inside VS Code / Cursor desktop client).
- Activity: Active extension maintained alongside Taskmaster AI updates (version 1.x+, official extension by Hamster).
- Pros: Provides a full drag-and-drop visual Kanban board directly inside developer IDEs; Real-time automatic polling and MCP synchronization with AI-updated tasks; Native UI for viewing subtasks, dependencies, and appending AI progress notes.
- Cons: Limited to VS Code / Cursor ecosystem (no standalone web dashboard without running full VS Code web); No native mobile access; Requires local Node.js environment to spawn the underlying MCP daemon.
- Sources: https://marketplace.visualstudio.com/items?itemName=Hamster.task-master-hamster, https://github.com/eyaltoledano/claude-task-master
- Confidence: high (verified on VS Code Marketplace listing and repository documentation).

## EXPAND
- LEAD: Hamster Studio Web Platform (tryhamster.com/product/studio) — WHY: Cloud counterpart that provides team-level collaborative briefs, cloud agents, and Linear/Jira two-way task sync connecting to Taskmaster CLI — ANGLE: webfetch https://tryhamster.com/product/studio and review team sync endpoints
- LEAD: DevDreed/claude-task-master-extension — WHY: Alternative community-created visual interface extension for Claude Task Master before official Hamster extension — ANGLE: search GitHub repo DevDreed/claude-task-master-extension
