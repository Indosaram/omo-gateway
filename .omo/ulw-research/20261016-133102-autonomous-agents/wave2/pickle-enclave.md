<!-- task=st_01a1451e -->
st_01a1451e [completed] model mahoquot/claude-sonnet-5-5
Ask: read Pickle's credential-enclave docs and release workflow and report what's worth copying. Wanted: a sourced report. For you: the report below is complete. Now: done. Next: none.

I did not run `grep` or build anything. I read three docs and the release workflow, but only the section headings and 5.8–5.10, §7 and §11 of `enclave.md`; the rest of `enclave.md` (5.1–5.7, 5.11–5.14, §6, §8–9, §12) is unread.

**Route:** `gh search repos` found the repo. Content came from `raw.githubusercontent.com` via curl and from `gh api .../contents` sliced with `--jq`. I did not need the webfetch, archive.org, jina, HN or Reddit routes, so none were tried. All sources are PRIMARY, pinned at `2dcf8957a4afb78a1c0400e3d37df0053faa97ad`.

Base: `https://github.com/pickle-com/credential-enclave/blob/2dcf8957a4afb78a1c0400e3d37df0053faa97ad/`. Line anchors weren't retrieved, so citations give file and section.

## Findings

- **Repo.** `pickle-com/credential-enclave` ("holds delegated user keys inside an AWS Nitro Enclave"). It has docs/, protocol/ (Rust crate), enclave/, host/, verify/, egress/*.tsv and scripts/secret-access.sh. [github.com/pickle-com/credential-enclave] PRIMARY, `gh search` + `gh api` tree.
- **Signed-value carriage.** Every signed JSON is sent as `{"body": b64u(bytes), "sig": b64u(sig)}`. The signature is checked over the received bytes and the body is never re-serialized. The Ed25519 signature input is `purpose-string\n || body`, with distinct purpose strings per message type (`...command\n`, `...reply\n`, `...log-entry\n`, `...head\n`). Every object carries `"v":1`. [docs/protocol.md §1, §3] PRIMARY, raw curl.
- **Grant.** The grant is a command `{v,type:"grant",user_id,node,challenge,sign_pk,log_pk,custody,user_key,not_after_ms,policy:{}}`. The app signs it with its account key. It is then HPKE-sealed (X25519/ChaCha20) to the node's sealing key, with the node id as AAD. [protocol.md §5.1–5.2] PRIMARY, raw curl.
- **Grant rules.**
  - Expiry is capped at now+30d by node time, and the app renews every 24h.
  - `policy` must be `{}`. Any key is refused with `unsupported_policy`, which reserves the field for future approval rules.
  - A grant with the wrong custody is refused.
  - A new grant for the same `user_id` replaces the old one.
  [protocol.md §5.2, §12] PRIMARY, raw curl.
- **Grant processing order.** There are 15 fixed steps, and a failure returns the code of the first failed check. The challenge is consumed only at step 12, after the cheap checks. The log entry is written and confirmed before the key enters memory (step 14). [protocol.md §5.3] PRIMARY, raw curl.
- **Challenge and revoke ordering.**
  - Challenges are single-use and live 300s.
  - One monotonic counter numbers both issued challenges and accepted revokes.
  - A grant is refused if its challenge serial is lower than the account's last revoke serial. This stops a carrier from replaying a held-back grant after a revoke.
  - After accepting any revoke, a node refuses delegation transfers for that account for the rest of its life.
  [protocol.md §5.3–5.4, §10.1; enclave.md §3 `serials`, `revoke_serial`, `revoked_here`] PRIMARY, raw curl + gh api.
- **Revoke.** Revoke is idempotent and never waits for the log store. It zeroizes the user key inside the account lock and leaves a `{key_id, sign_pk}` marker. If the store is down, the entry becomes a pending entry. [protocol.md §5.4, §7.6] PRIMARY, raw curl.
- **Reply freshness.** Each reply is node-signed and carries `challenge`, `key_id`, `head{seq,hash}` and `time_ms`. A separate `challenge_statement` signs `(nonce, challenge)`. This stops a relay from substituting an older challenge to replay an old signed reply. [protocol.md §4.2, §5.5] PRIMARY, raw curl.
- **Log chain.**
  - There is one chain per (node, account).
  - The entry body is `{v,node,user_id,key_id,seq,prev,time_ms,enc,ct}`.
  - The event is HPKE-sealed to the account's `log_pk`, with AAD `node\nuser\nkey_id\nseq`, so the operator sees only metadata.
  - `entry_hash = SHA256("pickle.secure.v1.log-chain\n" || body)`, and `prev` is the previous hash. The genesis `prev` is 32 zero bytes.
  - A signed `head{seq,hash,nonce,time_ms,final}` is returned on request, and a `final` head is signed at orderly shutdown.
  [protocol.md §7.1, §7.3] PRIMARY, raw curl.
- **Event types.** `grant_accepted`, `grant_revoked`, `connection_created`, `credential_refreshed`, `connection_removed`, `provider_request`, `secret_released`, `totp_issued`, `grant_transferred_out/in`. A `provider_request` carries method, host, path, query, `body_bytes`, `body_sha256` and a caller `context` of at most 256 chars. The `context` is unverified and shown separately in the UI. Identical GETs within 300s share one entry and carry `window_s:300`. [protocol.md §7.2] PRIMARY, raw curl.
- **Write-ahead to WORM storage.** The node PUTs each signed entry to an S3 bucket with Object Lock in COMPLIANCE mode and 365-day retention. The key is `v1/accounts/{user_id}/{node}/{seq:020}-{b64u(hash)}`. Only a 200 counts as confirmation, and the PUT sets no `If-None-Match` and does not treat "already exists" as confirmation. The act (provider call, release, key install) proceeds only after confirmation. A missing object for a `seq` means the act did not happen. Extra objects are possible. [protocol.md §7.6] PRIMARY, raw curl.
- **Pending entries and backlog.** Unconfirmed entries stay in memory and ride along on the next write. At 64 unacked entries per account, credential calls return `log_backlog`. Commands and transfers are exempt, because revocation must always work. [protocol.md §7.6, §12; enclave.md §3, §4] PRIMARY.
- **Verification rule.** A reader accepts a bucket object as an entry only if the node signature verifies and `entry_hash(body)` equals the hash in the key, because the operator holds the bucket credentials too. [protocol.md §7.6] PRIMARY.
- **Egress policy E1–E6.**
  - E1: secrets are `Secret<T>` types with no `Serialize`, `Display` or `ToString`, a fixed `Debug` output and zeroize on drop. Response bodies must implement the `OperatorResponse` marker trait. A response type holding a secret does not compile (`compile_fail` tests).
  - E2: secrets are read only via `expose_secret`, in a closed list of sinks K1–K7. `scripts/secret-access.sh --check` compares every function that calls `expose_secret` or a sink against `egress/*.tsv`, and CI fails on drift.
  - E3: every response field is classified P/E/D/C/S/V/R. A test compares the router's calls to the doc's table.
  - E4: provider responses are scanned for the injected credential. The scan covers raw, percent-decoded up to 3 times, base64/base64url at 3 alignments, and hex. The `Content-Encoding` and `Transfer-Encoding` checks and the `Set-Cookie` strip are in the same rule. A hit returns `response_withheld`.
  - E5: error codes are a closed vocabulary with `&'static str` messages.
  - E6: record `kind` is bound into the AEAD AAD and decides which open function can read it.
  [docs/egress-policy.md §3] PRIMARY, raw curl.
- **Canary test.** It plants high-entropy markers for every secret and has a hostile provider stub reflect them. It asserts no marker appears in any response in any encoding, and that credentials appear only in requests to the right provider host. [egress-policy.md "The canary test"] PRIMARY.
- **Address allowlist for forward.**
  - Per-provider host + path prefix/exact/suffix rules.
  - https and port 443 only; no userinfo, IP literal or trailing dot.
  - Refused path forms: `%2f`, `%5c`, `//`, `\`, `%252e` and dot segments including `..;x`.
  - A second parse with the WHATWG `url` crate must agree with the first.
  - A `denied` list is compared across decode depths 1–3, lowercased, with `;` stripped (this blocks the Graph `$batch` bypass).
  - Redirects are not followed. Method-override headers are refused.
  [docs/enclave.md §7, §5.8] PRIMARY, gh api.
- **Stated limits.** Path prefixes are wide, and APIs that open lasting access (watches, share links) are only logged. Response size and timing leak. The app is trusted. [egress-policy.md §5] PRIMARY.
- **Release workflow.**
  - Triggered by a `v*` tag, with `permissions: {}` at the top level.
  - Steps: `check` (fmt, clippy, tests, vector check, secret-access check, `cargo audit`), then two independent builds that must produce identical `measurements.json`, then `publish`.
  - `publish` uses `actions/attest-build-provenance` and `actions/attest` with a custom predicate type `https://pickle.com/credential-enclave/release/v1`. The bundles are attached to the GitHub release. Actions are pinned by SHA.
  [.github/workflows/release.yml; enclave.md §11.2] PRIMARY, raw curl.
- **Sigstore verification rules.**
  - Check the DSSE signature and the Fulcio chain.
  - Check certificate extensions: issuer `token.actions.githubusercontent.com`, repo, workflow `.github/workflows/release.yml`, ref `refs/tags/v*`.
  - Check the Rekor proof against a pinned key.
  [protocol.md §9] PRIMARY. Per `protocol.md` §9, this is stage 3 and not implemented in the current release.
- **Node-to-node transfer.**
  - A node hands a grant only to a node whose Nitro attestation verifies with identical PCR0/1/2, the same log bucket, and no all-zero (debug) PCRs.
  - For a newer release, it also needs a `hashedrekord` endorsement. The operator's release key signs `measurements.json`, and the node checks the Rekor signed entry timestamp (SET) with a compiled-in key.
  - The node does not verify inclusion proofs. The doc says so.
  - The receiving build compiles in a predecessor list.
  - One `grant_transferred_out` entry is made per (grant, receiver), with at most 64 receivers per grant.
  [protocol.md §10.1, §10.3; egress-policy.md §5] PRIMARY.

## Design patterns worth copying

1. **Wire envelope.** Signed value = `{body_b64u, sig}`, with a purpose-prefixed signature input and `v:1` everywhere. In Rust, use `ed25519-dalek` with `Vec<u8>` bodies and parse after verifying. Keep the purpose strings as one `const` table.
2. **Grant** as `struct Grant {user_id, sign_pk, log_pk, custody, user_key: Secret<[u8;32]>, not_after_ms, policy}`. For a personal gateway, make `policy` a real, typed approval ruleset (for example `require_approval: Vec<ToolMatcher>`) and have `deny_unknown_fields` reject keys the build doesn't implement.
3. **State machine:** `GrantState = None | Active | Expired{key_id,not_after} | Revoked{key_id,sign_pk}`. Check expiry at every read site and zeroize on transition. Never hold an Active state with a zeroed key.
4. **Ordering.** Use one `AtomicU64` serial shared by challenges and revokes. A grant must carry a challenge serial greater than the last revoke serial. Add a sticky `revoked_here` flag that blocks transfer or import.
5. **Intent log with write-ahead.** Entry creation and `head` update happen inside the per-account `Mutex`. The store write and the provider call happen outside the lock. Gate the act on a durable confirmation. Revoke and local-state erasure never wait on the store. Keep a bounded `unacked` queue, and refuse credential use at the cap while still allowing commands.
6. **Self-hosted WORM.** For a personal box, use an append-only file plus periodic anchoring to S3 Object Lock or another append-only target. Name objects `seq-hash`, and accept one only if the signature and the hash-in-key both verify.
7. **Typed egress.**
   - Make the secret type non-`Serialize`, with a sealed `ResponseBody` marker trait.
   - Put every `expose_secret` call in a checked list (`egress/*.tsv`) with a CI script.
   - Add a canary test that plants markers and runs a hostile upstream stub through every router endpoint.
   - Add an E4-style reflection scan (raw, percent, base64 at 3 alignments, hex).
8. **Per-tool egress allowlist.** Pin host, port 443 and path rules per upstream. Run two URL parsers and require them to agree. Refuse encoded-slash and dot-segment forms. Add an explicit `denied` list for batch endpoints that would hide sub-requests from the log. Do not follow redirects.
9. **Approval rule candidate (my inference, not Pickle's).** Pickle's log entry names method, host, path and body hash, so a human can approve per entry. Pair that with the reserved `policy` field to hold side-effecting methods (POST, PUT, DELETE, PATCH) for approval while GETs stay merge-able.
10. **Release.**
    - Run a two-job reproducible build and require equal outputs.
    - Use `permissions: {}` by default and SHA-pinned actions.
    - Attest `actions/attest` with a custom predicate that holds the measurements and bundle files.
    - Verify with a pinned identity, and for Rekor go beyond Pickle by checking inclusion proofs. Pickle verifies only the SET.

## CLAIMS

| CLAIM | RISK | SOURCES | PRIMARY |
|---|---|---|---|
| Pickle logs every credential-using act before performing it and gates the act on durable confirmation. | Low | protocol.md §7.6 | Yes |
| The operator domain cannot read plaintext tokens from responses. | Medium. It depends on provider contract adherence and on a code-review claim. Section 5 of egress-policy.md lists the limits. | egress-policy.md §3, §5 | Yes |
| Rekor is verified without an inclusion proof. | Medium. The doc says so and shifts the trust to Rekor's operation. | protocol.md §10.3; egress-policy.md §5 | Yes |
| Sigstore bundle verification (§9) and the grant allowlist of measurements are not active yet. | Medium. The code may have changed since the pinned SHA. | protocol.md stage table, §4.5 | Yes |
| The endorsement signing process is external to the repo. | Low | enclave.md §11.1 | Yes |
| Revoke-after-grant replay is blocked by the serial rule. | Low | protocol.md §5.4 | Yes |
| The 30-day cap and 24h renewal are node-side limits. | Low | protocol.md §5.2, §12 | Yes |

## EXPAND

- Read the unread parts of `enclave.md`: §5.13 (peer export/import code), §5.14 (log-store credentials), §6 (provider definition format), §8 (limits) and §12.
- Read `protocol/src/log.rs`, `envelope.rs` and `secret.rs`, and `verify/src/evaluate.rs`, to copy the actual Rust types.
- Read `scripts/secret-access.sh` and `egress/secret-access.tsv` to copy the CI audit mechanism.
- Check `protocol/vectors.json` for test-vector structure.
- Check the repo's issues and PRs for later changes to stage 3.