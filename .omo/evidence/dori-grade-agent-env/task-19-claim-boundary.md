# B19-1 agent claim boundary correction

SOURCE-SETTLED after explicit parent freeze release. Only the claim event body
in lanes/cli.rs now uses lanes::scrub::scrub followed by a 2,000-character bound
when claim=true. Owner objections remain exact. Internal claim_evidence and
history retain original text; existing transaction and generation fences remain.

Added agent_claim_event_scrubs_and_bounds_without_changing_internal_evidence
to tests/test_lane_events.rs. It uses the fake GitHub token with 36 suffix
characters and Korean text, asserts the stored done_claimed event is redacted
and 2,000 characters, and asserts internal evidence/history remain exact.
Existing owner-guidance regression is unchanged.

No Cargo, test, runtime QA or gate ran. Regression is UNRUN; no actual sink
verification is claimed. Literal HOME correction remains with helper owner63.
No unrelated todo12, helper, schema, config, service or commit changes occurred.
