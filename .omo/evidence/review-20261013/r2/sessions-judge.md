/END block listing only findings with a concrete trigger and cited lines I read, or NO FINDINGS.
</analysis>

REPORT-BEGIN
# sessions-judge review

### [P2] `sessions adopt --lead` is not validated as a session key
- Location: /Users/indo/code/project/omon-gateway/src/sessions/external.rs:94
- Trigger: `omo-gateway sessions adopt omarchy:abc --goal G --done "file x" --lead foo`. The only check is `goal.trim().is_empty() || lead.trim().is_empty()`. `lead` is then written verbatim to `lanes.lead_session_key` (line 114 INSERT, `.bind(&lane.lead_session_key)`).
- Impact: A lead that does not parse as a storage key is persisted. `lanes::cli` rejects the same input (the `from_storage_key` validation at lines ~312 and ~359). Lane events for this lane then target a session that cannot be resolved, so the lead is never woken or notified. The failure is silent. Round 1 raised this and it is still open.
- Fix: Call `SessionKey::from_storage_key(lead)` in `adopt` and return `invalid(...)` on `None`/`Err`, as `lanes::cli` does.

### [P2] The judge circuit-breaker owner alert is still undeliverable in production
- Location: /Users/indo/code/project/omon-gateway/src/judge/mod.rs:892-897
- Trigger: `handle_breaker_alert` sends only when `self.alerter` is `Some`. Every production `JevCli::new` call I read passes `None` for the alerter: /Users/indo/code/project/omon-gateway/src/main.rs:860-865, 884-889, 1263-1268 and 1292-1297, plus `lanes::cli::run` at src/lanes/cli.rs:120.
- Impact: When the breaker opens after repeated jev failures, the judge silently falls back to the fallback model. The owner is never told, because only the `FakeOwnerAlerter` test double implements `OwnerAlerter`. The comment-documented alert path is dead code outside tests.
- Fix: Wire a real `OwnerAlerter` into the gateway-side constructors, or log the breaker-open event at `warn` so it is observable without an alerter. At minimum, log when `alerter` is `None`.
