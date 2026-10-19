### CodeNomad
- URL: https://github.com/NeuralNomadsAI/CodeNomad
- Kind: agent-native
- What: A multi-instance desktop and web orchestrator for OpenCode V2 that manages parallel sessions, background workers, and workspace git branches; KANBAN/board view of agent tasks: no (uses a dual-drawer sidebar session list, missions view, terminal/diff panes, and web preview rather than a kanban column board).
- Agent integration: OpenCode V2 service/CLI daemon connection, MCP server inspection and tool toggling, sidecars (OpenVSCode, ttyd), agent-driven browser preview automation, targets OpenCode.
- Mobile: PWA / responsive web via server mode (`npx @neuralnomads/codenomad`) paired over OpenTunnel relay, with responsive drawer adaptations for touch/mobile screens; no native iOS or Android app.
- Pricing: free / open-source free ($0, MIT license, as of October 2026).
- Self-host: yes via local desktop app (Electron/Tauri), standalone npm server (`npx @neuralnomads/codenomad`), or Docker container; fully local files and local OpenCode instances.
- License: MIT License.
- Activity: 2.6k GitHub stars, latest dev release v0.20.1-dev-20261015 (October 15, 2026), latest stable v0.20.0 (October 2026), active development.
- Pros: Deep native integration with OpenCode V2 daemon, session forking, and subagent orchestration; Git worktree isolation and multi-instance project switcher; Built-in web preview with dev server detection and automated screenshot capture; Cross-platform desktop apps plus headless web server with secure remote tunnel support; Active community with frequent releases and multi-language UI support.
- Cons: Lacks a dedicated kanban or multi-column task board view; Strictly bound to OpenCode V2 (does not orchestrate Claude Code, Codex, or Cursor); Requires local git CLI and OpenCode runtime pre-installed; Unsigned desktop binaries on macOS require manual quarantine attribute removal.
- Sources: https://github.com/NeuralNomadsAI/CodeNomad, https://raw.githubusercontent.com/NeuralNomadsAI/CodeNomad/dev/README.md, https://raw.githubusercontent.com/NeuralNomadsAI/CodeNomad/dev/package.json, https://raw.githubusercontent.com/NeuralNomadsAI/CodeNomad/dev/packages/ui/src/components/instance/instance-shell2.tsx, https://yetanotherorchestrator.app/apps/codenomad/
- Confidence: high

### Codex Monitor
- URL: https://www.codexmonitor.app/
- Kind: agent-native
- What: A lightweight native desktop and mobile companion for OpenAI Codex CLI and app-server that manages multiple workspaces, background execution threads, git worktrees, and diff reviews; KANBAN/board view of agent tasks: no (features a multi-thread list, diff inspector, and tabbed terminal/review interface without a kanban board layout).
- Agent integration: Codex app-server protocol (JSON-RPC over stdio/TCP), GitHub CLI (`gh`) for pull requests and issue tracking, MCP server status inspection, custom prompt/skill integration (`$`, `/prompts:`, `/review`), targets OpenAI Codex.
- Mobile: Native iOS companion app in TestFlight / source build (built with Tauri mobile connecting to desktop daemon over Tailscale/local network); responsive mobile web layout; no Android app.
- Pricing: free / open-source free ($0, MIT license, as of October 2026).
- Self-host: yes via local desktop app (macOS/Windows/Linux Tauri app), headless daemon binary (`codex_monitor_daemon`), or local source build; zero cloud dependencies, local files only.
- License: MIT License.
- Activity: 4.3k GitHub stars, latest release v0.7.67 / tag v0.7.68 (October 2026), active commits and rapid release cadence.
- Pros: Native Rust/Tauri architecture delivering high performance and minimal memory footprint; Seamless Codex app-server protocol integration with full subagent and fork support; Companion iOS app enabling remote monitoring over secure Tailscale mesh; Built-in local voice dictation via Whisper and deep GitHub PR/diff review tools.
- Cons: Does not offer a kanban/board visual tracking workflow for tasks; Dedicated exclusively to OpenAI Codex without support for OpenCode, Claude Code, or Aider; iOS app is currently in TestFlight/manual build rather than a public App Store release; Windows and iOS builds require complex local native toolchains (Rust, CMake).
- Sources: https://www.codexmonitor.app/, https://github.com/Dimillian/CodexMonitor, https://raw.githubusercontent.com/Dimillian/CodexMonitor/main/README.md, https://yetanotherorchestrator.app/apps/codexmonitor/
- Confidence: high

### Constellagent
- URL: https://constellagent.vercel.app/
- Kind: agent-native
- What: A macOS desktop orchestration environment for Claude Code that runs multiple isolated agent instances across dedicated git worktrees, Monaco code editors, and xterm.js terminals; KANBAN/board view of agent tasks: no (organizes work using a workspace list sidebar, multi-tab terminal/editor panes, and cron triggers without kanban columns).
- Agent integration: Claude Code CLI execution in dedicated PTYs (`claude '<prompt>'`), Claude Code lifecycle hooks (`Stop`, `Notification`, `UserPromptSubmit`, `PreToolUse`), Codex hook support (`notify.sh`), GitHub GraphQL API / `gh` CLI for PR checks, targets Anthropic Claude Code.
- Mobile: none (desktop macOS only, no iOS app, no Android app, no web/PWA interface).
- Pricing: free / open-source free ($0, ISC license, as of October 2026).
- Self-host: yes via local macOS Electron desktop app compiled with Bun; runs completely on local files, local PTYs, and local git worktrees.
- License: ISC License.
- Activity: 217 GitHub stars, initial release v1.0.0 (active repository with commits through May 2026), no formal GitHub release tags yet.
- Pros: True parallel execution of multiple Claude Code sessions with automatic git worktree isolation; Embedded full Monaco editor and real-time diff viewer alongside interactive terminal sessions; Built-in cron-like scheduling with wake-from-sleep auto-recovery for persistent agent loops; Automated PR monitoring with notification hooks when reviews or status checks fail.
- Cons: Lacks a kanban board view to track task statuses across backlog, in-progress, and done; Restricted to macOS with no Windows, Linux, or mobile support; Pre-built downloadable binaries are not published via GitHub Releases (requires manual build from source); Tailored specifically to Claude Code CLI with limited multi-agent platform interoperability.
- Sources: https://constellagent.vercel.app/, https://github.com/owengretzinger/constellagent, https://raw.githubusercontent.com/owengretzinger/constellagent/main/README.md, https://raw.githubusercontent.com/owengretzinger/constellagent/main/package.json, https://yetanotherorchestrator.app/apps/constellagent/
- Confidence: high

## EXPAND
- LEAD: Claude-Sync (https://github.com/owengretzinger/claude-sync) — WHY: Companion daemon by the Constellagent author that syncs Claude Code session state and hooks across machines. — ANGLE: Search for "Claude-Sync" agent state sync tools and hooks.
- LEAD: 21st.dev Magic Agent Dashboard — WHY: Agent visual task boards emerging from UI component generators. — ANGLE: Search yetanotherorchestrator.app for kanban-native agent orchestrators like AgentBoard or AutoCodeRover-Web.
