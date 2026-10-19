### Businessmap (formerly Kanbanize)
- URL: https://businessmap.io
- Kind: general-kanban-with-agent-integration
- What: Enterprise-grade Lean portfolio and multi-level Kanban platform designed to connect high-level strategy with day-to-day execution.
- Agent integration: Official MCP server (featured directly on the homepage for Claude and other LLMs to plan and manage boards via natural language), comprehensive REST API (API v2 supporting boards, cards, workflows, custom fields), webhooks, and automation rules; easily invoked by Claude Code, Codex, Cursor via MCP or custom REST calls.
- Mobile: Native iOS app (App Store: id563967812) and native Android app (Google Play: com.kanbanize.android), alongside responsive web interface.
- Pricing: No complex feature tiers (all core features included from day 1); scaled by user tiers: $10.60/user/month billed annually or $12.70/user/month billed monthly for 15-user starter tier (min 15 seats = $190/mo monthly or $1,896/yr annual; €9.20/user/month annual or €11.00/user/month monthly), volume discounts for larger teams; 14-day free trial, no permanent free tier (verified 2026-10-16).
- Self-host: no (strictly Cloud SaaS with dedicated cloud instances available in EU and USA data centers; legacy on-premise discontinued).
- License: Proprietary.
- Activity: Active commercial SaaS (active releases and marketing in 2026; proprietary closed source, community MCP implementations on GitHub like edicarloslds/businessmap-mcp).
- Pros: Native first-party MCP support for AI agent workflows; powerful multi-level swimlanes and portfolio hierarchy (Flight Levels); robust Lean/Agile analytics (CFD, Monte Carlo forecasts, cycle time).
- Cons: High barrier to entry with minimum 15 seats ($190/month minimum commitment); no free-forever plan; no self-hosted on-premise option.
- Sources: https://businessmap.io, https://businessmap.io/plans-pricing, https://businessmap.io/about/pricing-parameters, https://businessmap.io/api, https://apps.apple.com/us/app/kanbanize/id563967812, https://play.google.com/store/apps/details?id=com.kanbanize.android
- Confidence: high

### Kaiten
- URL: https://kaiten.ru
- Kind: general-kanban-with-agent-integration
- What: Russian full-featured project management and Kanban platform that visualizes multiple boards on a single workspace screen with built-in AI assistants and modular extensions.
- Agent integration: REST API (comprehensive token-based API for boards, cards, spaces, comments), inbound and outbound webhooks, Telegram bot integration, native AI assistant capabilities (summarization, speech-to-text, context Q&A), and upcoming official MCP/Skills integration advertised on their platform; agents can push/read cards via REST API or webhooks.
- Mobile: Native iOS app (App Store: id6739029367) and native Android app (Google Play: ru.kaiten.app), alongside responsive web app.
- Pricing: Permanent Free tier (up to 5 users, up to 3 spaces, 5 boards, basic Kanban/cards, 0 add-on modules); Paid tiers (billed monthly/annually): "Start" from 185 ₽/user/month (up to 15 users, includes Kanban & Scrum modules), "Standard" from 430 ₽/user/month (up to 250 users, 2 selectable modules), "Business" from 580 ₽/user/month (up to 250 users, 6 modules), "Corporation" custom pricing for 250+ users; 14-day full feature trial (verified 2026-10-16).
- Self-host: yes (available under the "Corporation" enterprise tier; delivered as a Docker container deployment with all dependencies packaged for on-premise servers, licensed from 300+ users).
- License: Proprietary.
- Activity: Active commercial platform (regular 2026 updates, member of Skolkovo and Russian software register, proprietary).
- Pros: Unique multi-board workspace view allows observing multiple agent boards simultaneously; native on-premise Docker deployment available for enterprises; generous free tier (up to 5 users).
- Cons: Primary market and UI focus is Russia/CIS (Russian documentation/servers under FZ-152); on-premise self-hosting is restricted to high-seat enterprise contracts (300+ users); official MCP/Skills support listed as coming soon rather than fully released.
- Sources: https://kaiten.ru, https://kaiten.ru/tariffs, https://apps.apple.com/us/app/kaiten/id6739029367, https://play.google.com/store/apps/details?id=ru.kaiten.app
- Confidence: high

