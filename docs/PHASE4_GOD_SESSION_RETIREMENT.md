# Phase 4: God-Session Retirement & Per-Agent/Per-Thread Model Migration

## Overview

Historically, OmOMeow operated with a persistent, monolithic god-session architecture implemented in `god-session.ts`. This document establishes the migration and retirement procedure transitioning from the singleton god-session to the `omon-gateway` per-agent and per-thread session multiplexer architecture.

---

## 1. Architectural Comparison

| Attribute | Legacy (`god-session.ts`) | Target (`omon-gateway`) |
| :--- | :--- | :--- |
| **Session Model** | Single resident god-session running indefinitely | Ephemeral per-agent / per-thread multiplexed sessions |
| **State Storage** | In-memory globals + ad-hoc local files | SQLite ACID database (`sessions`, `messages`, `work_items`) |
| **Process Model** | Monolithic Bun/Node daemon | Rust daemon (`omon-gateway`) + child worker harnesses |
| **Thread Lifecycle** | Manual channel polling & thread creation | Event-driven Discord Gateway (`ThreadCreate`, `ThreadUpdate`, `ThreadDelete`) |
| **Heartbeat & SLA** | Unreliable in-process setInterval loops | Robust background heartbeat runner checking DB timestamps (15m overdue alerts) |
| **Approval Flow** | Blocking terminal prompts / brittle hooks | Non-blocking Discord buttons (`Approve`, `Reject`, nonterminal previews) |

---

## 2. Pre-Retirement Checklist

Before terminating the legacy god-session:
1. **Cron Parity Verified**: All 14 cron jobs (10 watch + 4 notification) migrated and verified in `cron_jobs` table (`scripts/cutover_phase1_cron.sh`).
2. **Shadow Bot Validated**: End-to-end messaging, button handling, and thread lifecycles verified via `scripts/verify_phase2_shadow.sh`.
3. **Inbound Traffic Cutover**: Primary Discord bot token successfully shifted to `omon-gateway` (`scripts/cutover_phase3_inbound.sh`).
4. **State Drained**: Ensure all in-flight turns in `god-session.ts` have reached terminal state.

---

## 3. Step-by-Step Retirement Procedure

### Step 3.1: Graceful Drain & Process Termination
1. Check for running `god-session.ts` or `omomeow` processes:
   ```bash
   pgrep -fl "god-session|omomeow"
   ```
2. Unload the persistent launchd service:
   ```bash
   launchctl bootout gui/$(id -u)/com.omomeow
   ```
3. Archive legacy launchd definitions:
   ```bash
   mv ~/Library/LaunchAgents/com.omomeow.plist ~/Library/LaunchAgents/com.omomeow.plist.retired
   ```
4. If process is still executing active turns, send `SIGTERM` and allow up to 30 seconds for completion, then `SIGKILL` if necessary:
   ```bash
   kill -15 <PID>
   ```

### Step 3.2: Verification of Per-Agent / Per-Thread Multi-Session Isolation
1. Confirm `OMON_PER_AGENT_WORKSPACE=true` is enabled in `omon-gateway` `.env`.
2. Confirm Discord auto-thread behavior:
   ```env
   DISCORD_AUTO_THREAD=true
   DISCORD_THREAD_SESSIONS_PER_USER=true
   ```
3. Verify session tracking in database:
   ```bash
   sqlite3 data/gateway.db "SELECT id, thread_id, session_key, updated_at FROM sessions ORDER BY updated_at DESC LIMIT 5;"
   ```
4. Verify thread cleanup on archive/delete events:
   - When a Discord thread is archived, `sessions.metadata.thread_archived` is marked `true` and the multiplexer process is stopped.
   - When a Discord thread is deleted, `omo_thread_id` is cleaned up while historical message logs are safely preserved.

### Step 3.3: Archiving `god-session.ts`
1. Move legacy source files to archive location or mark as deprecated:
   ```bash
   # In legacy omomeow repository:
   mv src/god-session.ts src/god-session.ts.deprecated
   ```
2. Record retirement timestamp and sign-off in deployment logs.

---

## 4. Rollback Contingency

If critical issues arise with the per-agent/per-thread multiplexer during Phase 4:
1. Execute `scripts/rollback_phase3_inbound.sh` to release the Discord Gateway token from `omon-gateway`.
2. Restore legacy launchd plist:
   ```bash
   mv ~/Library/LaunchAgents/com.omomeow.plist.retired ~/Library/LaunchAgents/com.omomeow.plist
   launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.omomeow.plist
   ```
3. Restart `god-session.ts` daemon.
