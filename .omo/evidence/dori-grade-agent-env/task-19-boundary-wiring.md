# Todo19 boundary wiring source receipt

SOURCE-SETTLED only. No Cargo, runtime QA or actual Discord sink verification ran.
The provider decoding error was not treated as a source verdict or a reason to
restart/revert work; the same session preserved and completed its patches.

Changed only src/lanes/posts.rs, activity.rs, runner.rs and
tests/test_lane_events.rs. Posts now use lanes::scrub::scrub before the existing
2,000-character bound. The Judge excerpt uses the same helper and bound.
Activity event bodies and runner failure event bodies are scrubbed and bounded
before binding to lane_events. These paths contain agent/gateway-generated text.
Existing generation predicates, transactions and delivery receipt states remain.

Owner CLI objection input/guidance is unchanged. Generic storage event enqueue
was not modified because it can carry owner input. Internal raw transcript and
history storage were not globally redacted. Scrub helper source, Cargo and shared
coordinator files were not changed.

New source regression:
agent_post_and_event_scrub_token_but_owner_answer_stays_exact
uses the actual activity -> SQLite typed post -> posts::drain -> fake PostSink
component path. A fake GitHub token with 36 suffix characters must be absent
from the sink payload, stored post body and question event; Korean text must
remain. The subsequent owner objection queues the original exact text including
that token, proving the owner-input path was not indiscriminately redacted.

This is a fake sink test source, not a real Discord delivery receipt. It has NOT
been executed. Worker63's reported 11 isolated helper-harness passes are not a
crate, integrated boundary or actual sink pass and are not credited as such here.
Current-tree compiler, regression tests, global gates and actual sink QA remain
pending the parent batch. Coordinator27 owns todo19 test registration.

No live DB, external message, service mutation, configuration/model change,
migration or commit occurred. No process or scratch resource remains running.