### KanbanFlow
- URL: https://kanbanflow.com
- Kind: general-kanban
- What: Lightweight Lean Kanban board tool focused on real-time task visualization, work-in-progress (WIP) limits, and integrated Pomodoro and time tracking.
- Agent integration: REST API (API key-based HTTP REST API supporting boards, columns, swimlanes, tasks, subtasks, webhooks), incoming email task creation, and Zapier integration; AI agents (Claude Code, Cursor, Codex) can read board status and create/update/move tasks directly using standard HTTP curl / fetch calls against the REST API with an API token.
- Mobile: Dedicated mobile web application / PWA (accessible at kanbanflow.com/m with touch gestures and offline-friendly view); no native iOS or Android apps in Apple App Store or Google Play Store.
- Pricing: Free plan with no time limit (unlimited boards, unlimited tasks, WIP limits, subtasks, basic filters); Premium plan at $5/user/month (adds file attachments, custom fields, Gantt/timeline, cycle/lead time reports, Pomodoro stats, API & webhooks, integrations); 14-day free trial for Premium (verified 2026-10-16).
- Self-host: no (SaaS-only cloud platform managed by CodeFlow AB; no on-premise, Docker, or self-hosted distribution available).
- License: Proprietary.
- Activity: Active stable SaaS (long-standing operational service by CodeFlow AB in Sweden; stable, low-churn proprietary product).
- Pros: Very clean, distraction-free UI with virtually instant load times; built-in Pomodoro/time tracker useful for measuring agent execution time; affordable flat pricing ($5/user/month) with solid core REST API.
- Cons: No native mobile apps (mobile web app only); REST API and webhooks are locked behind the paid $5/user/month Premium plan (not accessible on Free tier); no self-hosting support.
- Sources: https://kanbanflow.com, https://kanbanflow.com/pricing, https://kanbanflow.com/mobile-app, https://kanbanflow.com/api-docs
- Confidence: high

### Kanban Tool
- URL: https://kanbantool.com
- Kind: general-kanban
- What: Online visual management and workflow automation platform offering real-time Kanban boards, swimlanes, time tracking, and customizable Power-Ups.
- Agent integration: REST API (API v1 supporting XML/JSON resource-oriented endpoints and API v3 returning flat JSON with batch card modifications), JavaScript SDK, webhooks, and "Developer Tools Power-Up" allowing custom scripts and CSS; AI agents can query and manipulate boards, columns, and cards via token-authenticated REST API calls or webhooks.
- Mobile: Native iOS app (App Store: id1096506294) and native Android app (Google Play: com.kanbantool), alongside responsive web design; compatible with both cloud and on-site self-hosted instances.
- Pricing: Free plan for up to 2 users and 2 boards (no file attachments); Team plan at $6/user/month (or €6/user/month) with unlimited boards and attachments; Enterprise plan at $11/user/month (or €11/user/month) adding time tracking, process automation, and user group management; 14-day free trial on cloud (verified 2026-10-16).
- Self-host: yes ("Kanban Tool On-Site" enterprise edition available for teams of 10+ users, deployed as virtual machine images for VirtualBox, VMware, QEMU/KVM with annual billing; works offline without external cloud dependencies).
- License: Proprietary.
- Activity: Active commercial product with over 25,000 corporate customers (developed by Shore Labs; ongoing updates across cloud and on-premise appliances).
- Pros: True self-hosted virtual appliance option (VirtualBox/VMware/KVM) for on-premise privacy; native mobile apps support connecting directly to self-hosted instances; generous two-user free tier for prototyping agent dashboards.
- Cons: UI and board design feels somewhat dated compared to modern 2026 agent-first board tools; on-site self-hosting requires annual enterprise billing with minimum 10 users; no out-of-the-box MCP server (agents must interact via raw REST API v3).
- Sources: https://kanbantool.com, https://kanbantool.com/pricing, https://kanbantool.com/kanban-tool-on-site, https://kanbantool.com/developer/
- Confidence: high

## EXPAND
- LEAD: Businessmap MCP implementation details — WHY: Businessmap is one of the very few enterprise Kanban tools actively advertising first-party MCP support on their homepage for LLMs — ANGLE: Search for official Businessmap MCP repository or remote endpoint specs and documentation.
- LEAD: Kan.bn and Planka open-source alternatives — WHY: For developers seeking free self-hosted AGPL/MIT open-source agent Kanban boards without enterprise user minimums — ANGLE: Survey GitHub stars, Docker compose setup, and REST/GraphQL APIs for plankanban/planka and kan.bn.
