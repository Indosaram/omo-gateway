//! Context the gateway hands the lead engine ahead of an owner turn.
//!
//! The pack lives only in the engine prompt for that turn: it is never stored in `messages`,
//! so it is never re-packed or shown back to anyone.

pub mod pack;

pub use pack::{
    build_pack, install, owner_author_for, pack_for_turn, pack_has_foreign_text, wrap_prompt,
    ContextPackSettings, ContextPacker, LedgerPrecedents, NoPrecedents, PrecedentSource,
    ThreadReader, ENABLED_ENV, MAX_PACK_CHARS, MAX_PRECEDENTS, MAX_THREAD_MESSAGES,
};
