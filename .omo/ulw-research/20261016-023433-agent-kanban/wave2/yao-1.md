### Aizen
- URL: https://aizen.win/
- Kind: agent-native
- What: A native macOS workspace for parallel software development and AI coding agent orchestration where each project or branch gets an isolated environment with terminals, file browser, web browser, and agent chat sessions; it does NOT have a kanban/board view of agent tasks (no - organizes work via project/worktree lists, split panes, and tabs rather than a kanban board).
- Agent integration: ACP (Agent Client Protocol) registry-first with custom CLI/executable commands; seeded defaults support Claude Code, Codex, and OpenCode (also Gemini CLI and other ACP agents); built-in MCP server marketplace to attach tools to agents; companion CLI (`aizen`) for opening projects and attaching to sessions.
- Mobile: none (macOS only, strictly Apple Silicon / M-series Mac requiring macOS 13.5+; Intel intentionally dropped since v1.0.71; no iOS/Android apps or PWA).
- Pricing: Core app free forever ($0) under GPL-3.0; Pro tier $5.99/month or $59/year; Lifetime license $179 one-time (as of October 2026).
- Self-host: yes (local native desktop application built with Swift/SwiftUI and GhosttyKit/libghostty; builds from source via Xcode 16 and Zig; all project state stored locally in Core Data and git worktrees).
- License: GNU General Public License v3.0 (GPL-3.0).
- Activity: Open-source on GitHub (vivy-company/aizen); v1.0.83 marked as final release in 1.0.x series with active development pivoting towards 2.0.0; recent commits in July 2026.
- Pros: True native macOS experience with high-performance GPU-accelerated libghostty terminal; Seamless ACP support allowing arbitrary agent binaries and custom MCP servers; Zero-overhead parallel environments using native git worktrees and folders; Tmux-backed session persistence across reboots.
- Cons: No built-in kanban or visual task tracker board for agent workflows; Platform-locked to Apple Silicon macOS only (no Windows, Linux, or mobile); Breaking architecture redesign underway for 2.0.0; Cloud/team synchronization features absent.
- Sources: https://aizen.win/, https://yetanotherorchestrator.app/apps/aizen, https://github.com/vivy-company/aizen, https://github.com/vivy-company/aizen/releases
- Confidence: high

### Aperant
- URL: https://aperant.com/
- Kind: agent-native
- What: An autonomous multi-agent coding framework (formerly Auto Claude) that plans, builds, tests, and validates software using git worktrees and automated quality assurance loops; includes a dedicated visual KANBAN board (yes - full kanban board view with columns: Backlog, Queue, In Progress, AI Review, Human Review, Done, supporting drag-and-drop, concurrency limits, and task lifecycle management).
- Agent integration: Deep native orchestration of Anthropic Claude Code CLI (`@anthropic-ai/claude-code`) as well as multi-model SDK/APIs (OpenAI, Gemini, Bedrock, Groq, OpenRouter, Ollama); official MCP client support (command-based and HTTP/SSE MCP servers); Linear GraphQL API import; GitHub/GitLab issues & PR integration; task automation via implementation plans (`implementation_plan.json`, `requirements.json`).
- Mobile: none (Electron desktop application for macOS [Apple Silicon & Intel DMG], Windows [NSIS installer], and Linux [AppImage, DEB, Flatpak]; no native mobile app or responsive PWA web client).
- Pricing: Free and open-source under AGPL-3.0; commercial v3.0 cloud pricing waitlist is pending ("Pricing coming soon" as of October 2026); requires user's own Claude Pro/Max subscription or API keys.
- Self-host: yes (local Electron/Node.js desktop application running entirely on developer machines; stores tasks, worktrees, and specs in local directories like `.auto-claude/specs/` and SQLite/libsql).
- License: GNU Affero General Public License v3.0 (AGPL-3.0); commercial licensing available for closed-source derivatives.
- Activity: High community activity with 12,000+ GitHub stars (AndyMik90/Aperant, formerly Auto-Claude); stable v2.7.6 and v2.8.0-beta.6; core v2.x codebase in maintenance while v3.0 ground-up rewrite is underway.
- Pros: Rich visual kanban board purpose-built for managing parallel agent runs; Automated multi-phase execution (planning, coding, AI QA review, human review); Git worktree isolation per task with automated conflict resolution; Built-in rate-limit monitoring, proactive token usage management, and multi-profile account rotation.
- Cons: Desktop app only without web or mobile companion; v3.0 rebuild paused public code PRs on the current repository; Heavy Electron framework resource footprint compared to native Swift/Rust apps; Tightest workflows specifically optimized for Claude Code.
- Sources: https://aperant.com/, https://yetanotherorchestrator.app/apps/aperant, https://github.com/AndyMik90/Aperant, https://github.com/AndyMik90/Aperant/releases
- Confidence: high

