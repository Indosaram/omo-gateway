# Corrected fixture gate and exact binary preparation

Only owned test fixture changed; product inputs otherwise identical to grouped160343Z. Scoped fmt0, LSP no diagnostics, one strict all-target/all-feature Clippy0 (4.92s), focused test_lane_runner all-features/no-fail-fast0 (12 passed/0 failed/1 ignored, 1.01s). Disabled no-token service readiness and shutdown/socket/DB cleanup, enabled/default required-token rejection all passed in three isolated exact-selector children. Source pre/post cmp0, grouped160343Z full failure set hash-preserved0, test scratch cleanup0. No unchanged lib/CLI/events/reopen/registry rerun.

Sequential once-only cargo build -j2 --bin omo-gateway on target-w4 exit0, 50.39s. Source pre/post cmp0. Binary /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/debug/omo-gateway SHA-256 60fce5ae8276944e9930bc2c41aa2ff039ad3daa29c05facd79e796cc621bc32. Manifest fixture-repair-20261011T161103Z-post-manifest.sha256 SHA-256 45200128b3b5cd63fbc956b36e01daaa82f817b63607d176ef6751d89f486718. Source manifests cover191 inputs. Both validation and build PID handles completed without restoration duplicate.

Actual prepare argv:
```
/Users/indo/.bun/bin/bun --no-env-file /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/lane-scratch-qa.mjs prepare /Users/indo/code/project/omon-gateway-wt/dori-work/.omo/evidence/dori-grade-agent-env/fixture-repair-20261011T161103Z-post-manifest.sha256 /Users/indo/.cargo/bin/typesafe-judge 60fce5ae8276944e9930bc2c41aa2ff039ad3daa29c05facd79e796cc621bc32
```
Cwd/private/tmp, credential-free PATH=/usr/bin:/bin:/usr/sbin:/sbin HOME=/private/tmp TMPDIR=/private/tmp. PID96670 exited0, awaited, empty stderr. Stdout:
```json
{"result":"PREPARED_RUNTIME_UNRUN","preparedPath":"/private/tmp/omon-lane-scratch-qa-TzIHbY/prepared.json","root":"/private/tmp/omon-lane-scratch-qa-TzIHbY","dbPath":"/private/tmp/omon-lane-scratch-qa-TzIHbY/qa.db","binarySha256":"60fce5ae8276944e9930bc2c41aa2ff039ad3daa29c05facd79e796cc621bc32","manifestSha256":"45200128b3b5cd63fbc956b36e01daaa82f817b63607d176ef6751d89f486718"}
```
Prepared JSON SHA-256 4ae06ac9950ffd644d10ca0b573228e9a45447b4c619c5aedddef4a1193bad78. Native judge pinned daacff0341e0f0f1f86e4fa9335af723603a0515732ec72ed8aad2c4c6aa0fe6. Root0700/prepared0600, full isolated env read in prepared.json. Repo root/qa-repo; DATABASE_URL sqlite:///private/tmp/omon-lane-scratch-qa-TzIHbY/qa.db. Independent filesystem-only hash/absence check0: no DB or -wal/-shm/-journal. Runtime unreleased for worker72; no probe, shared-service stop, model/env edits, git, Discord or old-root reuse.

Raw runner log SHA-256 ef47a14f129b4b79125e1234494e1ba7527181e1c3a054a54a5e4fd63bd7d7c6; Clippy da1ab05512aeee54b11fb4cd128e3fc2ef3c37b688f403e1c5b1f354c8be8e34; build 4f2cda98555227f0d185293a6f5b50f7f52087d48b3971eeafdb157da6af6182. All exact commands/exits, gate and build manifests, retained prior failure checksums, and cleanup artifacts under this prefix.
