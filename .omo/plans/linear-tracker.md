# Plan: Linear-identical issue tracker inside the omon dashboard

Status: in progress (2026-10-16). Owner: lead session. Commits: FORBIDDEN until the user says so. Deploy: FORBIDDEN.

## Goal
A full Linear-equivalent issue tracker (teams, workflow states, issues, sub-issues, labels, priorities,
estimates, due dates, projects + project updates, cycles, relations, links, comments + reactions,
subscriptions, notifications/inbox, saved views, favorites, search, command palette, keyboard shortcuts,
list/board views, multi-select bulk edit, light/dark) rendered with Linear's exact visual language,
served by the existing axum dashboard at `#/tracker`. Agent work lanes (`lanes` table) are mirrored as
issues in the system team `AGT`, and any issue can be delegated to an agent (creates a lane).

## Hard rules for every worker
1. Do NOT run cargo, bun, npm, tsc, vite, or any build/test command. Write code only. The lead runs ONE
   integration gate after all workers finish (user instruction).
2. Write ONLY inside your owned paths (table below). Never edit another worker's files or the lead's
   foundation files; if a foundation API is missing, write a TODO comment in your own file and report it.
3. UI copy is English, exactly like Linear. Code comments in English.
4. No emojis as icons (emoji only inside comment reactions). Icons: lucide-react or hand-written SVG.
5. No coloured accent stripes (`border-l-2 border-primary` etc.). State = background alpha wash.
6. Animations: transform/opacity only, 120-180ms, `cubic-bezier(.2,0,0,1)`; respect prefers-reduced-motion.

## Ownership
| Worker | Owns (write) |
|---|---|
| lead | `.omo/plans/linear-tracker.md`, `web/src/tracker/types.ts`, `web/src/tracker/api.ts`, `web/src/tracker/store.tsx`, `web/src/tracker/router.ts`, `web/src/tracker/linear.css`, `web/src/tracker/hotkeys.ts`, `web/src/tracker/format.ts`, `web/src/App.tsx` (2-line mount), `web/src/main.tsx` |
| BE | `migrations/0044_tracker.sql`, `src/tracker/**`, `src/lib.rs` (add `pub mod tracker;`), `src/dashboard.rs` (ONLY: nest the tracker router + a test module block at the end), `tests/test_tracker_api.rs`, `Cargo.toml` ([[test]] entry only if needed) |
| FE-A | `web/src/tracker/ui/**` (primitives + icons + pickers) |
| FE-B | `web/src/tracker/TrackerApp.tsx`, `web/src/tracker/shell/**`, `web/src/tracker/pages/SettingsPage.tsx`, `web/src/tracker/pages/SearchPage.tsx` |
| FE-C | `web/src/tracker/issues/**` (list, board, filters, display options, bulk bar), `web/src/tracker/pages/TeamIssuesPage.tsx`, `MyIssuesPage.tsx`, `ViewPage.tsx`, `ViewsPage.tsx` |
| FE-D | `web/src/tracker/detail/**` (issue page, peek, comments, activity, agent panel), `web/src/tracker/create/**` (create modal) |
| FE-E | `web/src/tracker/pages/ProjectsPage.tsx`, `ProjectPage.tsx`, `CyclesPage.tsx`, `CyclePage.tsx`, `InboxPage.tsx`, `web/src/tracker/projects/**`, `web/src/tracker/inbox/**` |

