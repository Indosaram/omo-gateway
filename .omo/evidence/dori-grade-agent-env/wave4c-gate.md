# Wave 4c release gate

Status: released. release_allowed: true. All requested gates passed under the explicit backend baseline exception.

## Release identity

- Binary: `/Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway`
- Binary SHA-256: `d162a234313e3329d40724eb083458b1ae18fb7795ef37e5c16184eaafbe278f`
- Manifest: `wave4c-source-post-2.sha256` (224 entries)
- Manifest SHA-256: `8f08a9546538d4ff7af0b8d4641eecbcd8153dbb0d25b8b5bb10807298c67028`
- Pre/post source comparison: exit 0.

## Gate results

All Cargo commands used empty RUSTC_WRAPPER, warm shared target-w4, -j4, --offline, and 1800-second monitors. cargo check --all-targets --all-features exited 0 with no diagnostics. The single combined --no-fail-fast focused batch exited 0; no target reruns. qa_gaps lib/bin also exited 0.

| Target | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| test_cron_health | 12 | 0 | 2 |
| test_inbox_cli_digest | 9 | 0 | 0 |
| test_inbox_proposals | 10 | 0 | 0 |
| test_inbox_store | 11 | 0 | 0 |
| test_inbox_triage | 9 | 0 | 0 |
| test_lane_close | 27 | 0 | 0 |
| test_lane_dispatch | 10 | 0 | 0 |
| test_lane_done | 11 | 0 | 0 |
| test_lane_events | 25 | 0 | 1 |
| test_lane_guard_alerts | 5 | 0 | 0 |
| test_lane_kibitzer | 9 | 0 | 0 |
| test_lane_reopen | 5 | 0 | 0 |
| test_lane_runner | 12 | 0 | 1 |
| test_lane_status_transport | 1 | 0 | 0 |
| test_lane_thread_transport | 2 | 0 | 0 |
| test_profile_import | 7 | 0 | 0 |
| test_session_registry | 7 | 0 | 0 |
| test_session_registry_task27 | 2 | 0 | 0 |
| test_usage_done_gate | 9 | 0 | 0 |
| Focused total | 183 | 0 | 4 |
| qa_gaps lib | 3 | 0 | 0 |
| qa_gaps bin | 4 | 0 | 0 |

Nested lane_runner subprocess-helper summaries are excluded from the top-level totals. Filtered qa_gaps tests: lib 485, bin 61.

## Backend baseline comparison

One test_omo_backend run: exit 101, 45 passed, 3 failed, 1 ignored. Zero new failure names; observed names exactly match the requested three-name baseline:

- `test_omo_backend_aborts_turn_on_repeated_approval_denials`
- `test_omo_backend_deadline_interrupts_looping_turn`
- `test_omo_backend_e2e_thread_lifecycle_and_streaming`

All three are present in the canonical task-2-failing.txt in dori-grade-agent-env. That older file lists seven additional backend failures which passed here; this gate uses the three names explicitly prescribed by the user. It does not claim equivalence of historical panic causes or an all-green backend target. Current panic details are retained in wave4c-backend.log.

## Strict gate and source freeze

Attempt 1: fmt 1, clippy 0; build and cmp not run. The sole formatting diagnostic was guard.rs create_thread argument wrapping. Another workspace owner applied exactly that formatting before this owner's patch; the patch failed without edits. The two pre-manifests differ only at guard.rs.

The one allowed rerun (attempt 2): fmt 0, clippy 0, build 0, source cmp 0. Owned TMPDIR /tmp/wave4c-strict.IgT6E9 was retained through build completion and remains available. Attempt 1 TMPDIR /tmp/wave4c-strict.LciuiU is also preserved. No behavioral or compile/lint edits were needed by this gate owner.

The strict script passed bash -n; Bash LSP was unavailable. Logs: wave4c-check.log, wave4c-focused.log, wave4c-qa-gaps.log, wave4c-backend.log, wave4c-fmt-{1,2}.log, wave4c-clippy-{1,2}.log, wave4c-build-2.log. Exits and binary hash are retained alongside the logs.

## Scope limits

This is the requested local source/binary release gate, not deployment or live QA. No services, live Discord, models, env files, or git commands were used. Tests used owned loopback fixtures and isolated databases. Retained QA2 live failures remain intact. Installed app-server real token usage emission is still unavailable as documented by qa2-fix-usage-source-settled.md; passing synthetic usage tests do not establish real emitter support.
