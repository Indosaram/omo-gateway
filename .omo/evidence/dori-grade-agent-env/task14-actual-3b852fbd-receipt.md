# Released task14 actual QA: concrete completion-content failure

Executed helper07b95002796b0375355611f3739bc7f3edc83fd4ac05c622af6ba1886b4874e2 exactly once using task14-release-180128Z.json. Authenticated initialize passed without model turn. Fresh root `/private/tmp/omon-task14-live-4mnmFJ`, owned gateway PID51809, exact binary3b852fbd4d5f93911a3200fe07bb8bb7d037f317c24a7bc46fcc60a5b314b933 and manifest0d3fc34abc10daa6d15f4de0745762ff26e6e3afcfb39137e347b6b70095b9ee matched before/after. No source edits or additional turn.

Initial descriptor/PRAGMA prove fresh owned file database, WAL mode and empty tables. Pretrigger directory/WAL/log observers and initial queries were armed. Marker-blocked snapshot has event1 undelivered and no messages. Removal allowed admission in the same gateway with lead_turn receipt and System user row. At18:36:21.834212Z local delivery log records the exact requested reply; at18:36:21.835869Z a second final local delivery is logged for the SAME stream_id.

Authoritative timeout recovery and post-cleanup read-only inspection establish:
- assistant TASK14_ACTUAL_REPLY count1 remains durable;
- additional assistant content `✅ Done.` count1;
- one delivered obligation for the shared stream, but its content is `✅ Done.`, not TASK14_ACTUAL_REPLY;
- session cron_agent_output=`✅ Done.`, remote binding retained, resume_pending0;
- qa_actor_settlement empty because its trigger intentionally requires exact requested output;
- no interrupted-turn log.

Thus shutdown assistant removal did not recur, but this is NOT the requested happy-path pass. The backend emitted/persisted two final contents and overwrote the same obligation/output with Done. The helper timed out correctly because exact matching obligation and actor-settlement proof were absent; this was not merely a lost observer event. Backend terminal/actor state settled with the wrong final content. No semantic broadening of the success predicate, artifact rewriting or turn replay was performed.

Numeric cleanup: helper exit1; gateway graceful exit0; PID51809 absent; owned socket absent check0; sole existing daemon57142 remains on19742. Root retains task14.log, task14.json, descriptor.json and task14.db. Earlier MwogZ7/Y5YPVQ artifacts preserved. No token logged, model override, new daemon, ownerDB, Discord, build/tests or git action. Original baseline limitation remains: eight backend baseline failures and two uncovered tests are not green.

Monitor mon_XSVMHHFJGKTA7KGQ/session bash_3 completed exit1. Concrete failure is duplicate final delivery/content overwrite, not401 and not quota. Further implementation or runtime requires parent direction; source freeze retained.