## Module contract (frontend) — import paths are binding
- Foundation (lead, exists): `../types` (all entity types), `../api` (`trackerApi`), `../store` (`useTracker()`, `TrackerProvider`), `../router` (`useRoute()`, `navigate(path)`, `Route` union), `../hotkeys` (`useHotkey(combo, handler, opts)`, `useSequence(['g','i'], handler)`), `../format` (date/relative/identifier helpers), `../linear.css` (tokens; import once in TrackerApp).
- FE-A exports from `web/src/tracker/ui/index.ts` (exact names):
  - `Button({variant:'primary'|'secondary'|'ghost'|'danger', size:'sm'|'md', icon?, kbd?, ...button props})`
  - `IconButton({label, icon, size?:'sm'|'md', active?, ...})` (tooltip = label)
  - `Tooltip({content, kbd?, side?, children})`
  - `Kbd({keys: string[]})` renders ⌘ ⇧ ⌥ glyphs
  - `Menu` system: `Popover({open,onOpenChange,anchor:HTMLElement|null|DOMRect, align?, side?, children, width?})`, `MenuList({items: MenuItem[], onSelect, searchable?, placeholder?, multi?, selectedIds?, emptyText?})` where `MenuItem = {id, label, icon?, kbd?, hint?, color?, group?, disabled?, children?: MenuItem[]}`; full keyboard nav (arrows, enter, esc, type-to-filter, number shortcuts 0-9 for first ten items like Linear)
  - `ContextMenu({items, onSelect, children})` (right-click)
  - `Dialog({open,onOpenChange,title?, width?, children, footer?})` centered modal, overlay `rgba(0,0,0,.5)` (light) / `.65` (dark)
  - `Toast` API: `toast(message, {action?:{label,onClick}})` + `<Toaster/>`
  - `Avatar({user|null, size?:16|20|24|32})` initials on `user.avatar_color`; agent users get a square-ish bot glyph
  - `Checkbox({checked, indeterminate?, onChange})`
  - Icons: `StatusIcon({type, color, progress?, size?})` (backlog dashed circle, unstarted ring, started ring+pie by progress 0..1, completed filled circle+check, canceled filled circle+x, triage double-arrow), `PriorityIcon({priority:0..4, size?})` (0 dots "no priority", 1 urgent orange square with "!", 2-4 three bars filled 3/2/1), `LabelDot({color})`, `ProjectIcon({icon,color})`, `CycleIcon({progress})`, `EstimateIcon`, `AgentGlyph`
  - Pickers (popover menus bound to the store, each `({issueIds:string[], anchor, open, onOpenChange})`): `StatusPicker`, `PriorityPicker`, `AssigneePicker`, `LabelPicker` (multi), `ProjectPicker`, `CyclePicker`, `EstimatePicker`, `DueDatePicker` (calendar + shortcuts "Tomorrow/End of week/In 1 week…"), `ParentPicker`. Each applies changes via `useTracker().updateIssues(ids, patch)`.
  - `PropertyChip({kind, issue})` compact chip used in rows/board cards/detail sidebar that opens the right picker on click.
- FE-B exports default `TrackerApp` from `web/src/tracker/TrackerApp.tsx` (wraps `TrackerProvider`, renders shell + routes, mounts `<Toaster/>`, `CommandPalette`, global hotkeys, mobile drawer).
- FE-C exports from `web/src/tracker/issues/index.ts`: `IssueCollection({issues, viewKey, defaultDisplay?, emptyState?, showTeamTabs?})` (renders toolbar: filter + display + layout toggle, list or board), plus pages `TeamIssuesPage({teamKey, tab:'all'|'active'|'backlog'|'triage'})`, `MyIssuesPage({tab:'assigned'|'created'|'subscribed'|'activity'})`, `ViewPage({viewId})`, `ViewsPage()`.
- FE-D exports from `web/src/tracker/detail/index.ts`: `IssuePage({idOrIdentifier})`, `IssuePeek()` (reads `useTracker().peekIssueId`), and from `web/src/tracker/create/index.ts`: `CreateIssueModal()` (reads `useTracker().createDraft`).
- FE-E exports pages `ProjectsPage()`, `ProjectPage({projectId, tab})`, `CyclesPage({teamKey})`, `CyclePage({cycleId})`, `InboxPage({notificationId?})`.
- FE-B also exports pages `SettingsPage({section})`, `SearchPage({q})`.

