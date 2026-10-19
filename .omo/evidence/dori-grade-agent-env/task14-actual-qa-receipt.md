# Task14 actual headless QA: concrete failure

Frozen binary884a16827325e3512ef7a4ce7441f4dd0aa1fa2e4b8954cf95feefaf42bc461e and manifest6408b90a62b071b82666e6dec339dd0508f2dc3e3c50eedc19e95cba314d53a7 verified before and after execution. No product edits or source gates repeated.

Owned auditable root: `/private/tmp/omon-task14-live-MwogZ7`. Exact argv and explicit environment in descriptor.json and task14.json; raw gateway evidence in task14.log; migrated file database task14.db retained. Actual PID12731. Existing sole listener19742 PID57142 retained throughout and verified after cleanup. Inherited DEFAULT_MODEL=gemini-3.8-flash-high was retained without model override; no token was inserted. No ownerDB, Discord, shared-stop, daemon launch or env-file edit.

Result: FAIL, helper exit1; gateway graceful exit0; PID absent true; owned lane socket absent (shell check0). Initial helper failed before any root/process/turn on an ArrayBuffer hash decoder mistake; corrected helper then launched exactly one gateway and one event. No turn was rerun after observer timeout.

PRAGMA database_list proves owned file path; journal_mode=wal; initial events/messages/sessions/obligations empty. Directory/WAL-state and stream-log subscription preceded the insert and initial authority queries. Drain marker was present at startup. Blocked snapshot records event1 with delivered_at/delivery_reason null and no messages. Marker removed at2026-10-11T17:45:54.303Z; same gateway admitted event1 at17:45:59.300139Z, persisted lead_turn receipt and `[system] [LANE_EVENT]` user transcript.

Concrete failure at17:46:29.812273Z: `agent runner failed ... failed to connect to omo app-server at ws://127.0.0.1:19742: HTTP error: 401 Unauthorized`. Backend's existing pre-submission retry also logged at17:46:14.307824Z. No successful turn submission/completion, assistant row, delivered obligation or local-output acceptance log exists. The 180-second bounded observer ended with an authoritative final query, not an assumed observer failure. Recovery and post-cleanup read-only DB queries both show only one System user row, no obligations, no remote binding and resume_pending0. The admission receipt is not completion proof.

Cleanup logged `gateway lane service stopped` at17:48:54.309031Z; task14.json records gatewayExit0, pidAbsent=true, exitCode1. Root and evidence remain for inspection. Stop condition is concrete failure; authentication configuration requires parent resolution before another authorized runtime attempt. No settings were changed to bypass the401.
