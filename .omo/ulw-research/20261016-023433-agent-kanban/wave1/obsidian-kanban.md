### Obsidian Kanban Plugin
- URL: https://github.com/community-archive/obsidian-kanban
- Kind: general-kanban
- What: A community plugin for Obsidian that transforms Markdown lists within Obsidian vaults into interactive, drag-and-drop Kanban boards using Obsidian's native editor.
- Agent integration: Direct file access (no native agent integration, no built-in MCP server, and no official CLI/REST API). AI coding agents like Claude Code, Codex, or Cursor interact by directly reading, creating, and modifying local Markdown board files in Obsidian vault storage using standard file read/write/edit tools or markdown parsers.
- Mobile: Native iOS app and native Android app supported via official Obsidian mobile applications (plugins run inside Obsidian mobile via the mobile plugin architecture; cite: https://obsidian.md/mobile).
- Pricing: Free / open-source free ($0; Obsidian itself is free for personal and commercial use forever with optional Catalyst insider license at $25 one-time, optional Sync at $4-$5/month, and optional Publish at $8-$10/site/month as of 2026-10-16; cite: https://obsidian.md/pricing).
- Self-host: Yes, via local desktop/mobile app and local-only markdown files stored in user-controlled folders; syncable via Git, iCloud, Syncthing, or Obsidian Sync.
- License: GNU General Public License v3.0 (GPL-3.0).
- Activity: 4,530 GitHub stars; last release v2.0.51 on May 31, 2024; last repository commit on March 6, 2026; project transitioned to community-archive in January 2026 seeking maintainers.
- Pros: True Markdown single file format (`kanban-plugin: basic` YAML frontmatter and `##` lanes); 100% offline and local-first with zero tracking; runs seamlessly across desktop and mobile Obsidian; broad plugin interoperability (Dataview, Tasks plugin integration).
- Cons: Lacks agent-native interfaces (no MCP server, CLI, or task lease/locking protocol); concurrent writes between an AI agent and open Obsidian UI can trigger file sync conflict loops; maintainer stepped down in 2026 leaving plugin under community archive governance.
- Sources: https://github.com/community-archive/obsidian-kanban, https://publish.obsidian.md/kanban/, https://obsidian.md/pricing, https://obsidian.md/mobile, https://github.com/community-archive/obsidian-kanban/releases/tag/2.0.51, https://api.github.com/repos/community-archive/obsidian-kanban
- Confidence: high

### kanban-md
- URL: https://github.com/antopolskiy/kanban-md
- Kind: agent-native
- What: An agent-first, file-based Kanban board engine and interactive TUI built in Go for coordinating autonomous AI coding loops, human supervisors, and multi-agent systems.
- Agent integration: CLI and pre-built installable agent skills (`kanban-md skill install` for Claude Code, Codex, Cursor, and OpenClaw). Supports compact token-efficient CLI output (`--compact`), JSON output (`--json`), cooperative lease-based task claims (`--claim <agent-name>`), claim expiration timeouts, and automatic context generation (`kanban-md context --write-to AGENTS.md`).
- Mobile: None natively; accessible on mobile devices only via SSH into a workstation or server running the narrow-mode terminal TUI (`kanban-md tui --narrow`).
- Pricing: Open-source free ($0; no SaaS tiers, no cloud service, no paywalls).
- Self-host: Yes, via standalone Go binary on macOS, Linux, and Windows (Homebrew tap `antopolskiy/tap/kanban-md` or pre-built binaries); 100% local filesystem storage.
- License: MIT License.
- Activity: 224 GitHub stars; last release v0.39.0 ("Field Notes") on October 5, 2026; last commit on October 5, 2026; active development with regular updates.
- Pros: Built specifically for agentic loops with multi-agent lease locking (`--claim`); token-thrifty `--compact` and `--json` outputs protect LLM context windows; full-featured terminal UI with mouse support and narrow-width responsive layout; self-healing task IDs and cyclical dependency guards.
- Cons: No native mobile apps or hosted web GUI out of the box; lacks an official MCP server wrapper (agents run shell CLI commands or read markdown files directly); requires installing a Go binary on each machine.
- Sources: https://github.com/antopolskiy/kanban-md, https://github.com/antopolskiy/kanban-md/releases/tag/v0.39.0, https://api.github.com/repos/antopolskiy/kanban-md
- Confidence: high

### VS Code Agent Kanban / AgentKanban.io
- URL: https://agentkanban.io
- Kind: agent-native
- What: A specialized task board system and VS Code extension providing an integrated Kanban board, human-in-the-loop plan/todo/implement workflows, and Git worktree isolation for AI coding agents.
- Agent integration: Official MCP server (`mcp.agentkanban.io` / local MCP), VS Code GitHub Copilot Chat participant (`@kanban /task`, `@kanban /refresh`, `@kanban /worktree`), REST API, webhooks, and automatic system-prompt context injection via `AGENTS.md` and `INSTRUCTION.md` sentinel markers. Compatible with Claude Code, GitHub Copilot, Codex, Cursor, and any MCP client.
- Mobile: Responsive web application for cloud boards on agentkanban.io; no standalone native iOS/Android mobile app.
- Pricing: Free tier ($0 forever: 3 members, 3 boards, 30 tasks, 1 API key, 1 webhook, 250 API req/day, 14-day retention); Pro plan at $10/seat/month (up to 100 boards, 10,000 tasks, 50 API keys, 25 webhooks, 25,000 API req/day, 365-day retention as of 2026-10-16). Local-only VS Code workflow remains free and open.
- Self-host: Yes for the local VS Code extension (local markdown task files in `.agentkanban/tasks/` and local git worktrees); No for the hosted AgentKanban.io team cloud platform (proprietary SaaS).
- License: Elastic License 2.0 (ELv2) for the VS Code extension codebase; proprietary for the hosted cloud backend.
- Activity: 60 GitHub stars on `appsoftwareltd/vscode-agent-kanban`; v2.1.3 released May 12, 2026; commits and hosted platform active through mid-2026.
- Pros: Native MCP server integration allows Claude Code, Cursor, and Codex to read and transition tasks directly; built-in automated Git worktree provisioning isolates agent edits to dedicated branches; sentinel-managed `AGENTS.md` injection solves context window decay in long agent turns.
- Cons: Cloud collaboration features gated behind proprietary SaaS ($10/seat/month); Elastic License 2.0 limits commercial re-distribution of the extension; no dedicated native mobile app (browser only for mobile viewing).
- Sources: https://agentkanban.io, https://agentkanban.io/pricing, https://github.com/appsoftwareltd/vscode-agent-kanban, https://marketplace.visualstudio.com/items?itemName=AppSoftwareLtd.vscode-agent-kanban, https://api.github.com/repos/appsoftwareltd/vscode-agent-kanban
- Confidence: high

### Unified Kanban (VS Code Extension)
- URL: https://github.com/divisionseven/unified-kanban
- Kind: general-kanban
- What: A VS Code extension that renders interactive Kanban boards inside Visual Studio Code using the exact Obsidian Kanban Markdown format, providing format parity between Obsidian vaults and VS Code workspaces.
- Agent integration: Direct file modification (no dedicated MCP server, CLI, or API). Agents (Claude Code, Cursor, Codex) read and update the markdown board file (`kanban-plugin: basic`, `##` lanes, `- [ ]` tasks) directly using file system editing tools.
- Mobile: None (VS Code desktop extension); however, files created by Unified Kanban can be opened directly on mobile devices using Obsidian's iOS or Android app.
- Pricing: Open-source free ($0; available free on VS Code Marketplace and Open VSX Registry).
- Self-host: Yes, runs purely as a local extension inside VS Code on the user's computer; files are local Markdown files tracked in Git.
- License: MIT License.
- Activity: 7 GitHub stars, 1,448 Open VSX downloads; v0.7.3 released in April 2026; last repository commit August 30, 2026.
- Pros: 100% syntax compatibility with the Obsidian Kanban plugin (`kanban-plugin: basic`); board, table, and list view modes inside VS Code; supports tags, dates, and wikilinks; zero cloud dependencies.
- Cons: General-purpose developer tool without agent-aware hooks or locking mechanisms; concurrent edits between an agent and the VS Code webview can cause reload race conditions; no built-in agent CLI or MCP protocol support.
- Sources: https://github.com/divisionseven/unified-kanban, https://open-vsx.org/extension/divisionseven/unified-kanban, https://open-vsx.org/api/divisionseven/unified-kanban/0.7.3/file/readme.md, https://api.github.com/repos/divisionseven/unified-kanban
- Confidence: high

### AGINEAR
- URL: https://github.com/Legorobotdude/AGINEAR
- Kind: agent-native
- What: A markdown-native project management tool combining a CLI, React web GUI Kanban board, and built-in stdio Model Context Protocol (MCP) server that treats `todo.md` and `roadmap.md` as the single source of truth.
- Agent integration: Official MCP server (stdio transport via `@modelcontextprotocol/sdk`) exposing structured tools (`get_todos`, `add_task`, `complete_task`, `move_task`) to Claude Code, Claude Desktop, and Cursor; companion CLI (`npx aginear list`, `add`, `done`, `move`); WebSocket live updates on file changes.
- Mobile: Responsive web application accessed via local network browser (`http://<host>:4040` started with `aginear serve`); no native mobile apps.
- Pricing: Open-source free ($0; 100% free under MIT License).
- Self-host: Yes, via Node.js / npx (`npx aginear init`, `npx aginear serve`); completely local, storing data in repo-level `todo.md` and `roadmap.md`.
- License: MIT License.
- Activity: Created February 2026; initial public releases and blog post published February 2026; last commit July 9, 2026.
- Pros: Native MCP server specifically engineered for AI coding agents to eliminate token bloat from raw file dumps; custom concrete syntax tree (CST) parser guarantees lossless byte-for-byte markdown preservation; bundled drag-and-drop web GUI with real-time WebSocket file-watcher sync.
- Cons: Small community and early-stage project (1 star, solo author); lacks multi-agent claim leases or collision locking found in kanban-md; no native mobile app.
- Sources: https://www.adityabawankule.io/projects/aginear, https://github.com/Legorobotdude/AGINEAR, https://api.github.com/repos/Legorobotdude/AGINEAR
- Confidence: high

### vscode-kanban (mkloubert)
- URL: https://github.com/mkloubert/vscode-kanban
- Kind: general-kanban
- What: A classic Kanban board extension for Visual Studio Code supporting Markdown card descriptions, Mermaid diagrams, Toggl time tracking, and programmable JavaScript event automation.
- Agent integration: File-based and script-based hooks (`.vscode/vscode-kanban.json` data store and optional exported `.md` files; `.vscode/vscode-kanban.js` lifecycle event scripts). No native MCP server or LLM agent tool calling interface.
- Mobile: None (VS Code desktop extension only).
- Pricing: Open-source free ($0; free on VS Code Marketplace).
- Self-host: Yes, runs locally within VS Code; data stored in `.vscode/` workspace folder.
- License: GNU Lesser General Public License v3.0 (LGPL-3.0).
- Activity: 165+ GitHub stars; repository archived and marked deprecated by owner on November 7, 2022; remains available on VS Code marketplace for legacy installations.
- Pros: Custom event scripting (`vscode-kanban.js`) allows triggering arbitrary Node.js scripts when cards move; built-in Mermaid diagram and Toggl time tracking support; exports cards to individual markdown files.
- Cons: Project archived and unmaintained since 2022; uses JSON as primary persistence format rather than native human-friendly Markdown lists; completely unaware of modern AI agent workflows and lacks MCP.
- Sources: https://github.com/mkloubert/vscode-kanban, https://marketplace.visualstudio.com/items?itemName=mkloubert.vscode-kanban, https://raw.githubusercontent.com/mkloubert/vscode-kanban/master/package.json
- Confidence: high

## EXPAND
- LEAD: Kanbn CLI / vscode-kanbn (by Gordon Larrigan) — WHY: Widely cited Git-friendly markdown-based kanban tool using `.kanbn/` index and task files, but repository and marketplace pages show intermittent availability in 2026. — ANGLE: Search for forks or current npm package status (`npm info kanbn`).
- LEAD: nod (CLI markdown kanban for Claude Code) — WHY: Mentioned in ClaudeAI community discussions as a minimalist task manager built natively for Claude Code markdown workflows. — ANGLE: Search `github.com` and Reddit for `"nod" kanban markdown claude code`.
- LEAD: Kanban MCP servers (kanbanthing, mdskills) — WHY: Hosted or standalone MCP servers offering task boards without local markdown files. — ANGLE: Investigate whether database-backed MCP kanbans compare favorably to repo-level Markdown files for agent teams.
