### Restyaboard
- URL: https://restya.com/board/ (repo: https://github.com/RestyaPlatform/board)
- Kind: general-kanban
- What: A full-featured self-hosted open-source Trello clone built on PHP, PostgreSQL, Nginx, and Elasticsearch with extensive board customizations, offline cache, and apps/plugins ecosystem.
- Agent integration: REST API (comprehensive endpoints for boards, lists, cards, and activities at `/api/v1/`), webhooks, email-to-card parsing via IMAP; no official or community MCP server; accessible to Claude Code, Codex, Cursor, etc. via custom curl/HTTP scripts or custom tool calling against the REST API.
- Mobile: Responsive web interface (mobile browser optimized); previously advertised iOS app and Android APK in legacy marketplace documentation, but no active native apps are currently listed or maintained on modern Apple App Store or Google Play Store (cite: Apple App Store lookup returns 0 hits; Google Play URL returns 404 Not Found; Alibaba Cloud docs cite legacy mobile web/APK packaging).
- Pricing: Free community edition self-hosted under OSL-3.0; Commercial/Enterprise license pricing historically ranged from €7/user/month (annual commitments) up to €197/user/month for enterprise bundles and custom support tiers.
- Self-host: yes (Docker container via official Dockerfile/docker-compose, prebuilt Linux bash installer script `restyaboard.sh` targeting Debian/Ubuntu/CentOS with PHP-FPM, PostgreSQL, Nginx, and Elasticsearch; DigitalOcean 1-Click droplet and Cloudron package available).
- License: Dual License (Open Software License 3.0 / OSL-3.0 for community edition & Proprietary Commercial License for enterprise features/plugins).
- Activity: 2,089 GitHub stars, last tagged release v1.7.1 on 2022-03-14, last commit on dev branch 2023-10-26 (inactive / effectively maintenance-abandoned).
- Pros: Complete Trello feature parity with lists, cards, labels, and revisions; Comprehensive REST API covering all board actions; Self-hosted on standard Linux/Docker stacks without SaaS telemetry; Extensive plugin architecture for chat, webhooks, and LDAP.
- Cons: Technology stack is dated (PHP 5/7 era, Elasticsearch, jQuery/EJS/LESS); Inactive development with no releases since March 2022; Native mobile apps are defunct/unmaintained; Heavy resource footprint compared to modern single-binary or flat-file tools; No native MCP server or LLM agent tool definitions out of the box.
- Sources: https://github.com/RestyaPlatform/board, https://api.github.com/repos/RestyaPlatform/board, https://api.github.com/repos/RestyaPlatform/board/releases, https://marketplace.digitalocean.com/apps/restyaboard, https://forum.cloudron.io/topic/1202/restyaboard-trello-like-kanban-board, https://raw.githubusercontent.com/RestyaPlatform/board/master/server/php/R/r.php
- Confidence: high (verified GitHub repo, commits, release tags, API endpoints, mobile store 404s, and licensing)

