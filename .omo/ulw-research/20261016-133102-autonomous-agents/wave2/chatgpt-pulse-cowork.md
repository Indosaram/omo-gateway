<!-- task=st_01a14521 -->
st_01a14521 [completed] model mahoquot/claude-sonnet-5-5
Ask: research the nightly-briefing, approval, injection-classifier and phone-to-desktop pages and report with sources. Wanted: findings, patterns for a Rust gateway, and claims with sources.
For you: 5 of 8 targets read directly. The Pulse article (12293630) could not be read, and two other targets (ChatGPT agent help and the openai.com Pulse launch post) were not retrieved.
Now: report is complete.
Next: none.

## Findings

**Routes tried.** Direct curl returned a Cloudflare challenge for `help.openai.com`. `r.jina.ai` worked for Anthropic pages (retry after 429 rate limits). Wayback worked for OpenAI help pages. HN Algolia returned only metadata. I did not try Reddit or `gh`.

**ChatGPT Pulse help article 12293630: NOT READ.**
- Direct curl was blocked by a Cloudflare challenge. [https://help.openai.com/en/articles/12293630] (curl)
- jina returned a 404 "That page doesn't exist". [same URL] (r.jina.ai)
- Wayback has no capture of the bare ID. The slug `-chatgpt-pulse` has a 2026-09-07 capture that is itself a 404 page. [https://help.openai.com/en/articles/12293630-chatgpt-pulse] (Wayback)
- The Pulse launch post `openai.com/index/introducing-chatgpt-pulse/` was Cloudflare-blocked on Wayback. jina was blocked until about 13:49 UTC.
- HN thread 45375477 ("ChatGPT Pulse", 730 comments) exists and is SECONDARY. I did not read its comments. (HN Algolia)
- No Pulse design claims are made below.

**ChatGPT scheduled tasks (help article 10291617). PRIMARY, Wayback snapshot of 2026-08-30. [https://help.openai.com/en/articles/10291617-tasks-in-chatgpt]**
- The article is now titled "Scheduled tasks in ChatGPT". It covers one-time, recurring, monitoring and event-triggered tasks.
- Active-task caps by plan: 3 Free/Go, 5 Plus, 10 Business/Edu, 15 Pro/Enterprise.
- Free plans get a flexible window (morning, afternoon, night). Exact times and hourly runs need a paid plan.
- Event-triggered tasks run in "Work" with an explicit Trigger / Condition / Prompt triple. Sources are Gmail, Slack and GitHub PRs.
- Event-triggered runs are limited to 30 per hour and 720 per day, and events may be grouped.
- Actions that send messages or change external data may require approval, and the task pauses until you review it.
- A task pauses when it is inactive, needs more action, or its chat is deleted.
- Monitoring tasks use prior-run info and stop on a defined end condition.
- Shared task links carry only title, instructions, schedule and time zone. They exclude history, memory, files and credentials. Recipients use their own connections.
- Tasks created inside a project cannot access that project's files.
- Notifications go by push or email. Task runs are covered by the Compliance API.

**Cowork cloud/surfaces (support article 15520349). PRIMARY. [https://support.claude.com/en/articles/15520349] (jina)**
- Since 2026-10-06, new Cowork tasks on Pro/Max run in Anthropic's cloud. The "Only on your computer" option was removed.
- Local folders stay on the machine. Claude reaches them only through the open desktop app, and fetches a copy of just the needed file.
- Sessions follow the account. You can resume on another surface, and the phone gets a notification when a task finishes or needs input.
- Local file access, browser use and computer use need the desktop app open.

**Dispatch (13947068). PRIMARY. [https://support.claude.com/en/articles/13947068] (jina)**
- Dispatch is one persistent thread, reachable from phone and desktop.
- Claude picks the session type: development goes to Claude Code, knowledge work to Cowork.
- Dispatch tasks run on the desktop, so the computer must be awake with the app open. Cloud sessions keep running with the computer off.
- Claude reports the outcome rather than every step, and sends push notifications on completion or when it needs a go-ahead.
- Memory is viewable, editable and deletable.
- The page warns that a phone-to-desktop agent chain means a manipulated instruction can cascade into hard-to-undo actions.

**Cowork safety (13364135). PRIMARY. [https://support.claude.com/en/articles/13364135] (jina)**
- A prompt injection needs two conditions at once: the agent reads content outside the trust boundary, and it can act in a way that compromises the user. Breaking either one makes the attack harder.
- Cloud isolation limits where code runs, not what Claude reads or does.
- There are three approval modes: "Manually approve", "Automatically approve" and "Skip all approvals".
- In "Automatically approve", Claude reviews each action for safety before it runs. In "Skip all approvals", nothing checks its actions.
- Permanent deletion always asks, in any mode.
- Scheduled tasks run unattended, including with the computer off, so the page asks for extra care. Past runs are reviewed on the "Scheduled" page.
- Computer use has no sandbox between Claude and the screen.

**Scheduled tasks in Cowork (13854387, an extra page I fetched). PRIMARY. [https://support.claude.com/en/articles/13854387] (jina)**
- A daily briefing is a named use case: Slack, email and calendar from the last 24 hours.
- Manual setup fields are name, prompt, approval mode, frequency (hourly, daily, weekly, weekdays, manual), optional model and optional folder.
- Each run is its own session.
- A task that needs local files or apps runs locally only.
- Claude proposes name, schedule and instructions, and the user clicks "Schedule" to confirm.
- Pause, resume, run-on-demand and delete are supported.

**Claude in Chrome safety (12902428). PRIMARY. [https://support.claude.com/en/articles/12902428] (jina)**
- There are two classifiers. One screens incoming content. The other checks every action before it runs. Flagged actions are blocked or paused for user approval.
- Anthropic reports under 0.08% attack success with Opus 4.8 against its internal combined-technique test.
- Some high-risk site categories are blocked outright. Financial sites require permission.
- Admins can set an allowlist. Regulated-data orgs get it off by default.

**Anthropic prompt-injection research (2025-11-24). PRIMARY. [https://www.anthropic.com/research/prompt-injection-defenses] (jina)**
- It names three defenses: RL training against simulated injections, classifiers scanning all untrusted content entering context, and human red teaming.
- Classifiers cover hidden text, manipulated images and deceptive UI. A detection triggers an "intervention" that changes model behavior.
- It reports about 1% attack success for Opus 4.5 against a Best-of-N attacker with 100 attempts. The post says this is still meaningful risk.

## Design patterns worth copying (Rust gateway)

1. **Briefing job as a first-class record.** This mirrors Cowork's fields and ChatGPT's Trigger/Condition/Prompt.
   ```rust
   struct Job { id, name, prompt, schedule: Schedule, approval: ApprovalMode,
     model: Option<..>, workdir: Option<..>, state: JobState, runs_cap: .. }
   enum Schedule { Once(DateTime), Cron{expr, tz}, Window(Morning|Afternoon|Night), Event{trigger, cond}, Manual }
   ```
   Store the time zone with the schedule. ChatGPT's shared links carry the original zone.
2. **Run as its own session.** Each firing creates a `Run{job_id, session_id, status, started, outcome}`. Browse history by job.
3. **Job state machine.** `Active -> Running -> (AwaitingApproval | Paused | Failed | Done)`. A job auto-pauses on inactivity, an unresolved approval, or a deleted source chat, and only resumes when the user acts.
4. **Approval modes.** `Manual | AutoReviewed | Skip`, set per job. `AutoReviewed` runs an action classifier before each tool call. Hard-code a rule that irreversible actions (delete, send, pay) always prompt, in every mode. Default scheduled jobs to `Manual` or `AutoReviewed`, never `Skip`.
5. **Lethal-trifecta gate (the two-condition rule above).** Tag each tool as `reads_untrusted` or `writes_external`. When a run has touched an untrusted source, deny or escalate external-write tools. This gives you a mechanical rule, not a classifier guess.
6. **Two-stage classifier hooks.** One hook on content entering context, one on each action before it executes. Both return `Allow | Block | Escalate`. Escalate puts the run in `AwaitingApproval`. The action hook must be an independent code path from the model.
7. **Phone-to-desktop handoff.** Run one persistent thread keyed to the user. Dispatch work to a desktop worker and track `worker_online`. If the worker is offline, queue the job or fall back to a cloud-safe subset, as Cowork does when a task needs local files. Phone approvals arrive as push messages carrying `(run_id, action_digest)`. Bind approval to that digest so a changed action invalidates it.
8. **Rate caps.** Cap active jobs per user and event-driven runs per hour and day, with event coalescing. This stops a flood of events from mass-running agents.
9. **Notification contract.** Notify only on completion or a needed decision, and report the outcome, not the step stream.
10. **Share/export safety.** Exported jobs include prompt, schedule and zone only. Exclude memory, history, files and credentials.

## CLAIMS

| CLAIM | RISK | SOURCES | PRIMARY |
|---|---|---|---|
| Pulse (12293630) design details are unknown from this research | High if assumed; the page is 404 on help.openai.com | jina 404, Wayback 404 | Not read |
| ChatGPT approval-gated actions pause the task until the user reviews | Low | help 10291617 snapshot | Yes |
| Cowork "Automatically approve" reviews each action; "Skip all approvals" has no check; delete always asks | Low | 13364135 | Yes |
| Claude in Chrome uses an input classifier plus a per-action classifier; flagged actions block or pause | Low | 12902428 | Yes |
| Injection success is below 0.08% (Opus 4.8) | Medium: vendor-reported, internal test | 12902428 | Yes, self-reported |
| About 1% attack success at 100 adaptive attempts (Opus 4.5) | Medium: vendor-reported | prompt-injection-defenses | Yes, self-reported |
| Dispatch tasks run on the desktop and need the app open | Low | 13947068 | Yes |
| Cowork Pro/Max tasks run in the cloud from 2026-10-06 | Medium: state at fetch time, pages change fast ("Updated yesterday") | 15520349 | Yes |
| The Wayback snapshot of 10291617 reflects ChatGPT's current tasks page | Medium: it is dated 2026-08-30 | Wayback | Yes, but dated |

## EXPAND

- Read Pulse 12293630 and the openai.com launch post. Retry jina after 13:49 UTC for `openai.com/index/introducing-chatgpt-pulse/`, and try Wayback captures under other slugs.
- Fetch the ChatGPT agent help pages. I did not find or fetch them.
- Read the HN 45375477 comments for community-reported Pulse behavior (SECONDARY).
- Read the Cowork pages 13345190 (get started), 13455879 (Team and Enterprise), and 14477985 (OpenTelemetry). Also read the Chrome permissions guide 12902446 for the permission-prompt model.