### Atrium
- URL: https://getatrium.dev/
- Kind: agent-native
- What: A persistent local-first workspace application and agent multiplexer for running up to 49 AI coding agents in parallel across persistent tiled rooms and git worktrees; includes an integrated KANBAN board (yes - first-class built-in Tasks pane with kanban board view by default, custom statuses/lanes, color-coded priority dots, live embedded agent chat right on cards, repeat/cron schedules, and execution run history).
- Agent integration: Adapter architecture supporting 12+ CLI coding agents (Claude Code via `claude-sdk`, Codex via `codex-app-server`, Antigravity via `acp`, Grok, Kimi Code, OpenCode, Cursor Agent, Pi via `pi-rpc`, Hermes, Goose, Muse Code); official bidirectional CLI (`atrium task`, `atrium pane`, `atrium agent message`, etc.) allowing agents to script and inspect other agents; hook server (`atrium://hooks/`) for real-time lifecycle status transitions; MCP support via underlying harnesses.
- Mobile: native iOS app (yes - native iOS and iPadOS app in private TestFlight beta requiring iOS/iPadOS 26; connects to desktop/headless daemons via encrypted peer links and Tailscale; features live agent chat, multi-pane iPad layouts, lockscreen Live Activities, dynamic island status, and spoken Walk Mode; no Android app yet).
- Pricing: Free during current early access / public beta ($0) with bring-your-own model accounts; commercial pricing tiers not yet published as of October 2026.
- Self-host: yes (local desktop application for macOS and Windows, plus headless daemon for Linux servers via SSH/Tailscale; SQLite database stored locally at `~/.atrium/tasks.db` and state stored on local disk).
- License: Proprietary / Closed-source core (adapters and canonical skill registry are open-source on GitHub at jonnyasmar/atrium-adapters).
- Activity: Very high velocity; shipped v0.345.0 in October 2026 with frequent updates every few days; active Discord community and rapid changelog iterations.
- Pros: Full-featured native kanban task tracker with live streaming agent chats embedded directly inside cards; Broadest CLI agent support (Claude Code, Codex, Antigravity, OpenCode, Pi, etc.); Native iOS/iPadOS mobile app with Live Activities, audio Walk Mode, and remote server control; Scriptable JSON CLI enabling multi-agent coordination, task dispatch, and recurrence/cron loops.
- Cons: Core desktop client is closed-source/proprietary; Desktop GUI currently supports macOS and Windows only (Linux runs headless daemon/tools only); Mobile app currently restricted to iOS private TestFlight beta (no public App Store link or Android); Steep learning curve due to extensive protocol and feature set.
- Sources: https://getatrium.dev/, https://yetanotherorchestrator.app/apps/atrium, https://getatrium.dev/docs, https://getatrium.dev/docs/panes/tasks, https://getatrium.dev/docs/concepts/tasks, https://getatrium.dev/docs/reference/cli, https://getatrium.dev/docs/mobile, https://getatrium.dev/docs/adapters
- Confidence: high

## EXPAND
- LEAD: Automaker (https://yetanotherorchestrator.app/apps/automaker/) — WHY: Cross-platform orchestrator (macOS, Windows, Linux, Web) supporting autonomous agent execution and kanban task management across Claude and Codex — ANGLE: web search "site:yetanotherorchestrator.app/apps/automaker" OR "automaker ai agent kanban"
- LEAD: Conductor & Superset — WHY: Directly cited on Atrium's comparison matrix (getatrium.dev/vs) as parallel multi-agent workspaces with task tracking features — ANGLE: web search "getatrium.dev/vs/conductor" OR "getatrium.dev/vs/superset"
