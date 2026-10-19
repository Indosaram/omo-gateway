# Multi-Agent Frameworks & Kanban Task Board Survey

Seen: Trello, Jira, Linear, Notion, Asana, ClickUp, monday, Plane, WeKan, Kanboard, Planka, Vikunja, Focalboard, Taiga, Huly, Vibe Kanban, Task Master, Backlog.md.

> **Survey Focus & Finding**: Do mainstream multi-agent frameworks (CrewAI, AutoGen, LangGraph, Agno, MetaGPT, ChatDev, OpenAI Agents SDK, Dify) provide kanban task views?
> **Answer**: None of the mainstream multi-agent frameworks provide native Kanban task board UIs out of the box. Instead, they center their visual interfaces around **DAG workflow graphs / node-edge canvases** (LangGraph Studio, Dify, ChatDev 2.0), **hierarchical execution trace trees / waterfall timelines** (LangSmith, OpenAI Agents Tracing, CrewAI AMP, Agno AgentOS), or **chat playgrounds / form cards** (AutoGen Studio, MetaGPT/MGX).
> However, this gap has fueled an active 2025–2026 wave of **agent-native Kanban frameworks and orchestration platforms** (such as KaibanJS, Routa, Agent Teams AI, Kandev, Dev 3.0, Veritas Kanban, OpenKanban, AIF Handoff, Chorus, and It's a Plan) that treat Kanban boards as the primary control plane for multi-agent execution.

---

### CrewAI AMP & Studio
- URL: https://crewai.com/amp
- Kind: agent-native
- What: An enterprise agent management platform and visual studio for orchestrating, deploying, and monitoring role-based multi-agent teams. It does NOT offer a Kanban board, structuring executions through sequential/hierarchical pipelines, state-driven Flows, and execution trace trees.
- Agent integration: Python SDK (`crewai`), CLI (`crewai CLI`), REST API on AMP, official CrewAI docs MCP server (`https://docs.crewai.com/mcp`), Claude Code skills plugin (`crewai-skills`), Cursor/Windsurf skills.
- Mobile: responsive web (AMP web dashboard at app.crewai.com is responsive in browser; no native iOS or Android app)
- Pricing: open-source free (core framework); AMP cloud has a free tier at app.crewai.com, paid plans and custom Enterprise tiers on inquiry (as of Oct 2026)
- Self-host: yes via Python package (`uv tool install crewai`) for core; on-premise AMP deployment is restricted to Enterprise licenses
- License: MIT License (core framework); proprietary (AMP platform)
- Activity: 59,750 GitHub stars, active daily development, latest release v0.102.0+
- Pros: Leading role-based multi-agent collaboration framework with autonomous task delegation; Event-driven Flows for deterministic business logic and state routing; Deep execution traces and observability in AMP console; First-class coding agent skills for Claude Code and Cursor.
- Cons: Completely lacks a Kanban task board interface for tracking card progression; AMP Enterprise on-premise control plane is commercial and closed-source; Debugging focuses on log traces rather than interactive visual boards; Heavy Python ecosystem dependency.
- Sources: https://github.com/crewAIInc/crewAI, https://docs.crewai.com, https://docs-platform.crewai.com/, https://crewai.com/amp, https://crewai.com/pricing
- Confidence: high

### AutoGen Studio
- URL: https://microsoft.github.io/autogen/stable/user-guide/autogenstudio-user-guide/index.html
- Kind: agent-native
- What: A no-code prototyping graphical interface built on Microsoft AutoGen for composing agents, managing skills, and running playground chat sessions. It does NOT provide a Kanban board view, relying instead on a multi-agent chat session view, skill cards, and workflow configuration forms.
- Agent integration: Python SDK (`autogen-agentchat`, `autogen-core`), local CLI (`autogenstudio ui`), REST API; MCP client integration via `autogen_ext.tools.mcp` (McpWorkbench); supports OpenAI, Azure, and local LLMs.
- Mobile: none (local web UI at localhost:8080; desktop-first layout without native iOS or Android app)
- Pricing: open-source free (100% free under MIT License; no paid tiers)
- Self-host: yes via Python package (`pip install autogenstudio`) with local SQLite or PostgreSQL backend dialects
- License: MIT License / CC-BY-4.0 documentation
- Activity: 61,434 GitHub stars (monorepo); status is maintenance mode as Microsoft transitions active investment to Microsoft Agent Framework (MAF)
- Pros: 100% free and open-source local UI for rapid multi-agent prototyping; SQLModel-backed storage layer supporting SQLite and PostgreSQL; Clean integration with AutoGen 0.4 AgentChat primitives; Built-in MCP workbench support for external tool connections.
- Cons: No Kanban task board or column-based task lifecycle view; Framework is placed into maintenance mode in favor of Microsoft Agent Framework; Explicitly documented as a research prototype not meant for production; No native mobile app or PWA capabilities.
- Sources: https://github.com/microsoft/autogen, https://raw.githubusercontent.com/microsoft/autogen/main/python/packages/autogen-studio/README.md
- Confidence: high

### LangGraph Studio & LangSmith
- URL: https://smith.langchain.com
- Kind: agent-native
- What: A specialized agent IDE (LangGraph Studio) and production observability platform (LangSmith) for cyclical, stateful agent graphs. It does NOT provide a Kanban board view, visualizing agent executions strictly as interactive node-edge DAG graphs, time-travel state steps, and waterfall trace trees.
- Agent integration: Python/TypeScript LangGraph SDKs, LangGraph CLI, LangSmith REST API, LangChain MCP server integration, environment variable auto-instrumentation; supports Claude Code, Cursor, Codex, OpenAI, and custom agents.
- Mobile: none (macOS desktop app and cloud web dashboard at smith.langchain.com; no native iOS or Android apps)
- Pricing: LangGraph framework is open-source free; LangSmith has a Developer free tier (5,000 traces/mo, 1 seat at $0), Plus tier at $39/seat/mo, and custom Enterprise tier (as of Oct 2026)
- Self-host: yes for LangGraph framework (local execution); LangSmith self-hosted instance is available for Enterprise customers via Docker/Kubernetes
- License: MIT License (LangGraph OSS); proprietary (LangSmith Cloud and Enterprise)
- Activity: 43,332 GitHub stars (LangGraph repository), highly active daily development and releases
- Pros: Industry standard for stateful, cyclical graph-based multi-agent orchestration; Visual interactive graph debugger with time-travel state inspection and rewinds; Deep observability with waterfall traces, token usage, and latency metrics; One-click deployment from Studio directly to LangSmith Cloud.
- Cons: No Kanban task board view (strictly node-edge graphs and linear thread traces); LangSmith self-hosting is restricted behind expensive Enterprise contracts; Steep learning curve for graph primitives and channel state reducers; Desktop application is primarily macOS-focused.
- Sources: https://github.com/langchain-ai/langgraph, https://docs.langchain.com/langsmith/studio, https://docs.smith.langchain.com/, https://www.langchain.com/pricing
- Confidence: high

### Agno AgentOS UI
- URL: https://agno.com
- Kind: agent-native
- What: A runtime and control plane UI for multi-agent platforms that allows developers to run agents as services with durable execution and multi-replica scaling. It does NOT provide a Kanban board, offering instead session logs, run tracing, and API/MCP endpoint management.
- Agent integration: Agno Python SDK (`agno`), AgentOS runtime, official MCP server (`mcp.agno.com`), REST API, interfaces for Slack/WhatsApp/Telegram; supports Claude Agent SDK, Codex SDK, and OpenAI.
- Mobile: responsive web (AgentOS web UI on app.agno.com; no native mobile app)
- Pricing: open-source free (SDK and AgentOS runtime templates); Agno cloud control plane has a free developer tier with paid enterprise tiers for multi-replica scaling (as of Oct 2026)
- Self-host: yes via Docker Compose, Railway, or AWS templates with PostgreSQL and FastAPI
- License: Apache-2.0
- Activity: 42,760 GitHub stars, active daily development and frequent releases
- Pros: Fast and lightweight Python agent runtime with minimal orchestration overhead; Built-in MCP server integration for exposing agents to Claude and ChatGPT; Complete ownership of data, memory, and JWT-based RBAC in self-hosted setups; Strong multi-modal and structured output capabilities.
- Cons: No Kanban task board or column-based work visualizer; UI is focused on operational monitoring rather than task management; Rebranded from Phidata causing legacy documentation divergence; Lacks native mobile applications.
- Sources: https://github.com/agno-agi/agno, https://docs.agno.com, https://agno.com
- Confidence: high

### MetaGPT & MGX
- URL: https://mgx.dev
- Kind: agent-native
- What: A multi-agent framework that models a virtual software company through standardized operating procedures (SOPs), alongside its commercial SaaS evolution MGX (Atoms.dev) for natural language product development. It does NOT have a Kanban board, structuring work as sequential SOP artifacts (PRD, System Design, Code, Review) and conversational builders.
- Agent integration: Python SDK (`metagpt`), CLI (`metagpt`), REST API, Git integration, webhook triggers; integrates with OpenAI, Claude, Gemini, and open-source LLMs.
- Mobile: none (web platform only at mgx.dev / atoms.dev; no native mobile apps)
- Pricing: open-source free (MetaGPT OSS under MIT License); MGX / Atoms.dev operates on a freemium SaaS model with subscription credits for cloud app building (as of Oct 2026)
- Self-host: yes for MetaGPT (local Python/Docker); MGX/Atoms cloud platform is proprietary hosted
- License: MIT License (MetaGPT OSS); proprietary (MGX / Atoms.dev)
- Activity: 70,873 GitHub stars, active development (v0.8+)
- Pros: Pioneered role-based software company collaboration (CEO, Architect, PM, Engineer); Rigorous SOP approach reduces hallucination by producing real technical artifacts; MGX allows zero-code full-stack product building and deployment; Strong research foundation (oral presentation at ICLR 2025).
- Cons: No visual Kanban board to drag, drop, or monitor task card progression; Open-source CLI output is primarily terminal logs and generated markdown files; Complex multi-role runs consume substantial token budgets; MGX cloud product is decoupled from local developer workflows.
- Sources: https://github.com/geekan/MetaGPT, https://mgx.dev, https://atoms.dev
- Confidence: high

### ChatDev 2.0 (DevAll)
- URL: https://chatdev.modelbest.cn
- Kind: agent-native
- What: A zero-code multi-agent orchestration platform that evolved from a virtual software company simulator into a visual canvas and launchpad for customizable agent workflows. It does NOT provide a Kanban board, utilizing a node-based visual workflow canvas, launch execution logs, and an animated pixel-art visualizer.
- Agent integration: Python SDK (`chatdev`), YAML workflow definitions, FastAPI backend, Vite/Vue 3 frontend; integrates with OpenAI, Anthropic, and local LLMs via LiteLLM.
- Mobile: none (web console running on localhost:5173 or hosted SaaS; no native iOS/Android apps)
- Pricing: open-source free (Apache-2.0); hosted SaaS at chatdev.modelbest.cn offers free trial credits
- Self-host: yes via Docker Compose or local FastAPI backend + Vue 3 frontend
- License: Apache-2.0
- Activity: 34,527 GitHub stars, major 2.0 release in January 2026
- Pros: Visual drag-and-drop workflow canvas for designing multi-agent communication networks; ChatDev 2.0 introduces zero-code YAML orchestration beyond software dev; Replay visualizer with charming pixel-art agent interaction animations; Human-in-the-loop review checkpoints.
- Cons: Lacks a Kanban task board for backlog/in-progress tracking; High compute/token consumption during multi-turn seminar dialogues; Vue 3 web console requires separate backend and frontend launch; No mobile interface.
- Sources: https://github.com/OpenBMB/ChatDev, https://chatdev.modelbest.cn/
- Confidence: high

### OpenAI Agents SDK Tracing
- URL: https://platform.openai.com/docs/guides/agents
- Kind: agent-native
- What: OpenAI's official multi-agent framework providing agent handoffs, guardrails, tools, and integrated cloud tracing in the OpenAI Platform dashboard. It does NOT offer a Kanban board, displaying runs as a hierarchical trace tree of agent calls, tool invocations, inputs, and outputs.
- Agent integration: Python SDK (`openai-agents`), JS/TS SDK (`openai-agents-js`), official MCP tool integration (`autogen_ext` / MCP server adapters), OpenAI Responses/Completions API; provider-agnostic for 100+ LLMs.
- Mobile: none (developer tracing UI is desktop web on platform.openai.com; consumer ChatGPT mobile apps do not expose developer tracing)
- Pricing: open-source free (Python SDK under MIT License); Tracing UI is free on platform.openai.com for OpenAI API users; API model token usage billed at standard rates
- Self-host: yes for the SDK (runs locally); tracing dashboard is hosted on OpenAI's cloud platform
- License: MIT License (Python SDK); proprietary cloud dashboard
- Activity: 30,063 GitHub stars, highly active official OpenAI repository launched in 2025
- Pros: Official, lightweight, and cleanly designed multi-agent primitives from OpenAI; First-class support for agent-to-agent handoffs and guardrails; Integrated visual trace tree in OpenAI dashboard with zero setup; Broad model support beyond OpenAI via provider-agnostic engine.
- Cons: No Kanban task board or work status tracker; Tracing UI is tied to OpenAI's hosted platform; Does not provide persistent issue or backlog management; No native developer mobile monitoring tool.
- Sources: https://github.com/openai/openai-agents-python, https://openai.github.io/openai-agents-python/
- Confidence: high

### Dify
- URL: https://dify.ai
- Kind: agent-native
- What: An open-source LLM application development platform combining visual DAG workflow orchestration, RAG pipelines, agent assistants, and model management. It does NOT provide a Kanban board view, relying on a node-and-edge visual canvas, chat studio, and tabular log/annotation tables.
- Agent integration: REST API, Webhooks, Plugin ecosystem, Tool integrations; integrates with Claude, GPT-4, DeepSeek, Ollama, and external services.
- Mobile: responsive web (deployed webapps support mobile web/PWA; admin studio is desktop web only; no native mobile app)
- Pricing: open-source free; Cloud Sandbox is free (200 credits), Professional is $59/mo, Team is $159/mo, Enterprise custom (as of Oct 2026)
- Self-host: yes via turnkey Docker Compose (Community edition free)
- License: Apache-2.0
- Activity: 158,596 GitHub stars, extremely active with daily commits and enterprise adoption
- Pros: Comprehensive all-in-one platform for RAG, agents, and complex workflows; Intuitive drag-and-drop visual DAG workflow canvas; Powerful self-hosting via single Docker Compose command; Robust observability with execution logs and user feedback annotation.
- Cons: No Kanban task board for tracking multi-agent deliverables; Canvas can become overwhelming for very complex cyclical logic; Heavy resource footprint when self-hosting full stack (PostgreSQL, Redis, Weaviate/Qdrant, Celery); Admin console is not tailored for mobile screen use.
- Sources: https://github.com/langgenius/dify, https://dify.ai, https://docs.dify.ai
- Confidence: high

---

### KaibanJS
- URL: https://www.kaibanjs.com
- Kind: agent-native
- What: A JavaScript-native multi-agent framework built explicitly around the Kanban methodology, featuring "The Kaiban Board" UI where agents visibly move tasks from To-Do to In-Progress to Done.
- Agent integration: JS/TS SDK (`kaibanjs`), React component library (`@kaibanjs/react`), external coding agent bridge for Claude Code, OpenCode, and Codex (`type: 'ExternalCodingAgent'`), MCP and A2A support in ecosystem (kaiban-distributed).
- Mobile: responsive web (Kaiban Board web interface and playground run cleanly on mobile browsers; no native iOS or Android app)
- Pricing: open-source free (100% free under MIT License); hosted playground is free (as of Oct 2026)
- Self-host: yes via `npx kaibanjs@latest` or embedded directly in Next.js, Vite, or Node.js applications
- License: MIT License
- Activity: 1,480 GitHub stars, active releases (v0.24.2)
- Pros: Directly answers the need for a Kanban board UI for multi-agent systems; Real-time visual feedback as agents pick up and complete cards; Embeddable React component to put agent boards directly inside user apps; First-class JavaScript/TypeScript native design with external coding agent adapters.
- Cons: Smaller ecosystem and community compared to Python frameworks; Documentation is still expanding for advanced enterprise flows; No native mobile app store applications; Complex multi-node deployments require community distributed extensions.
- Sources: https://github.com/kaiban-ai/KaibanJS, https://www.kaibanjs.com, https://docs.kaibanjs.com
- Confidence: high

### Routa
- URL: https://github.com/phodal/routa
- Kind: agent-native
- What: A workspace-first multi-agent coordination platform for software delivery, integrating shared specifications (Specs) with Kanban board orchestration and MCP/ACP/A2A protocols across web and desktop.
- Agent integration: MCP (Model Context Protocol), ACP (Agent Client Protocol), A2A (Agent-to-Agent), CLI (`routa-cli`), REST API; integrates with Claude Code, OpenCode, Codex, and local LLMs.
- Mobile: responsive web (built on Next.js 16; accessible via mobile browsers; no native iOS/Android apps)
- Pricing: open-source free (100% free under MIT License; no paid tiers)
- Self-host: yes via Docker Compose, Rust Axum backend + Next.js frontend, or local desktop build
- License: MIT License
- Activity: 1,828 GitHub stars, active development and releases (v0.19.0)
- Pros: True Kanban board orchestration designed specifically for multi-agent software delivery; Native support for modern agent protocols (MCP, ACP, A2A); Direct linking between architecture specs and execution task cards; High-performance Rust backend paired with a modern Next.js frontend.
- Cons: Rapidly evolving API and user interface; Limited third-party tutorials outside official documentation; Setup requires multi-service orchestration (Rust + Node); No dedicated mobile app.
- Sources: https://github.com/phodal/routa
- Confidence: high

### Agent Teams AI
- URL: https://agentteams.live
- Kind: agent-native
- What: A multi-agent team operating platform where the human acts as the executive boss overseeing an autonomous Kanban board while agents self-assign, collaborate, communicate, and review tasks.
- Agent integration: CLI, REST API, webhooks; supports 300+ models, 200+ LLM providers, and coding agents including Claude Code, Codex, Cursor, OpenCode, Grok, Kimi.
- Mobile: responsive web (dashboard accessible via mobile browser; no native iOS or Android app)
- Pricing: open-source free (AGPL-3.0); commercial cloud offering at agentteams.live (as of Oct 2026)
- Self-host: yes via Docker Compose or local Node.js environment
- License: GNU Affero General Public License v3.0 (AGPL-3.0)
- Activity: 2,265 GitHub stars, highly active development and releases (v2.17.10)
- Pros: Full-fledged Kanban board built for hierarchical multi-agent teams; Real-time card transitions across backlog, in-progress, peer review, and done; Broadest LLM provider support (300+ models including free tiers); Human review gates allow high-level executive steering.
- Cons: AGPL-3.0 license limits proprietary embedding; High concurrency can strain local CPU and memory; Interface can become dense with numerous active agent teams; Mobile web view is usable but complex.
- Sources: https://github.com/777genius/agent-teams-ai, https://agentteams.live/, https://yetanotherorchestrator.app/apps/agent-teams/
- Confidence: high

### Agent Kanban
- URL: https://agent-kanban.dev
- Kind: agent-native
- What: An autonomous Kanban board for AI coding agents designed to take humans out of the loop by converting backlog issues into parallel coding sessions with automated verification.
- Agent integration: CLI, GitHub webhooks, REST API; integrates with Claude Code, Codex, Aider, and custom shell agents.
- Mobile: responsive web (responsive web dashboard; no native mobile app)
- Pricing: open-source free under Functional Source License 1.1 (FSL-1.1-ALv2, converting to Apache-2.0 after 2 years)
- Self-host: yes via Docker Compose and Node.js
- License: Functional Source License 1.1 (FSL-1.1-ALv2)
- Activity: 492 GitHub stars, active commits
- Pros: Streamlined issue-to-code automated Kanban pipeline; Visual board cards track live agent execution stages; Isolated worktree management prevents git race conditions; Clean, focused developer experience.
- Cons: FSL license restricts commercial competition for the first two years; Smaller community and fewer contributors; Advanced multi-team collaboration features are still nascent; No native mobile applications.
- Sources: https://github.com/saltbo/agent-kanban, https://agent-kanban.dev, https://yetanotherorchestrator.app/apps/agent-kanban/
- Confidence: high

### Kandev
- URL: https://github.com/kdlbs/kandev
- Kind: agent-native
- What: An AI Kanban and development environment that orchestrates multiple autonomous agents across pipeline and board views, providing an integrated code editor, git diff viewer, and embedded terminal.
- Agent integration: CLI adapters for Claude Code, Codex, Aider, OpenCode; MCP tool support; local terminal PTY integration.
- Mobile: responsive web (mobile remote access explicitly supported with token authentication per `docs/public/mobile-remote-access.md`; no native app store app)
- Pricing: open-source free (100% free under AGPL-3.0; zero telemetry, no paid tier)
- Self-host: yes as a local-first application (runs via Bun/Node or system service, local files and git worktrees)
- License: AGPL-3.0
- Activity: 954 GitHub stars, active development and roadmap
- Pros: Combines Kanban and pipeline views with an integrated workspace (editor, diff, terminal); Dedicated mobile remote access mode specifically documented for monitoring runs from a phone; Git worktree isolation for each running agent task; Completely local-first with zero telemetry and multi-provider support.
- Cons: AGPL-3.0 license; Requires local terminal developer tooling installed; Setup involves local configuration and environment management; No hosted multi-tenant cloud version.
- Sources: https://github.com/kdlbs/kandev, https://raw.githubusercontent.com/kdlbs/kandev/main/README.md
- Confidence: high

### Dev 3.0
- URL: https://github.com/h0x91b/dev-3.0
- Kind: agent-native
- What: A mission control Kanban board for the "One Person Studio" where every card is a live AI coding agent running in its own git worktree and tmux session.
- Agent integration: CLI and PTY orchestration for Claude Code, Codex, Gemini CLI, OpenCode, and arbitrary shell agents.
- Mobile: responsive web (web interface accessible over local network or VPN/Tailscale via mobile browser; no native iOS/Android apps)
- Pricing: open-source free (100% free under Apache-2.0)
- Self-host: yes via Node/Bun, tmux, and git worktrees locally
- License: Apache-2.0
- Activity: 312 GitHub stars, active releases (v1.59.0)
- Pros: Direct 1-to-1 mapping between Kanban cards and live background agent processes; Git worktree isolation prevents files from colliding across parallel agents; Real-time terminal streaming embedded directly inside the card drawer; Permissive Apache-2.0 license.
- Cons: Deeply tied to Unix/macOS environments (requires tmux and git worktrees); Focused strictly on coding tasks rather than general business/analytical workflows; UI is geared toward wide desktop monitors; No native mobile app.
- Sources: https://github.com/h0x91b/dev-3.0, https://raw.githubusercontent.com/h0x91b/dev-3.0/main/README.md
- Confidence: high

### Veritas Kanban
- URL: https://github.com/BradGroux/veritas-kanban
- Kind: agent-native
- What: A local-first task management board featuring native AI agent orchestration, MCP server integration, squad chat webhooks, and governance audit trails.
- Agent integration: Official MCP server (`veritas-kanban-mcp`), CLI, Squad Chat webhooks, REST API; integrates with Claude Code, Cursor, OpenClaw, Codex.
- Mobile: responsive web (responsive web interface optimized for mobile and tablet browsers; no native iOS/Android apps)
- Pricing: open-source free (100% free under MIT License)
- Self-host: yes as local-first via Node.js/Next.js or Docker with local SQLite/JSON storage
- License: MIT License
- Activity: 841 GitHub stars, highly active development (v6.2.1)
- Pros: Official MCP server allows AI agents to directly read, create, and transition board tasks; Local-first architecture ensures 100% data privacy and offline capability; Built-in squad chat and governance logs for agent accountability; Permissive MIT license.
- Cons: Configuring advanced multi-agent automations requires setting up MCP or webhook bridges; Primarily a project management board with agent capabilities rather than a code sandbox; Community is smaller than traditional PM tools; No native mobile apps.
- Sources: https://github.com/BradGroux/veritas-kanban, https://bradgroux.github.io/veritas-kanban/demo/
- Confidence: high

### OpenKanban
- URL: https://github.com/TechDufus/openkanban
- Kind: agent-native
- What: A terminal UI (TUI) Kanban board written in Go for orchestrating, observing, and managing AI coding agents in parallel from the command line.
- Agent integration: CLI and PTY runner; integrates with Claude Code, Codex, Aider, and custom shell agents.
- Mobile: none (terminal application; accessible remotely via SSH using mobile terminal clients like Termius)
- Pricing: open-source free (100% free under MIT License)
- Self-host: yes as a single standalone Go binary (`go install` or precompiled GitHub releases)
- License: MIT License
- Activity: 151 GitHub stars, active releases
- Pros: Single static Go binary with zero external dependencies and instant startup; High-speed keyboard-driven navigation for terminal purists; Clear visual columns (Backlog, Running, Review, Done) directly inside the console; Perfect for remote SSH workflows on headless servers.
- Cons: No graphical web interface or browser access; Lacks rich artifact rendering (no web previews or markdown formatting); Process management is limited to basic CLI lifecycles; Niche appeal for TUI users.
- Sources: https://github.com/TechDufus/openkanban, https://yetanotherorchestrator.app/apps/openkanban/
- Confidence: high

### AIF Handoff
- URL: https://github.com/lee-to/aif-handoff
- Kind: agent-native
- What: An autonomous Kanban board where AI agents plan, implement, and review tasks through automated stages (Backlog → Planning → Plan Ready → Implementing → Review → Done) with automatic rework loops.
- Agent integration: `@aif/runtime` adapters for Claude Code and Codex; CLI and webhooks.
- Mobile: responsive web (responsive web dashboard; no native mobile app)
- Pricing: open-source free (100% free under MIT License)
- Self-host: yes via Node.js and Docker locally
- License: MIT License
- Activity: 308 GitHub stars, active commits
- Pros: Purpose-built stages for autonomous agent handoffs and verification; Automated rework loop sends failed reviews back to implementation; Clear fallback to manual human review when automated agents do not converge; Permissive MIT license.
- Cons: Heavily tied to the AI Factory (`ai-factory`) methodology and runtime; Minimal configuration documentation for non-standard workflows; Small community; No mobile apps.
- Sources: https://github.com/lee-to/aif-handoff, https://raw.githubusercontent.com/lee-to/aif-handoff/main/README.md
- Confidence: high

### Chorus
- URL: https://doc.chorus-ai.dev
- Kind: agent-native
- What: An agent harness for human-AI collaboration that structures coding agent proposals, human verification, and software shipping across an AI-DLC (AI-Driven Development Lifecycle) board.
- Agent integration: CLI, REST API, adapters for Claude Code, Codex, Cursor; git branch and pull request integration.
- Mobile: responsive web (responsive web dashboard; no native iOS or Android app)
- Pricing: open-source free (100% free under AGPL-3.0)
- Self-host: yes via Docker Compose or local monorepo setup
- License: AGPL-3.0
- Activity: 1,199 GitHub stars, active releases (v0.22.2)
- Pros: Implements rigorous AI-DLC methodology for agent proposals and human sign-offs; Clean visual dashboard tracking proposal lifecycle and verification state; Multi-agent collaboration with explicit review gates; Multi-language documentation (EN, ZH, KO, JA).
- Cons: AGPL-3.0 license; Can introduce process friction for rapid single-file prototyping; Monorepo setup requires running full server stack; No mobile apps.
- Sources: https://github.com/Chorus-AIDLC/Chorus, https://doc.chorus-ai.dev
- Confidence: high

### It's a Plan (ItsAPlan)
- URL: https://itsaplan.dev
- Kind: general-kanban-with-agent-integration
- What: An open-source, self-hosted project management alternative to Linear and Plane where AI agents act as first-class assignees and teammates on Kanban boards via MCP, REST API, and webhooks.
- Agent integration: Official MCP server, REST API, webhooks; supports Claude Code, Cursor, Codex, and autonomous agent frameworks.
- Mobile: responsive web (Next.js/React responsive web interface; no native iOS/Android apps)
- Pricing: open-source free (100% free under AGPL-3.0); no per-seat pricing
- Self-host: yes via Docker Compose, Coolify, or Railway
- License: AGPL-3.0
- Activity: 983 GitHub stars, active commits and releases
- Pros: Full issue tracker (boards, cycles, custom fields) where AI agents are real assignees; Official MCP server enables agents to manipulate tasks directly; Self-hosted with zero per-seat fees; Modern Linear-style aesthetic.
- Cons: AGPL-3.0 copyleft license; Project is in active development with potential breaking changes; Requires running Docker/Postgres database stack; No native mobile app store apps.
- Sources: https://github.com/croffasia/itsaplan, https://itsaplan.dev
- Confidence: high

## EXPAND
- LEAD: Yet Another Orchestrator (yetanotherorchestrator.app) catalog — WHY: Catalogs 48 emerging agent orchestrator applications (e.g., Supacode, Conductor, OpenKanban, Agent Teams, Herdr, 1code, Aizen) comparing worktree paradigms, terminal streaming, and board interfaces — ANGLE: webfetch https://yetanotherorchestrator.app/
- LEAD: KaibanJS React Component library (@kaibanjs/react) — WHY: Demonstrates the only embeddable Kanban board component natively designed for AI agents in client-side React/Next.js applications — ANGLE: webfetch https://docs.kaibanjs.com/how-to/React-Integration
- LEAD: Microsoft Agent Framework (MAF) Visual Tooling Roadmap — WHY: Successor to AutoGen 0.4 entering general availability; enterprise replacement for AutoGen Studio — ANGLE: web_search "Microsoft Agent Framework" "Studio" OR "UI" OR "board"
