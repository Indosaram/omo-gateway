# Task12 next-worker67 binary release workflow - held

No runtime is authorized by this document. Backend baseline tests are pending; root-fix candidate is not built. Task23 scoped confirmation is separate from the final binary gate. This workflow consumes the next worker67 full binary SHA256, not a prefix or the old884a identity.

1. Obtain worker67's actual build exit0, full binary path/SHA256, settled source manifest path/SHA256 and released gate artifact path/SHA256. Read those artifacts and compare the binary and all manifest inputs. Do not reuse old global green or an old prepared root.
2. Run existing lane-scratch-qa.mjs prepare with that manifest/native judge/full binary SHA only after runtime release. It creates a dedicated fresh root. Preserve prepared.json unchanged; its SHA is authoritative.
3. Use the established absolute owned scope-check.mjs protocol on retained sole ws://127.0.0.1:19742. Reconstruct script with apply_patch from exact new prepared values and hash before/after. One actual scope observation must prove cwd/explicit child env/full binary --help exit0. No new daemon, no lane yet.
4. Via apply_patch create release.json schema omon-lane-qa-release-v1. Copy root/dbPath/databaseUrl/binary/binarySha256/manifestSha256 exactly from fresh prepared. Set preparedSha256 to its actual hash. Set gate.status=released and gate.evidence={path,sha256} from actual worker67 gate. Copy scope-probe.json.scope verbatim into backendScope and attach evidence={path,sha256} from actual scope artifact. No placeholder verified fields or substituted inherited equality.
5. Privately inject supported OMON_OMO_APPSERVER_AUTH_TOKEN and existing native judge credentials; no token in argv/logs/artifacts. Run lane-question-qa.mjs validate <prepared> <release> with Bun --no-env-file and observe exit0. No environment file mutation. The primary token is trimmed and passed explicitly; source omo_config.rs reads that name before OMON_OMO_AUTH_TOKEN.
6. On parent runtime release, register a background bounded monitor for lane-question-qa.mjs execute <same prepared> <same release>. One use only. Report helper numeric exit and owned gateway numeric0/PIDgone. Archive is unique outside root; root and old task8/9 evidence retained. No replay on observer presentation failure: query retained DB and receipt first.

## Review corrections

The observer subscribes root+existing WAL before initial query and trigger; it attaches WAL on creation and reattaches on rename. Unknown directory filename also triggers authoritative query. Final deadline queries before rejection. Backend failed status, extra turns and duplicate events fail closed. Terminal count and user-prompt count are part of the predicate, not checked only after CLI admission. Question created_at must be finite and inside [start,start+120seconds].

Initial blocked completed turn proves one actual captured question and one completed outcome. Send is explicit --text, matching cli.rs Send parser. Second turn uses exact role=user owner text plus actual agent response and two completed outcomes on the same thread. A second deliberate question pauses the resumed turn; no claim/file completion is needed.

After forceclose and owned gateway exit, read-only authoritative DB assertions require closed row, turns2, same thread, last outcome completed; exact original question event IDs/body/timestamps and resumed user/agent/outcome transcript survive unchanged. task-12-question.json is emitted only after those assertions and gateway exit0, never on admission or precleanup observation alone. Failure captures question-postcleanup-db.json when DB was verified. Shared daemon is never stopped.

Current events25/postHTTP2 remain scoped local regressions. three_errors_fail_with_dead_event still needs parent current repaired-source runner regression; historical161103Z runner exit0 is not a205-batch pass. The fullplan live question criterion remains unrun until this workflow produces actual receipts.
