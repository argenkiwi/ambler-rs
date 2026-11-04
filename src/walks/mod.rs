//! Walks: state machine graphs of nodes wired together. `counter` is the template's reference
//! walk; the rest are the eight verbs of the `azk` command line, one walk each.
//!
//! Each verb walk exports `Error` and `run(args)`; `src/main.rs` picks the verb.

pub mod counter;
pub mod create;
pub mod delete;
pub mod get;
pub mod help;
pub mod link;
pub mod reindex;
pub mod search;
pub mod update;

/// A bad invocation of a verb. The walk has already printed this usage message to stderr; the
/// process exits 1.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct UsageError(pub String);

impl UsageError {
    /// `Err` when a node queued a usage message, `Ok` otherwise.
    pub(crate) fn check(usage: Option<String>) -> Result<(), Self> {
        usage.map_or(Ok(()), |message| Err(Self(message)))
    }
}
