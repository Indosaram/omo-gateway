### Automaker
- URL: https://automaker.app/
- Kind: agent-native
- What: Autonomous AI software engineering studio and desktop/web environment where users manage feature backlogs and orchestrate coding agents; includes a visual Kanban board (yes: Backlog, In Progress, Waiting Approval, Verified columns with drag-and-drop powered by dnd-kit) where moving a card to "In Progress" triggers autonomous agent execution in isolated git worktrees.
- Agent integration: Claude Agent SDK (@anthropic-ai/claude-agent-sdk), Claude Code CLI integration (detects credentials automatically), multi-provider runner support (Claude Opus/Sonnet/Haiku, Codex via CodexAppServerService, Copilot, Cursor, Gemini, OpenCode); local Express REST API (/api/features, /api/agent, /api/auto-mode, /api/running-agents, /api/pipeline) and WebSocket event streams (/api/events, /api/terminal/ws); supports MCP testing/listing endpoints (/api/mcp).
- Mobile: responsive web and PWA accessible via browser when run in web mode (`npm run dev:web` on http://localhost:3007 or remote host/Docker deployment); no native iOS or Android app listed in App Store/Play Store (verified via automaker.app site, docs, and repo).
- Pricing: Free and open-source (MIT); runs locally or self-hosted in Docker; users bring their own LLM API keys or Claude subscription (notably subject to Anthropic's Claude Agent SDK programmatic credit pool changes as of mid-2026).
- Self-host: yes — local Electron desktop app (macOS, Windows, Linux), local Node/Vite web server (`npm run dev:web`), or Docker container deployment with docker-compose (pre-built Playwright Chromium sandbox, worktree isolation). No enterprise-only restrictions.
- License: MIT
- Activity: GitHub repository AutoMaker-Org/automaker (transferred from webdevcody/automaker); ~1,000+ stars community footprint; last commit on main branch May 22, 2026; official status in README states "This project is no longer actively maintained. The codebase is provided as-is for those who wish to use, study, or fork it."
- Pros: True agent-native Kanban board with automated trigger on card drop; isolated git worktrees prevent destructive changes on main branch; real-time agent thought streaming and multi-planning modes (lite, spec, full); Docker sandbox container with Playwright automated verification tests pre-installed; extensible REST and WebSocket server API.
- Cons: Project is officially deprecated / unmaintained by original authors; heavy reliance on local Node/Electron runtime setup and Claude Agent SDK authentication quirks; no native mobile app; complex multi-package monorepo if bugs need patching.
- Sources: https://automaker.app/, https://yetanotherorchestrator.app/apps/automaker/, https://github.com/webdevcody/automaker, https://github.com/AutoMaker-Org/automaker
- Confidence: high

### Caudex
- URL: https://caudex.sh/
- Kind: agent-native
- What: Lightweight desktop terminal and orchestration workspace purpose-built for Claude Code; does not feature a Kanban/board view (no: uses multi-tab, multi-split terminal layouts and a card-grid "Tab Overview" session switcher rather than a Kanban task board).
- Agent integration: Claude Code native CLI integration via CLI PATH detection; status sync via Claude Code lifecycle hooks (SessionStart, SessionEnd, Stop, Notification, PermissionRequest, UserPromptSubmit injected into `~/.claude/settings.json`) and statusLine pipe monitoring (`tee ~/.caudex/status-data.json`); visual MCP server manager (connect, enable/disable, monitor) and Skills manager; no public general REST/GraphQL API for external agent push, relies on Claude Code hooks and local files.
- Mobile: PWA and touch-responsive web interface supported via built-in Remote Access WebSocket server on loopback paired with tunnels like ngrok, Tailscale Funnel, or Cloudflare Tunnel (touch shortcut bar, swipe tabs, commit/diff on mobile); no native iOS or Android app in App Store/Play Store (cite: https://caudex.sh/docs#remote_access).
- Pricing: Free (closed source freeware in current early versions v0.1.52 as of October 2026; no paid tiers or per-user subscriptions listed on website).
- Self-host: yes — local desktop application running entirely on user machine (macOS DMG for Apple Silicon/Intel, Windows .exe/.msi, Linux .deb/.rpm/.AppImage) with local WebSocket server for remote tunneling; completely local data stored in `~/.caudex/`.
- License: Proprietary (closed source freeware).
- Activity: Active; current version v0.1.52 (as of October 2026); desktop app distributed via direct download on caudex.sh with Linux packages coming soon.
- Pros: First-class Claude Code lifecycle and hook integration with zero configuration; real-time context token usage and cost monitoring progress bar; built-in remote access feature with PWA mobile support over tunnels; visual MCP and skill configuration panels; lightweight multi-split terminal emulator.
- Cons: Closed source / proprietary binary; lacks a Kanban board view for task management (tab overview grid only); strictly tailored to Claude Code (lacks first-class Codex/Pi/Cursor multi-agent hub capabilities); single concurrent remote connection limit in current beta.
- Sources: https://caudex.sh/, https://caudex.sh/docs, https://yetanotherorchestrator.app/apps/caudex/
- Confidence: high

### Clave
- URL: https://github.com/codika-io/clave (redirects/hosted at https://github.com/antasphere/clave)
- Kind: agent-native
- What: Open-source macOS (and early Windows) desktop workstation for running and orchestrating fleets of AI coding agents side-by-side; includes a task board and session management system (yes: provides a task board and session view for parallel agent tracking alongside split/grid PTY terminals).
- Agent integration: Multi-agent CLI support including Claude Code, OpenAI Codex CLI, Google Antigravity CLI, and Pi CLI; provides official companion agent plugin (`@clave/plugin-sdk`, installable via `npx plugins add antasphere/clave` or Claude Code plugin marketplace `/plugin install clave@clave`); built-in loopback HTTP and WebSocket CQRS server (`@clave/server`, `@structure-ai/http`, `@structure-ai/cqrs`) with agent tool endpoints (`/agent-tools/announce`, `/agent-tools/resolve-token`) and MCP server bridge (`@modelcontextprotocol/sdk`) enabling agents to read/manipulate sessions, workspace files (`.clave`), and window layout.
- Mobile: none — native desktop app designed for macOS (Universal DMG/zip) with early Windows installer (`clave-setup.exe`); no mobile apps (iOS/Android) and no responsive web client hosted currently (cite: README and ADR 0003 state browser client and remote web are future architecture targets, loopback daemon only today).
- Pricing: Free and open source (MIT license); 100% free with no paid subscription; optional Antasphere account login.
- Self-host: yes — fully local desktop app (Electron + Node/Bun + Vite) and standalone local daemon (`@clave/server` on Bun with Node terminal-process sidecar over gRPC); stores all sessions and `.clave` workspace files locally in `~/.clave/` with no cloud backend.
- License: MIT
- Activity: Very active; 51 GitHub stars; releases v2.4.0 (stable, Oct 13, 2026) and v3.0.0-beta.3 (pre-release, Oct 15, 2026); commits pushed as recently as Oct 15-16, 2026 (`antasphere/clave` repo).
- Pros: Fast, parallel multi-agent PTY execution across multiple leading CLI agents (Claude, Codex, Antigravity, Pi); robust companion agent plugin with MCP tool bridge and `.clave` workspace generation; modern CQRS/event-sourced backend architecture (`@structure-ai` framework) separating UI client from local daemon; built-in Git panel with MagicSync AI commits and diff viewer.
- Cons: Primarily tailored for macOS (Windows support is early and lacks state dots/tmux persistence; Linux not supported); no mobile app or live responsive web interface; relatively new project with smaller community star count.
- Sources: https://github.com/antasphere/clave, https://yetanotherorchestrator.app/apps/clave/, https://raw.githubusercontent.com/antasphere/clave/prod/README.md, https://raw.githubusercontent.com/antasphere/clave/prod/docs/adr/0003-server-and-client.md
- Confidence: high

## EXPAND
- LEAD: cmux & CNVS (desktop agent orchestrators listed alongside Automaker/Caudex/Clave on yetanotherorchestrator.app) — WHY: cmux offers native macOS terminal multitasking with agent programmability, while CNVS offers voice-commanded multi-agent orchestration that may feature task tracking — ANGLE: webfetch https://yetanotherorchestrator.app/apps/cmux/ and https://yetanotherorchestrator.app/apps/cnvs/
