# Grouped repair source settlement

Owned sources are settled. No Cargo or runtime checks were run during this repair track.

- Removed only the obsolete update_referenced_message helper from src/storage/message_search.rs. LSP found no references; complete literal searches across src and tests found only its declaration. index_inbound and index_inbound_with_metadata remain intact. Actor and steered Discord callers still await index_inbound before the inbox wake.
- Removed the redundant Into conversion from export_profile in src/profiles/lanes_block.rs.
- Removed unnecessary mut from ad_web and ad_prefix in tests/test_inbox_store.rs; assertions unchanged.
- Removed unused default_http from src/main.rs; per-bot HTTP client routing remains intact.

Scoped rustfmt --check with edition 2021 and skip_children=true passed for all four owned files. bash -n passed for existing batch-gates.sh. No lint suppression or test weakening. No edits to backend.rs:61 or workerada's test_inbox_sources.rs.

Next sequence is recorded in next-gate-order.md: await st_01a12ada settlement, then one target-w4 jobs-2 all-targets/all-features compile-only --no-run gate under timeout_ms 1800000. Failure returns diagnostics without full-suite execution. Success immediately releases one full four-gate batch, including dashboard after changed main, also under timeout_ms 1800000. No interim source edits.
