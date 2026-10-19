### Herdr
- URL: https://herdr.dev/
- Kind: agent-native
- What: Herdr is an agent-native terminal runtime and multiplexer daemon written in Rust that manages persistent agent sessions across panes and workspaces; it does NOT feature a built-in kanban or board view (no kanban board view, uses hierarchical workspace/tab/pane list and status sidebar, though third-party plugins can render custom panes).
- Agent integration: Local Unix domain socket / Windows named pipe REST-style JSON-RPC API, dedicated CLI (`herdr agent`, `herdr pane`, `herdr workspace`), native detection manifests with automated remote updates from herdr.dev, and agent skills (`add-skill subsy/ralph-tui` or Herdr skill guide); official hooks/integrations for 22+ agents including Claude Code, Codex, OpenCode, Cursor Agent CLI, Pi, Gemini CLI, Droid, Grok CLI, Kimi, Kilo, Devin CLI, Copilot CLI, Hermes, plus self-reporting agents (Crush, Prime Agent, Command Code, Kiro CLI).
- Mobile: None native; responsive web/PWA none (accessible on mobile devices solely via terminal SSH clients like Termius/Blink connecting to the persistent daemon).
- Pricing: Free and open-source (Core CLI/runtime); raised a $6M seed round led by Bessemer Venture Partners with YC in 2026; "Herdr Cloud" (hosted zero-SSH machine interconnect) is in waitlist/coming soon (commercial pricing not yet published as of October 2026).
- Self-host: Yes; 100% locally self-hosted single compiled binary (~10MB Rust) across macOS, Linux, and Windows (ConPTY), supporting remote machine clustering via standard OpenSSH.
- License: Apache-2.0 (source code repository https://github.com/herdrdev/herdr; directory listing at yetanotherorchestrator.app states AGPL-3.0 historically/erroneously, but official GitHub repo HEAD explicitly specifies Apache-2.0).
- Activity: Active; latest release v0.9.5 (hotfix) / v0.9.4 in October 2026, with active daily development and preview builds (e.g. preview build 2026-10-14-fbd2ae23927f).
- Pros: True persistent agent runtime where agent sessions survive client detach/terminal reboots; Automatic semantic state inference (idle, working, blocked) via terminal screen inspection; Robust bidirectional JSON socket API and CLI enabling agents to orchestrate other agents; Multi-machine federation over SSH without vendor lock-in.
- Cons: Lacks a visual kanban or board UI natively; Strict terminal/CLI environment with no desktop GUI or native mobile application; Screen-scraping heuristic detection can occasionally misclassify custom or changing agent prompt formats.
- Sources: https://herdr.dev/, https://herdr.dev/docs/, https://herdr.dev/docs/agents/, https://herdr.dev/docs/socket-api/, https://herdr.dev/docs/connecting-machines/, https://herdr.dev/blog/herdr-raised-a-seed/, https://herdr.dev/compare/, https://github.com/herdrdev/herdr, https://github.com/herdrdev/herdr/releases, https://yetanotherorchestrator.app/apps/herdr/
- Confidence: high

### OpenKanban
- URL: https://github.com/TechDufus/openkanban
- Kind: agent-native
- What: OpenKanban is an open-source terminal UI (TUI) kanban board designed specifically for orchestrating multiple AI coding agents in parallel; it HAS a native visual kanban board view (yes kanban view) with columns (Backlog, In Progress, Done), ticket cards, and embedded terminal worktrees.
- Agent integration: Direct CLI agent execution per ticket (embedded PTY), configuration-driven via `~/.config/openkanban/config.json`, built-in shared server integration for OpenCode (status detection on port 4096), live status hook via `oh-my-claude` plugin writing to `~/.cache/openkanban-status/` for Claude Code, and support for any CLI agent (Gemini CLI, Codex, Aider, OpenCode, Claude Code) with ticket context interpolation (`{{.Title}}`, `{{.Description}}`, `{{.BranchName}}`).
- Mobile: None (terminal UI only; no native iOS/Android app, no PWA, no web interface; accessible on mobile only over SSH terminal).
- Pricing: Free and 100% open-source (no paid tiers or user licenses as of October 2026).
- Self-host: Yes; locally installed standalone Go binary (via Homebrew `brew install TechDufus/tap/openkanban` or `go install`). Local-only filesystem storage (`~/.config/openkanban/`).
- License: AGPL-3.0 (GNU Affero General Public License v3.0).
- Activity: Active; version v0.1.1 released in early 2026, continuous patch releases through v0.0.33–v0.1.1 addressing PTY terminal handling, scrollback, and multi-project support.
- Pros: Dedicated terminal-native kanban board interface designed specifically for tracking AI coding tasks; Automatic git worktree and branch isolation per kanban ticket; Embedded terminal allows immediate inspection and manual attachment (`enter`) to any running agent; Deep zero-config live status integration with OpenCode server and Claude Code (`oh-my-claude`).
- Cons: Terminal only with no mobile/web dashboard; Lacks external REST/MCP socket daemon for external agent manipulation; Limited team collaboration capabilities (strictly local git repo workflow).
- Sources: https://github.com/TechDufus/openkanban, https://github.com/TechDufus/openkanban/blob/main/README.md, https://github.com/TechDufus/openkanban/blob/main/docs/CONFIGURATION.md, https://github.com/TechDufus/openkanban/blob/main/LICENSE, https://github.com/TechDufus/openkanban/releases, https://yetanotherorchestrator.app/apps/openkanban/
- Confidence: high

### Ralph TUI
- URL: https://ralph-tui.com/
- Kind: agent-native
- What: Ralph TUI is an autonomous AI agent loop orchestrator and terminal interface implementing Geoffrey Huntley's 'Ralph Wiggum loop' pattern to run AI agents through backlogs until completion; it does NOT feature a traditional column-based kanban board view (no kanban board view; uses sequential backlog task lists, status dashboards, subagent execution tree panels, and PRD iteration views).
- Agent integration: Pluggable agent architecture with CLI runners for Claude Code, OpenCode, Factory Droid, Cursor CLI, Gemini CLI, Codex, and Kiro CLI; plugin task trackers (`prd.json`, git-backed Beads issue tracker, Beads-BV, Beads-Rust, Linear, Jira Cloud); slash commands and bundled agent skills installed via Vercel's `add-skill` (`/ralph-tui-prd`, `/ralph-tui-create-json`, `/ralph-tui-create-beads`); WebSocket remote server/client protocol (`run --listen` on port 7890 with two-tier token auth).
- Mobile: None native; no responsive web UI or PWA (terminal UI only, though its remote WebSocket listener architecture allows headless daemon deployment and remote TUI tab federation).
- Pricing: Free and open-source (no paid tier, community open-source project).
- Self-host: Yes; self-hosted open-source software built with TypeScript/Bun and OpenTUI, distributed via npm (`bun install -g ralph-tui`), run locally or in headless/daemon remote mode (`--listen`).
- License: MIT License.
- Activity: Highly active; latest major release v0.12.0 (May 2026) following frequent releases v0.4.0–v0.11.0 in early 2026; active GitHub repository at https://github.com/subsy/ralph-tui.
- Pros: Sophisticated autonomous execution loop (select-prompt-execute-evaluate) with completion detection; Extensive issue tracker compatibility (Beads graph dependencies, prd.json, Linear, Jira); Built-in multi-epic parallel worktrees with AI conflict resolution; Remote instance federation via secure token-authenticated WebSockets and multi-tab TUI.
- Cons: No kanban card/column board view (relies on list and tree dashboard views); Steep learning curve around PRD generation and Beads dependencies; Primarily Node/Bun-based terminal dependency stack.
- Sources: https://ralph-tui.com/, https://github.com/subsy/ralph-tui, https://github.com/subsy/ralph-tui/releases, https://yetanotherorchestrator.app/apps/ralph-tui/
- Confidence: high

## EXPAND
- LEAD: Agent Deck, Agent of Empires, and Conductor/Emdash/Superset — WHY: Surveying agent task orchestration tools highlights a clear divergence between TUI multiplexers (Herdr), TUI task boards (OpenKanban), autonomous loop trackers (Ralph TUI), and full GUI/web-based kanban boards (Agent of Empires, Emdash, Conductor) which offer native browser/mobile accessibility — ANGLE: web search "Agent of Empires orchestrator kanban" OR "Emdash agent kanban review"