### Kanri
- URL: https://kanriapp.com (repo: https://github.com/kanriapp/kanri)
- Kind: general-kanban
- What: An offline-first, privacy-focused cross-platform desktop Kanban board application designed for personal task management, built with Tauri, Rust, Nuxt 3, and Tailwind CSS.
- Agent integration: Local file-based integration via simple JSON; board data and configuration are stored locally in unencrypted `.json` files on the filesystem; no built-in REST API, daemon, or MCP server; AI agents (Claude Code, Cursor, Codex) can read and modify the local JSON board files directly with file read/write tools, but live updates require UI reloads.
- Mobile: none (desktop only; official installers provided exclusively for macOS `.dmg`/Homebrew cask, Windows `.exe`/`.msi`, and Linux `.deb`/`.rpm`/`.AppImage`; cite: https://kanriapp.com/download and GitHub release assets).
- Pricing: Free (100% free and open-source, no paid tiers, no cloud subscription).
- Self-host: yes (local desktop application running natively on macOS Apple Silicon/Intel, Windows x64, and Linux amd64/arm64; data lives entirely in local `.json` on user disk).
- License: GPL-3.0 (GNU General Public License v3.0 for application code; logo and brand assets are proprietary/copyright trobonox).
- Activity: 2,056 GitHub stars, active development; latest release app-v0.8.2 published on 2025-09-24, latest commit pushed on 2026-10-15.
- Pros: Sleek, lightweight, modern UI with smooth drag-and-drop; Completely offline with zero telemetry and zero external network dependencies; Low memory footprint thanks to Tauri/Rust foundation; Simple JSON storage schema easy for agents to parse and inspect offline.
- Cons: Desktop-only with no mobile apps (iOS/Android) or remote web browser access; No server daemon, WebSocket, or REST API for concurrent network access; Concurrent file writes by AI agents while the desktop app is open risk race conditions or file locking; Multi-user collaboration is not natively supported (single-user design).
- Sources: https://github.com/kanriapp/kanri, https://kanriapp.com/about, https://kanriapp.com/download, https://api.github.com/repos/kanriapp/kanri, https://api.github.com/repos/kanriapp/kanri/releases
- Confidence: high (verified official website, GitHub repo, release history up to v0.8.2, Tauri architecture, and license)

### Nullboard
- URL: https://nullboard.io (preview: https://nullboard.io/preview, repo: https://github.com/apankrat/nullboard)
- Kind: general-kanban
- What: An ultra-minimalist, single-file HTML/vanilla-JS Kanban board and task list manager focused on extreme compactness, keyboard-driven navigation, and zero-dependency local execution.
- Agent integration: REST-like HTTP synchronization protocol via local companion daemons (e.g. Nullboard Agent in C++, or `nullboard-agent-express` via Node.js running on port 10001); exposes PUT `/board/:id` and PUT `/config` with token auth (`X-Access-Token`); stores raw boards in JSON `.nbx` format; agents can push/pull board updates via HTTP to the local agent or write directly to `.nbx` JSON backup files; no native MCP server.
- Mobile: Responsive web (single HTML file renders in mobile Safari and mobile Chrome, but explicitly designed and optimized for keyboard and desktop mouse navigation; cite: author states "Essentially untested on mobile devices and against tap/touch input", no native iOS or Android app).
- Pricing: Free (completely free for personal and non-commercial internal use).
- Self-host: yes (single self-contained `nullboard.html` file runnable directly from local disk via `file:///`, hosted on any static HTTP server or Docker container, with companion local backup daemons for Windows, Linux, and macOS).
- License: BSD 2-Clause License with Commons Clause (source-available / non-commercial restriction: free to use, modify, and redistribute, but prohibits commercial sale or selling hosted services).
- Activity: 4,145 GitHub stars, last commit 2023-11-05 (stable/dormant; core single-file web app is considered feature-complete by author Alexander Pankratov).
- Pros: Extremely fast and lightweight with instantaneous page loads (single HTML file under 150 KB); Uncluttered compact UI fitting dozens of tasks and columns on one screen; Robust undo/redo history (last 50 revisions preserved); Optional local companion daemon provides automated backup and HTTP synchronization.
- Cons: Commons Clause license prevents commercial SaaS distribution or integration into commercial offerings; Mobile touch experience is secondary and clumsy without keyboard shortcuts; Browser localStorage storage model risks accidental data loss if cache is cleared without the backup daemon; No built-in authentication, user access control, or concurrent multi-agent conflict resolution.
- Sources: https://github.com/apankrat/nullboard, https://api.github.com/repos/apankrat/nullboard, https://nullboard.io, https://raw.githubusercontent.com/apankrat/nullboard/master/nullboard.html, https://github.com/justinpchang/nullboard-agent-express
- Confidence: high (verified HTML source code, licensing header, storage class implementation, companion agent Express API, and GitHub metrics)

### Kanbn CLI
- URL: https://github.com/basementuniverse/kanbn (MCP wrapper: https://github.com/kjduling/kanbn-mcp)
- Kind: general-kanban-with-agent-integration
- What: A Git-friendly terminal-based Kanban CLI application and library that stores boards, columns, and tasks entirely as human-readable Markdown files with YAML front-matter inside a project repository.
- Agent integration: Native CLI (`kanbn add`, `move`, `task`, `find`, `status`), official Node.js library API (`@basementuniverse/kanbn`), first-party VS Code extension (`vscode-kanbn`), and a dedicated community Model Context Protocol server (`kanbn-mcp` / `@kduling/kanbn-mcp`); fully supports Claude Code, Cursor, Codex, Windsurf, Cline, and OpenCode via standard stdio MCP tools (`kanbn_create_task`, `kanbn_move_task`, `kanbn_status`, etc.) and automated guidance injection (`AGENTS.md`, `CLAUDE.md`, `.clinerules`).
- Mobile: none (command-line interface and editor-bound extension only; no mobile apps, no responsive web dashboard unless rendered into static markdown previews on GitHub/GitLab mobile).
- Pricing: Free (100% free, open-source under MIT).
- Self-host: yes (local CLI binary installed via `npm install -g @basementuniverse/kanbn`, local repository markdown files inside `.kanbn/` directory; zero cloud dependency).
- License: MIT License.
- Activity: 45 GitHub stars for core CLI (plus 71 stars for VS Code extension), actively maintained; latest core release v2.5.3 on 2026-09-07, latest commit pushed on 2026-10-02; companion `kanbn-mcp` actively released with updates in October 2026.
- Pros: First-class AI agent integration with a dedicated Model Context Protocol (MCP) server and automated agent rules generator; Pure Markdown + YAML front-matter storage natively versioned in Git alongside source code; Feature-packed CLI supporting sprints, burndown charts, Gantt charts, subtasks, and custom metadata; Frictionless developer UX via VS Code interactive visual board extension (`vscode-kanbn`).
- Cons: No standalone standalone web server or native mobile application for on-the-go phone review; Concurrent edits in distributed teams can lead to git merge conflicts on `.kanbn/index.md`; Small community compared to enterprise boards; Large task boards can grow `.kanbn` directory file counts substantially.
- Sources: https://github.com/basementuniverse/kanbn, https://api.github.com/repos/basementuniverse/kanbn, https://api.github.com/repos/basementuniverse/kanbn/releases, https://github.com/kjduling/kanbn-mcp, https://github.com/basementuniverse/vscode-kanbn, https://raw.githubusercontent.com/kjduling/kanbn-mcp/main/README.md
- Confidence: high (verified npm packages, GitHub releases through 2.5.3 in September 2026, MIT license, full MCP schema, and CLI capabilities)

## EXPAND
- LEAD: Planka — WHY: Modern open-source self-hosted Trello alternative built on React/Node.js/PostgreSQL with active 2026 maintenance, active community MCP servers, and responsive web / PWA support. — ANGLE: web_search("planka kanban github mcp react")
- LEAD: Kanboard — WHY: Classic PHP/SQLite/MySQL self-hosted kanban with an extensive JSON-RPC API and multiple community CLI/MCP bridges, heavily deployed in homelabs. — ANGLE: web_search("kanboard json-rpc api mcp")
- LEAD: Vikunja — WHY: Full-featured modern open-source to-do and kanban platform written in Go and Vue with official REST API, CalDAV sync, webhooks, and modern mobile PWAs. — ANGLE: web_search("vikunja kanban open source api mobile")
