# Lane discord-auth

### [P2] Pairing failed-attempt counter never ages out, so occasional operator typos accumulate into a platform-wide lockout
- Location: src/discord/pairing.rs:183
- Trigger: An operator mistypes `/pair <code>` with an unknown code once a week. The counter only resets on a successful pairing, or on a failure after a lockout has expired. After the 5th stray typo, however far apart they were, `pairing_platform_lockout` is locked for 15 minutes.
- Impact: For 15 minutes no new pairing codes are issued, `check_and_record_notification_at` returns `None`, and every pending code returns `LockedOut`. Unpaired users who DM the bot during that window get no reply, and the cause is not obvious to anyone.
- Evidence: `WHEN strftime('%s', ?) >= strftime('%s', pairing_platform_lockout.locked_until) AND pairing_platform_lockout.failed_attempts >= ? THEN 1 / ELSE pairing_platform_lockout.failed_attempts + 1`. While `failed_attempts < 5`, no branch resets the count and `locked_until` is not consulted, so there is no time window. Only the `>= 5` case is reset after expiry.
- Fix: Also reset the count to 1 when the previous failure is older than a window, such as `LOCKOUT_DURATION_SECONDS`. That needs a `last_failed_at` column, or reusing `locked_until` as the last-failure timestamp for counts below 5. The condition then becomes `now >= locked_until` regardless of the attempt count.

I found no other defects whose root cause lies in the scope. The following are already fixed or declined, so I did not re-report them:
- the table placeholder upload failure and the `|` splitting in tables;
- the typing-error abort in the throttler;
- unknown pairing codes bumping every pending code's attempts;
- the over-broad "Always" pattern key.