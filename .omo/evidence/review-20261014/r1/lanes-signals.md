# Lane lanes-signals

### [P1] done_diff rebuilds the wrong "new" side when the lane cwd is a subdirectory of the git repo
- Location: src/lanes/settle.rs:359
- Trigger: Start a lane with `--cwd` set to a subdirectory of a repository, such as a monorepo crate (`/repo/crates/foo`). `cli.rs` canonicalizes the cwd but does not move it up to the repo root, and `capture_lane_start` still records a start commit. The lane edits `crates/foo/src/lib.rs` and adds `crates/foo/NEW.md`, commits and pushes, then claims done.
- Impact:
  - `git diff --cached --name-only -z <rev>` (settle.rs:294) prints paths relative to the repo root, for example `crates/foo/src/lib.rs`. This happens because neither `--relative` nor `diff.relative` is set.
  - The size probe (settle.rs:322) and the content read (settle.rs:359) both resolve those paths against `cwd`, which gives `/repo/crates/foo/crates/foo/src/lib.rs`. That returns NotFound, so `new` becomes `None`.
  - `git show <rev>:<path>` resolves from the repo root, so the old side is found.
  - In the temp repo, every modified file therefore shows up as deleted. Every added file has `old = None` and `new = None`, so it disappears.
  - `input.3 > 0`, so the "diff over budget" skip at settle.rs:228 does not apply. The judge scores an empty diff or one that only deletes, and `fulfills < 0.65 || risk > 1.5` rejects it.
  - The lane is bounced to `not_done` with a `done_diff fulfills=...` objection. The agent cannot fix this, so every new claim fails the same way.
- Evidence: `.args(["diff", "--cached", "--name-only", "-z", revision, "--"])` (settle.rs:294), then `tokio::fs::metadata(cwd.join(&path))` (settle.rs:322) and `tokio::fs::read(cwd.join(&path))` (settle.rs:359). The listed paths are relative to the repo root, but they are joined onto `cwd`.
- Fix: Resolve the top level once with `git -C cwd rev-parse --show-toplevel` and join paths onto that. Alternatively, run the listing with `--relative` (and `git show <rev>:./<path>`) so that all three operations use cwd-relative paths.

All four files were read in full and checked against both earlier review rounds. The previously recorded items are not repeated here: the budget `break`, the kibitzer cap and fence handling, the restore path, scrub gaps, env and output bounds, the splitter quote handling, the stale `now`, and the prunable worktree registration. The cited lines 294, 322 and 359 were re-checked against the file. As instructed, nothing was built or run, so the git path behaviour above comes from reading the code and standard git defaults.