//! Re-exports profile extras dual store from `anylive-db`.
//!
//! Implementation lives in the db crate so SQL + memory backends stay together.
//! Call sites may import from here or from `anylive_db` directly.

#[allow(unused_imports)] // public re-export surface for API consumers
pub use anylive_db::{
    AnyProfileExtras, MemoryProfileExtras, PostgresProfileExtras, ProfileExtras,
};
