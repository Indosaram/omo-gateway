### Claude Squad
- URL: https://smtg-ai.github.io/claude-squad
- Kind: agent-native
- What: A terminal UI (TUI) and session manager built with Go, Bubble Tea, tmux, and git worktrees designed to orchestrate and monitor multiple concurrent AI terminal coding agents (Claude Code, Codex, Aider, Gemini, etc.) across isolated workspaces.
- Agent integration: CLI-based process execution inside managed tmux sessions. Agents are invoked as standard terminal CLI processes (configured via `program` or JSON profile entries); status is tracked non-invasively by scraping tmux pane output (`capture-pane`), computing git diffs (`git diff` and `--numstat`) per worktree, and detecting terminal prompts (e.g. matching prompt strings to automatically dismiss or accept prompts via AutoYes/Enter). No official REST/GraphQL API, no MCP server, and no webhook system exist.
- Mobile: none (desktop terminal TUI application requiring local tmux, git, and gh; no iOS/Android native app, no web UI server, and no PWA)
- Pricing: free / open-source free ($0; 100% free with no paid tiers, subscriptions, or enterprise pricing as of 2026-10-16)
- Self-host: yes (local-only CLI/TUI binary; installed via Homebrew `brew install claude-squad` or shell script into `~/.local/bin`; runs fully on the user's local machine with all state kept in `~/.claude-squad/config.json` and local git worktrees; no server or cloud required)
- License: GNU AGPL v3.0 (AGPL-3.0)
- Activity: 8,609 GitHub stars; 628 forks; last release v1.0.20 on 2026-08-20 (last pushed commit 2026-08-20); active open-source project
- Pros: Lightweight single-binary Go TUI with minimal resource overhead; seamless task isolation via automatic git worktrees and tmux sessions avoiding workspace branch collisions; built-in diff viewer and live preview pane to inspect agent changes in real-time; supports background daemon auto-prompting (experimental AutoYes / yolo mode) across Claude Code, Aider, and Gemini; zero configuration needed for local git repositories.
- Cons: Terminal-only interface with no kanban-style visual boards, drag-and-drop cards, or browser-based dashboard; strictly local with no mobile access (no native app, web interface, or remote sync); relies on tmux PTY pane scraping rather than structured agent protocol/MCP/API for state updates; limited to CLI-invoked coding agents and lacks team collaboration or multi-user sharing features.
- Sources: https://github.com/smtg-ai/claude-squad, https://smtg-ai.github.io/claude-squad/, https://api.github.com/repos/smtg-ai/claude-squad, https://api.github.com/repos/smtg-ai/claude-squad/releases/latest, https://formulae.brew.sh/formula/claude-squad
- Confidence: high (verified directly against official GitHub source code, repository metadata, release logs, and project documentation)

## EXPAND
- LEAD: CodeAgentSwarm and Pane — WHY: Direct desktop GUI alternatives to Claude Squad that manage parallel Claude Code/terminal agents with visual multi-pane/board interfaces instead of pure terminal tmux TUI — ANGLE: compare "CodeAgentSwarm" vs "Pane" vs "Claude Squad" agent workspace manager