## Routes (hash, all under `#/tracker`) — `router.ts`
`/inbox[/:notificationId]`, `/my-issues[/assigned|created|subscribed|activity]`, `/team/:key/(all|active|backlog|triage)`,
`/team/:key/cycles`, `/cycle/:id`, `/projects`, `/project/:id[/overview|issues|updates]`, `/views`, `/view/:id`,
`/issue/:identifier`, `/search?q=`, `/settings[/account|preferences|teams|team/:key|labels|members|workflow/:key]`.
Default `/` → `/team/OMO/active`... first team.

## Data model (BE) — `migrations/0044_tracker.sql`
IDs are TEXT uuid v4 unless noted. Timestamps TEXT RFC3339 UTC (`strftime('%Y-%m-%dT%H:%M:%fZ','now')`).
- `tracker_users(id PK, name, display_name, email NULL, avatar_color, kind CHECK in ('human','agent'), model NULL, is_me INTEGER 0/1, created_at)`
- `tracker_teams(id PK, key UNIQUE CHECK(key GLOB '[A-Z]*' AND length(key) BETWEEN 1 AND 5), name, icon, color, issue_seq INTEGER NOT NULL DEFAULT 0, system INTEGER 0/1, cycles_enabled INTEGER, cycle_weeks INTEGER DEFAULT 2, triage_enabled INTEGER DEFAULT 0, created_at, sort_order REAL)`
- `tracker_states(id PK, team_id FK, name, type CHECK in ('triage','backlog','unstarted','started','completed','canceled'), color, position REAL, lane_status NULL UNIQUE-per-team, description NULL)`
- `tracker_labels(id PK, team_id NULL FK, name, color, parent_id NULL, description NULL, created_at)` (team_id NULL = workspace label)
- `tracker_projects(id PK, name, description TEXT DEFAULT '', summary DEFAULT '', icon DEFAULT 'box', color, status CHECK in ('backlog','planned','started','paused','completed','canceled'), priority INTEGER DEFAULT 0, lead_id NULL, start_date NULL, target_date NULL, sort_order REAL, created_at, updated_at, archived_at NULL)`
- `tracker_project_teams(project_id, team_id, PK both)`, `tracker_project_members(project_id, user_id, PK both)`
- `tracker_project_updates(id PK, project_id, user_id, health CHECK in ('on_track','at_risk','off_track'), body, created_at, updated_at)`
- `tracker_milestones(id PK, project_id, name, target_date NULL, sort_order REAL)`
- `tracker_cycles(id PK, team_id, number INTEGER, name NULL, description NULL, starts_at, ends_at, completed_at NULL, UNIQUE(team_id, number))`
- `tracker_issues(id PK, team_id, number INTEGER, identifier UNIQUE, title, description TEXT DEFAULT '', state_id, priority INTEGER 0..4 DEFAULT 0, estimate INTEGER NULL, assignee_id NULL, creator_id, project_id NULL, milestone_id NULL, cycle_id NULL, parent_id NULL, due_date NULL (YYYY-MM-DD), sort_order REAL, lane_key NULL UNIQUE, created_at, updated_at, started_at NULL, completed_at NULL, canceled_at NULL, archived_at NULL, snoozed_until NULL)`
- `tracker_issue_labels(issue_id, label_id, PK)`; `tracker_subscribers(issue_id, user_id, PK)`
- `tracker_comments(id PK, issue_id, user_id, body, parent_id NULL, created_at, updated_at, edited INTEGER DEFAULT 0, resolved_at NULL)`; `tracker_reactions(comment_id, user_id, emoji, PK all three)`
- `tracker_relations(id PK, issue_id, related_issue_id, type CHECK in ('blocks','related','duplicate'), created_at, UNIQUE(issue_id, related_issue_id, type))`
- `tracker_links(id PK, issue_id, url, title, created_at)`
- `tracker_activity(id INTEGER PK AUTOINCREMENT, issue_id, actor_id NULL, kind, from_value NULL, to_value NULL, created_at)` kinds: created,title,description,state,priority,assignee,estimate,label_added,label_removed,project,cycle,parent,due_date,archived,unarchived,relation_added,relation_removed,delegated,lane_status
- `tracker_views(id PK, name, description NULL, icon DEFAULT 'layers', color NULL, owner_id, team_id NULL, filters TEXT JSON, display TEXT JSON, shared INTEGER DEFAULT 1, created_at, updated_at)`
- `tracker_favorites(id PK, user_id, kind CHECK in ('issue','project','view','cycle','team','label'), target_id, sort_order REAL, UNIQUE(user_id, kind, target_id))`
- `tracker_notifications(id PK, user_id, issue_id NULL, project_id NULL, kind, actor_id NULL, body, created_at, read_at NULL, snoozed_until NULL, archived_at NULL)` kinds: assigned, mentioned, comment, status_change, subscribed_update, lane_blocked, lane_done_claimed, lane_verified, lane_failed, project_update
- `tracker_meta(key PK, value)` (stores `seeded=1`)
Seed (in Rust on first bootstrap, idempotent via tracker_meta): human user `me` (name `Me`, is_me=1, color `#5e6ad2`); team `OMO` "Omon" with states Backlog(backlog #bec2c8) Todo(unstarted #e2e2e2) In Progress(started #f2c94c) In Review(started #4cb782) Done(completed #5e6ad2) Canceled(canceled #95a2b3) Duplicate(canceled #95a2b3); system team `AGT` "Agents" (system=1) with lane-mapped states (table below); workspace labels Bug #eb5757, Feature #bb87fc, Improvement #4ea7fc.

## Lane mapping (system team AGT)
| lane status | AGT state name | type | color |
|---|---|---|---|
| queued | Queued | unstarted | #e2e2e2 |
| working | Working | started | #f2c94c |
| blocked | Blocked | started | #eb5757 |
| done_claimed | In Review | started | #4cb782 |
| not_done | Changes Requested | started | #fc7840 |
| verified_done | Verified | completed | #5e6ad2 |
| closed | Done | completed | #5e6ad2 |
| failed | Failed | canceled | #95a2b3 |

Sync (`tracker::lanes::sync(pool)`, called at the top of `GET /bootstrap`, `GET /issues`, `GET /issues/{id}`, `GET /notifications`): for each lane row, upsert one issue by `lane_key`. New lane not linked → create issue in AGT (title = lane.title, description = brief file contents capped 20 KB, creator = agent user for lane.model, assignee = same agent user, state = mapped AGT state). Existing linked issue: if it is in AGT, set state to the mapped state; if it lives in another team (delegated), set state to that team's first state whose type equals the mapped type (completed→first completed, etc.), and store the raw lane status in the response field `lane.status`. On every observed lane status change write `tracker_activity(kind='lane_status', from,to)` and a notification for `me` for blocked/done_claimed/verified_done/failed. Agent users: one per distinct `lanes.model` (`kind='agent'`, name = short model name, model = full).

Human state change on a lane-linked issue (PATCH state_id, or board drag) translates to lane commands via `lanes::cli::execute` (judge = `inbox::triage::production_judge(pool.clone())`):
- to type completed: lane `verified_done` → `Close{force:false}`; otherwise 409 `{"error":"lane must be verified before closing; use force close","code":"lane_not_verified"}` unless body has `force_note` → `Close{force:true, note:"forced: <force_note>"}`.
- to type canceled: `Close{force:true, note:"forced: canceled from tracker"}` (only from working/blocked/verified_done; else 409).
- to started/unstarted/backlog while lane closed/failed → `Reopen{text: None}`.
- any other → 409 `{"error":"IllegalTransition ...","code":"illegal_lane_transition"}`.
Explicit lane actions: `POST /issues/{id}/lane-action {action:'send'|'object'|'close'|'reopen', text?, force?}` mapping 1:1 to `LaneCommand::{Send,Object,Close,Reopen}`; errors → 409 with message.
Delegate: `POST /issues/{id}/delegate {cwd, done, model?:'auto'|'small'|'large', thread?: string}` → key = lowercase identifier + "-" + 4 hex (≤48, `[a-z0-9-]`), brief = `# {identifier}: {title}\n\n{description}` written to `${OMON_TRACKER_BRIEF_DIR:-.omo/tracker-briefs}/<key>.md` (relative to the gateway process cwd, created if missing); then `LaneCommand::Start(StartArgs{key,title,brief,done,thread,cwd:Some(cwd),lead:None,model})`; on exit code 0|4 set `issues.lane_key`, write activity `delegated`, return issue detail; else 400 with the lane error JSON.

## REST contract (BE) — base `/api/tracker`, JSON, errors `{"error": msg, "code"?: str}` with proper status
| Method & path | Body / query | Response |
|---|---|---|
| GET /bootstrap | – | `{me, users, teams, states, labels, projects, cycles, milestones, views, favorites, unread_count}` |
| GET /issues | `?team_id&include_archived=0|1` | `{items: Issue[]}` (all, client filters) |
| POST /issues | `IssueCreate` | `Issue` (201) |
| POST /issues/batch | `{ids:[], patch: IssuePatch}` | `{items: Issue[]}` |
| GET /issues/:idOrIdentifier | – | `IssueDetail` |
| PATCH /issues/:id | `IssuePatch` (+ optional `force_note`) | `Issue` |
| DELETE /issues/:id | – | 204 (409 if lane-linked) |
| POST /issues/:id/comments | `{body, parent_id?}` | `Comment` |
| PATCH /comments/:id | `{body?, resolved?}` | `Comment` |
| DELETE /comments/:id | – | 204 |
| POST /comments/:id/reactions | `{emoji}` (toggle) | `Comment` |
| POST /issues/:id/relations | `{type, related_issue_id}` | `Relation` |
| DELETE /relations/:id | – | 204 |
| POST /issues/:id/links | `{url, title?}` | `Link` |
| DELETE /links/:id | – | 204 |
| POST /issues/:id/subscribe | `{subscribed: bool}` | `{subscribed}` |
| POST /issues/:id/delegate | see above | `IssueDetail` |
| POST /issues/:id/lane-action | see above | `IssueDetail` |
| GET/POST /projects, GET/PATCH/DELETE /projects/:id | `ProjectInput` | `Project` / `ProjectDetail {project, updates, milestones, member_ids, team_ids}` |
| POST /projects/:id/updates, PATCH/DELETE /project-updates/:id | `{health, body}` | `ProjectUpdate` |
| POST /projects/:id/milestones, PATCH/DELETE /milestones/:id | `{name, target_date?}` | `Milestone` |
| POST /cycles, PATCH/DELETE /cycles/:id | `{team_id, starts_at, ends_at, name?}` | `Cycle` |
| POST /labels, PATCH/DELETE /labels/:id | `{name,color,team_id?,parent_id?}` | `Label` |
| POST /teams, PATCH /teams/:id | `{key,name,icon?,color?,cycles_enabled?,triage_enabled?}` (POST seeds default 7 states) | `Team` |
| POST /states, PATCH/DELETE /states/:id | `{team_id,name,type,color,position?}` (409 on system team) | `State` |
| POST /users, PATCH /users/:id | `{name, display_name?, email?, avatar_color?}` | `User` |
| POST /views, PATCH/DELETE /views/:id | `{name, filters, display, team_id?, icon?, color?}` | `View` |
| POST /favorites | `{kind, target_id}` (toggle) | `{favorited, favorite?}` |
| GET /notifications | – | `{items: Notification[]}` |
| PATCH /notifications/:id | `{read?, snoozed_until?, archived?}` | `Notification` |
| POST /notifications/read-all | – | `{updated}` |
| GET /search | `?q=` | `{issues: Issue[], projects: Project[]}` (title/identifier/description LIKE, top 50) |
Every mutation of an issue writes `tracker_activity` rows with actor = me and bumps `updated_at`; assigning someone else creates a notification for the assignee only if they are human; comments notify subscribers (except author). Creating an issue auto-subscribes creator and assignee. `number` = `teams.issue_seq + 1` in the same transaction; identifier = `KEY-number`.
Issue JSON = all columns + `label_ids: string[]`, `sub_issue_count`, `sub_issue_completed`, `comment_count`, `lane: {key,status,model,turns_run,last_activity_at,thread_channel_id} | null`.
IssueDetail = `{issue, children: Issue[], parent: Issue|null, relations: (Relation & {issue: Issue})[], comments: Comment[] (with reactions [{emoji,user_ids}]), activity: Activity[], links, subscriber_ids, lane_detail: null | {lane: full lane row, history, transcript (last 200), events (last 100)}}`.

## Design contract (Linear app, exact tokens) — `web/src/tracker/linear.css`
Font: Inter Variable (`@fontsource-variable/inter`), `font-feature-settings: "cv01","ss03"`, base 13px, weights 400/510/590, mono = `"Berkeley Mono", ui-monospace, SFMono-Regular, Menlo`.
App shell: full viewport. Sidebar 244px (resizable 220-330, collapsible with `[`), bg `--sidebar`; content = inset panel with 8px margin top/right/bottom, radius 8px, 1px `--border`, bg `--panel`. Header 44px with breadcrumb (team icon + name › view) left, actions right; view tabs row 40px ("All issues / Active / Backlog" as 26px pill buttons, active = `--bg-active`). List rows 36px (44px comfortable) with: hover reveal checkbox at far left, priority icon, identifier (mono-ish tertiary, 64px fixed), status icon, title (510 weight primary), then right-aligned: label pills, project chip, estimate, due date, created/updated date (tertiary 12px), assignee avatar 18px. Group header 34px, bg `--bg-group`, status icon + name + count + `+` button on hover. Board: columns 340px, gap 8px, column header 36px, cards radius 6px, bg `--card`, 1px `--border`, padding 10px 12px, hover border `--border-strong`. Peek panel: right overlay 560px. Issue page: two columns — content (max 720px) + right properties sidebar 260px.
Tokens (dark default `.lin[data-theme=dark]`): `--sidebar #090a0b; --panel #0f1011; --card #141516; --bg-hover rgba(255,255,255,.04); --bg-active rgba(255,255,255,.07); --bg-selected rgba(94,106,210,.16); --bg-group rgba(255,255,255,.02); --border rgba(255,255,255,.06); --border-strong rgba(255,255,255,.1); --text-1 #f7f8f8; --text-2 #d0d6e0; --text-3 #8a8f98; --text-4 #62666d; --accent #5e6ad2; --accent-hi #7170ff; --accent-hover #828fff; --danger #eb5757; --warn #f2c94c; --ok #4cb782; --urgent #fc7840; --popover #1c1d1f; --overlay rgba(0,0,0,.65); --shadow-pop 0 0 0 1px rgba(255,255,255,.08),0 8px 24px rgba(0,0,0,.5)`.
Light (`.lin[data-theme=light]`): `--sidebar #f4f5f8; --panel #ffffff; --card #ffffff; --bg-hover rgba(0,0,0,.035); --bg-active rgba(0,0,0,.06); --bg-selected rgba(94,106,210,.12); --bg-group rgba(0,0,0,.02); --border rgba(0,0,0,.08); --border-strong rgba(0,0,0,.14); --text-1 #1b1c1f; --text-2 #3c3f44; --text-3 #6b6f76; --text-4 #9a9da3; --popover #ffffff; --overlay rgba(0,0,0,.4); --shadow-pop 0 0 0 1px rgba(0,0,0,.08),0 8px 24px rgba(0,0,0,.12)`.
Radii: 4 (chips), 6 (buttons, cards, menu items), 8 (popovers, panels), 12 (dialogs). Popover/menu: bg `--popover`, padding 4px, item 32px tall, radius 6, hover `--bg-active`, kbd hints right-aligned `--text-4`. Focus ring: `0 0 0 2px var(--accent-hi)` only on `:focus-visible`.
Responsive: <768px sidebar becomes off-canvas drawer (hamburger in header), panel margin 0 / radius 0, list rows hide everything except priority, status, title, assignee; board scrolls horizontally with snap; issue page stacks properties above content; peek becomes full-screen sheet.

## Keyboard (FE-B registers globally unless noted)
`⌘K` palette · `C` create issue · `⌘⇧?`/`?` shortcuts help · `/` search · `G I` inbox · `G M` my issues · `G A` active · `G B` backlog · `G P` projects · `G V` cycles · `G S` settings · `[` toggle sidebar · `⌘B` toggle list/board (FE-C) · `F` filter (FE-C) · `J/K`/arrows move, `X` select, `⇧X`/shift-click range, `⌘A` select all, `Enter`/`O` open, `Space` peek, `Esc` close/clear (FE-C) · on focused/selected/open issue (FE-C rows + FE-D page): `S` status, `P` priority, `A` assignee, `I` assign to me, `L` labels, `⇧P` project, `⇧C` cycle, `⇧E` estimate, `⇧D` due date, `⌘.` copy ID, `⌘⇧,` copy branch name, `⌘⌫` delete, `⌘⇧A` archive, `E` edit title (FE-D), `M` comment (FE-D).

## Verification (lead only, once, after all workers)
1. `cargo build` + `cargo clippy --all-targets -- -D warnings` + `cargo test --test test_tracker_api` + `cargo test --lib tracker` + existing dashboard tests (`cargo test --bin omo-gateway dashboard`) — shared target dir `target`, sccache.
2. `cd web && bun run build` (tsc -b + vite build).
3. Scratch DB (copy of omon_gateway.db → `/tmp/tracker-qa/omon.db`) + standalone dashboard on 127.0.0.1:9333 (`DATABASE_URL=sqlite:///tmp/tracker-qa/omon.db omo-gateway dashboard --port 9333`), curl scenarios S1-S6, browser scenarios B1-B10 via omowright, desktop 1440x900 + mobile 390x844, dark + light.
4. Cleanup receipt: kill dashboard pid, close browser, rm -rf /tmp/tracker-qa.

### QA scenarios
- S1 `curl -s localhost:9333/api/tracker/bootstrap | jq '.teams|map(.key)'` → contains `OMO` and `AGT`; `.states|length >= 15`.
- S2 POST /issues `{team_id:<OMO>, title:"QA issue", priority:2}` → 201, `identifier` = `OMO-1`; PATCH state to In Progress → `started_at` set; activity has `state` row.
- S3 Lane mirror: scratch DB has 4 closed lanes → GET /issues?team_id=<AGT> returns 4 issues in state `Done` with `lane.status=closed`.
- S4 Illegal lane transition: PATCH an AGT issue (lane closed) to `Working` state → Reopen accepted or 409 per mapping; PATCH a verified-less working lane issue to Done without force_note → 409 `lane_not_verified`.
- S5 Comments/relations/links/subscribe/notifications/views/favorites/projects/cycles/labels CRUD each return 2xx and round-trip in GET.
- S6 Search `?q=QA` returns the QA issue.
- B1 `#/tracker` loads Linear shell (screenshot desktop dark). B2 Create issue via `C` + modal (typed title, status, priority, label) → row appears. B3 ⌘B board, drag card Todo→In Progress → persisted after reload. B4 Filter by priority=High via `F` menu → only matching rows; save as view → appears in sidebar. B5 Issue page: change status with `S`, add comment, add sub-issue, add relation. B6 Peek with Space. B7 ⌘K palette → "Go to Projects" works; create project + update. B8 Cycles page + inbox notification visible. B9 multi-select 2 rows with X → bulk set priority. B10 Mobile 390px: drawer, list, issue page; light theme screenshot.
