# Freeze snapshot - 20261020T100754Z

Captured on the lead's explicit snapshot signal, after the backend worker confirmed exactly two
`assert` reflows patched and no further source edits. This is the tree that
`review-20261020-drift` gates and the tree a single fix-all commit would carry.

Scope: `src web/src migrations tests scripts` plus the root manifests (`Cargo.toml`, `Cargo.lock`,
`web/bunfig.toml`, `web/test-preload.ts`, `web/package.json`, `web/tsconfig*.json`,
`web/vite.config.ts`, `web/vitest.config.ts`, `web/index.html`).

Reports, logs, `.omo/` evidence and `web/dist` artifacts are excluded, so evidence writes cannot
masquerade as source drift.

## Artifacts

| File | Content | sha256 |
| --- | --- | --- |
| `tracked-20261020T100754Z.diff` | **full tracked diff** vs HEAD `5fcfc08` (644978 B) | `4da7066144d01c2b95b5482d0d04de3db745f1c881eefd70c7f3109d3fc0cf32` |
| `tracked-20261020T100754Z.diff.ignore-ws` | same diff with `-w` (640181 B) | `82f3b7e9b10eabbd60f3fbdea7431cf4caa344a1732b34777c030afc989073e9` |
| `changed-paths-20261020T100754Z.txt` | `git diff --name-status` for the set | `3a0166d1beefcb1b2a7323add52ffe064eff101c779b94bae670a16f57d663cc` |
| `tracked-hashes-20261020T100754Z.txt` | **per-file sha256 for all 611 tracked source files** | `10b6ff178d889707e054428175b9c5ed10e0b307cb400332e679959c849602ba` |
| `untracked-hashes-20261020T100754Z.txt` | per-file sha256 for the 11 untracked source files | `d02f2f9614bc8f59645a7913d1a8c349e665a177da2cfd764d4218ae23b30ffc` |

## Aggregate tripwire

| Aggregate | Value |
| --- | --- |
| Source-set `git diff` sha256 | `4da7066144d01c2b95b5482d0d04de3db745f1c881eefd70c7f3109d3fc0cf32` |
| Source-set `git diff -w` sha256 | `82f3b7e9b10eabbd60f3fbdea7431cf4caa344a1732b34777c030afc989073e9` |
| Tracked files in set | 611 |
| Tracked modified | 86 |
| Untracked source files | 11 |

The aggregate alone was the defect in the previous freeze (`6b086892…`): it proved *that* drift
happened but could not reconstruct content. These artifacts fix that - the diff file can be diffed
against any later state, and the per-file manifest identifies a single changed file without relying
on mtime.

## Drift check for the commit

```sh
# 1. aggregate tripwire - must match
git diff -- src web/src migrations tests scripts Cargo.toml Cargo.lock \
    web/bunfig.toml web/test-preload.ts web/package.json web/tsconfig.json \
    web/vite.config.ts web/vitest.config.ts web/index.html web/tsconfig.app.json \
    web/tsconfig.node.json | shasum -a 256

# 2. exact content diff - must be empty
diff <(git diff -- <set>) tracked-20261020T100754Z.diff

# 3. per-file manifest - every line must match
shasum -a 256 -c tracked-hashes-20261020T100754Z.txt
```

## Known state at capture

- Two `assert!` reflows in `src/agent/omo_backend.rs` (~3586, ~3599) are **patched** in this
  snapshot: `assert!(backend(true).ensure_workspace_preset(&enabled_path).await);` and the
  `!backend(false)` sibling are collapsed onto one line.
- The earlier `fmt exit=1` (2 hunks) remains a recorded historical failure. It is not rewritten:
  `review-20261020-final` passed a tree that no longer existed, and this snapshot is the tree that
  `review-20261020-drift` must certify.
- Not deployed. `DEPLOYED rev=5fcfc08` is unchanged.
