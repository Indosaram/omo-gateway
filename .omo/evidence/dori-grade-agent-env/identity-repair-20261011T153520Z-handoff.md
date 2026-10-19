# Source/binary/prepare identity ready

Once-only cargo build -j2 --bin omo-gateway on existing target-w4 exited 0, 2.09s, PID70315. Source pre/post cmp 0. Build restored binary SHA-256 dbde346e2f7be84327a919e9242cba2a999a2d23ad417819e960fde1230d8a7e, the original verified build identity, from previously observed 92799c12c4f4e6bbb318d06ebfb545a5c70390f4a869e66e26e785d1ef8d742f. Cargo output only Finished dev profile; it reused existing artifacts. This establishes this invocation and settled source binding, but does not identify the prior replacement builder. No rebuild compilation line or provenance of the earlier replacement is invented.

All 191 manifest entries measured; only confirmed todo21 docs/AGENTS_DELEGATE.md help-port changed. All compiled product/source, Cargo, test and migration inputs unchanged from successful race-port gate. Regenerated manifest SHA-256 59beefea246726d79453775c2a62a6e149d21519302b695864bf2d70faa302db. Prior gate receipt remains retained unchanged. No Clippy/tests/full suite rerun.

Actual prepare argv:
```
/Users/indo/.bun/bin/bun --no-env-file /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/lane-scratch-qa.mjs prepare /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/identity-repair-20261011T153520Z-manifest.sha256 /Users/indo/.cargo/bin/typesafe-judge dbde346e2f7be84327a919e9242cba2a999a2d23ad417819e960fde1230d8a7e
```
Cwd /private/tmp, explicit credential-free PATH=/usr/bin:/bin:/usr/sbin:/sbin HOME=/private/tmp TMPDIR=/private/tmp. PID73789 exit 0, awaited, stderr empty. Actual stdout:
```json
{"result":"PREPARED_RUNTIME_UNRUN","preparedPath":"/private/tmp/omon-lane-scratch-qa-GUgT9Z/prepared.json","root":"/private/tmp/omon-lane-scratch-qa-GUgT9Z","dbPath":"/private/tmp/omon-lane-scratch-qa-GUgT9Z/qa.db","binarySha256":"dbde346e2f7be84327a919e9242cba2a999a2d23ad417819e960fde1230d8a7e","manifestSha256":"59beefea246726d79453775c2a62a6e149d21519302b695864bf2d70faa302db"}
```

Prepared JSON SHA-256 b970264d8ddb528c5fe01712971e800eabb7afc7175da39a59ea85cce2ac032c. Native judge /Users/indo/.cargo/bin/typesafe-judge SHA-256 daacff0341e0f0f1f86e4fa9335af723603a0515732ec72ed8aad2c4c6aa0fe6. Empty dotenv SHA-256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855. Root mode0700; prepared.json mode0600. Independent hash/absence check exited0: binary and copied manifest match, qa.db and all -wal/-shm/-journal sidecars absent. Build/prepare PIDs no longer exist.

Full env identity read in prepared.json: isolated HOME/XDG/TMPDIR/PATH under exact root, cwd repo root/qa-repo, DATABASE_URL sqlite:///private/tmp/omon-lane-scratch-qa-GUgT9Z/qa.db, endpoint ws://127.0.0.1:19742, lanes true, Discord/kibitzer/dashboard false, credentials not copied. Runtime unreleased. zarZ0e and pibpnv preserved. No scope probe; worker72 is sole probe owner. No backend/service/DB/network/Discord or product edits.
