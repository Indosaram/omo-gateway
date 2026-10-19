# Lane r3c-dashboard-deploy

NO FINDINGS

**Note on method:** I have no shell, so I couldn't run `git diff HEAD`. I reviewed the current code of each changed region directly instead: `src/dashboard.rs:1209-1243` and `:4215-4239`, all of `docker-compose.yml`, `README.md:194`, and all of `scripts/deploy.sh`.

## The specific checks

- **F15 bind count:** The new `DELETE FROM sessions WHERE session_key = ? AND NOT EXISTS (SELECT 1 FROM cron_jobs WHERE session_key = ?)` has two placeholders and two `.bind(&storage_id)` calls (`dashboard.rs:1230-1236`). They match. The COUNT pre-check has one placeholder and one bind. It returns 409 with "session has N cron job(s)…".
- **F15 test:** `deleting_a_session_with_cron_jobs_is_refused` (`dashboard.rs:4216`) binds a job through `scheduler.register_with_id`, sends DELETE, and asserts 409. It also asserts the job still exists, so it checks the outcome that matters.
- **F15 404 when a job is bound mid-request:** If a cron job is registered between the COUNT and the DELETE, `rows_affected()==0` returns 404 "session not found", but the session still exists. The wording is wrong, but nothing is lost: the session and its job both survive, and only the session's running process (multiplexer) has already been stopped. The window is tiny and the next DELETE correctly returns 409. This is at most a P2 wording issue, below this lane's bar, so I didn't file it. The cleanest fix would be to retry the COUNT when zero rows are deleted and return 409 if jobs are now bound.
- **F12 nested `${A:-${B:-}}`:** Docker Compose v2 supports nested interpolation (documented). Only the legacy Python `docker-compose` v1 rejected it, and it is end-of-life. Values set in the project `.env` are interpolated too, so a token set only as `OMON_OMO_AUTH_TOKEN` in `.env` still reaches `OMON_OMO_APPSERVER_AUTH_TOKEN`.
  - When neither is set, the variable is an empty string. `omo_config.rs:302-306` filters empty strings out, so the token becomes `None`, which is correct.
  - `OMON_OMO_AUTOSPAWN: "off"` is quoted, so YAML won't turn it into a boolean. `omo_daemon.rs:443-452` accepts "off".
  - `extra_hosts: host.docker.internal:host-gateway` is valid.
- **F13 under `set -euo pipefail`:** `{ cp -p … && mv -f …; } || die …` is correct. Commands on the left of `||` are exempt from errexit, so a failed `cp` or `mv` reaches `die` instead of falling through to `restart`. `mv` within one directory is an atomic rename. The later `verify … && die A || die B` always exits with the right message, because `die A` exits before `||` can run.
  - Cosmetic only: if `cp` fails partway, a stale `$BIN.rollback` file is left behind. It causes no harm.
- **README:194** matches the compose defaults and env names.

## Adversarial pass

I found nothing new at P0/P1 in these four files. The remaining deploy.sh behaviour (the `verify` heuristics, `--verify` using `since="0000"`) was there before this change, so I skipped it as instructed.

## Verdict
0 P0, 0 P1.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/dashboard.rs - delete_session at 1209-1243 (F15) and its test at 4216
- /Users/indo/code/project/omon-gateway/docker-compose.yml - F12 env vars and extra_hosts
- /Users/indo/code/project/omon-gateway/scripts/deploy.sh - F13 rollback by copy then atomic rename
- /Users/indo/code/project/omon-gateway/README.md - line 194 documents the host app-server
- /Users/indo/code/project/omon-gateway/src/agent/omo_config.rs - 302-306: token alias, empty value treated as unset
- /Users/indo/code/project/omon-gateway/src/agent/omo_daemon.rs - 443-452: the "off" value is parsed
</files>
<answer>No findings. F15's bind count matches, and the 404 on a job bound mid-request is a harmless P2 wording issue that wasn't filed. F12's nested interpolation is valid in Compose v2. F13 behaves correctly under set -euo pipefail. 0 P0, 0 P1.</answer>
<next_steps>Ready to proceed. Optional P2: return 409 instead of 404 when the DELETE removes zero rows because a job was bound in the meantime.</next_steps>
</results>