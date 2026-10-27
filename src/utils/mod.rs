//! What nodes need from the outside world, as narrow capability traits.
//!
//! Nodes are generic over these; [`System`] is the production implementation and tests script
//! their own.

use crate::model::{GraphLink, Hit, Link, Note};
use std::io;

pub mod clock;
pub mod embed;
pub mod index;
pub mod notes;
mod system;

pub use system::System;

/// Standard output and error.
pub trait Out {
    fn print(&self, message: &str);
    fn eprint(&self, message: &str);
}

/// Everything piped to standard input.
pub trait Stdin {
    fn read_stdin(&self) -> io::Result<String>;
}

/// Fresh identities and timestamps.
pub trait Clock {
    /// An ISO-8601 UTC timestamp with milliseconds.
    fn timestamp(&self) -> String;
    /// A new note id.
    fn new_id(&self) -> String;
}

/// The Markdown notes on disk, the source of truth.
pub trait NoteStore {
    /// `None` when there is no such note.
    fn read(&self, id: &str) -> io::Result<Option<Note>>;
    fn write(&self, note: &Note) -> io::Result<()>;
    fn list(&self) -> io::Result<Vec<String>>;
    fn remove(&self, id: &str) -> io::Result<()>;
}

/// The embeddings host. An unreachable host yields `None`, never an error.
pub trait Embedder {
    fn embed(&self, text: &str) -> Option<Vec<f32>>;
}

/// The derived SQLite index. Errors are the message to show the caller.
pub trait Index {
    /// Upserts a note, hashing its body. `None` leaves any stored vector alone.
    fn upsert(&self, note: &Note, embedding: Option<&[f32]>) -> Result<(), String>;
    /// The body hash the index holds for `id`, if it is indexed.
    fn indexed_hash(&self, id: &str) -> Result<Option<String>, String>;
    /// Adds links out of `from`, ignoring ones already there.
    fn add_links(&self, from: &str, links: &[Link]) -> Result<(), String>;
    /// Replaces every link out of `from`.
    fn replace_links(&self, from: &str, links: &[Link]) -> Result<(), String>;
    /// Every link touching `id`, in either direction.
    fn links_for(&self, id: &str) -> Result<Vec<GraphLink>, String>;
    /// Removes the entry for `id` and every link touching it.
    fn delete(&self, id: &str) -> Result<(), String>;
    /// Full-text matches, best first.
    fn keyword_search(&self, query: &str, limit: usize) -> Result<Vec<Hit>, String>;
    /// The notes closest to `vector`, best first.
    fn semantic_search(&self, vector: &[f32], limit: usize) -> Result<Vec<Hit>, String>;
    /// Drops entries whose id is not in `keep`, returning how many went.
    fn prune(&self, keep: &[String]) -> Result<usize, String>;
}
