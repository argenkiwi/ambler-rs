//! The nodes only the `reindex` walk uses.

use crate::Node;
use crate::model::{Link, body_hash};
use crate::nodes::{HasEmbedding, HasId, HasNote, StoreError, print_json};
use crate::utils::{Index, NoteStore, Out};
use serde_json::json;
use std::convert::Infallible;

/// The queue of notes to reindex and the counters accumulated while draining it.
pub trait HasQueue {
    /// Replaces the queue and zeroes the counters.
    fn seed(&mut self, ids: Vec<String>);
    /// Every note on disk, as listed at the start.
    fn all_ids(&self) -> &[String];
    /// Takes the next id off the queue.
    fn take_next(&mut self) -> Option<String>;
    fn count_indexed(&mut self);
    fn count_updated(&mut self);
    fn set_removed(&mut self, removed: usize);
    /// `(indexed, updated, removed, total)`
    fn summary(&self) -> (usize, usize, usize, usize);
}

/// Seeds the loop from the Markdown files on disk and zeroes the counters.
///
/// Must stay the entry node: the counters are accumulated onto downstream.
#[derive(Clone, Debug, Default)]
pub struct List<U> {
    utils: U,
}

impl<U> List<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for List<U>
where
    S: HasQueue + Send,
    U: NoteStore + Sync,
{
    type Outcome = ();
    type Error = StoreError;

    async fn run(&self, state: &mut S) -> Result<(), StoreError> {
        let ids = self
            .utils
            .list()
            .map_err(|e| StoreError(format!("could not list notes: {e}")))?;
        state.seed(ids);
        Ok(())
    }
}

/// Where [`Next`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextOutcome {
    Note,
    Done,
}

/// Takes the next note id off the queue. The only node that decides how long the reindex runs.
#[derive(Clone, Debug, Default)]
pub struct Next;

impl<S> Node<S> for Next
where
    S: HasQueue + HasId + Send,
{
    type Outcome = NextOutcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<NextOutcome, Infallible> {
        let next = state.take_next();
        let outcome = if next.is_some() {
            NextOutcome::Note
        } else {
            NextOutcome::Done
        };
        *state.id_mut() = next;
        Ok(outcome)
    }
}

/// Where [`DetectChange`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetectOutcome {
    Read,
    /// The file vanished between listing and reading.
    Vanished,
}

/// Reads the current note and decides whether it needs re-embedding, by comparing its body hash
/// with the index's, counting it as newly indexed or updated.
///
/// The pending embed text is cleared when nothing changed, so one note's body can never be
/// carried into the next one's embedding.
#[derive(Clone, Debug, Default)]
pub struct DetectChange<U> {
    utils: U,
}

impl<U> DetectChange<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for DetectChange<U>
where
    S: HasQueue + HasId + HasNote + HasEmbedding + Send,
    U: NoteStore + Index + Sync,
{
    type Outcome = DetectOutcome;
    type Error = StoreError;

    async fn run(&self, state: &mut S) -> Result<DetectOutcome, StoreError> {
        let id = state.id().clone().unwrap_or_default();
        let note = self
            .utils
            .read(&id)
            .map_err(|e| StoreError(format!("could not read note {id}: {e}")))?;
        let Some(note) = note else {
            *state.note_mut() = None;
            *state.text_to_embed_mut() = None;
            return Ok(DetectOutcome::Vanished);
        };
        let stored = self.utils.indexed_hash(&id).map_err(StoreError)?;
        let changed = match stored {
            None => {
                state.count_indexed();
                true
            }
            Some(hash) if hash != body_hash(&note.body) => {
                state.count_updated();
                true
            }
            Some(_) => false,
        };
        *state.text_to_embed_mut() = changed.then(|| note.body.clone());
        *state.note_mut() = Some(note);
        Ok(DetectOutcome::Read)
    }
}

/// Rebuilds the current note's outgoing links from its own frontmatter, changed or not, so a
/// hand-edit that only touched links is still picked up.
#[derive(Clone, Debug, Default)]
pub struct ReplaceLinks<U> {
    utils: U,
}

impl<U> ReplaceLinks<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for ReplaceLinks<U>
where
    S: HasNote + Send,
    U: Index + Sync,
{
    type Outcome = ();
    type Error = StoreError;

    async fn run(&self, state: &mut S) -> Result<(), StoreError> {
        if let Some(note) = state.note() {
            let links: Vec<Link> = note.links.iter().map(Link::from).collect();
            self.utils
                .replace_links(&note.id, &links)
                .map_err(StoreError)?;
        }
        Ok(())
    }
}

/// Drops index entries whose Markdown file no longer exists, so a note deleted by hand
/// disappears from search.
#[derive(Clone, Debug, Default)]
pub struct PruneOrphans<U> {
    utils: U,
}

impl<U> PruneOrphans<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for PruneOrphans<U>
where
    S: HasQueue + Send,
    U: Index + Sync,
{
    type Outcome = ();
    type Error = StoreError;

    async fn run(&self, state: &mut S) -> Result<(), StoreError> {
        let removed = self.utils.prune(state.all_ids()).map_err(StoreError)?;
        state.set_removed(removed);
        Ok(())
    }
}

/// Prints the reindex summary. Terminal.
#[derive(Clone, Debug, Default)]
pub struct Finish<U> {
    utils: U,
}

impl<U> Finish<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Finish<U>
where
    S: HasQueue + Send,
    U: Out + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        let (indexed, updated, removed, total) = state.summary();
        print_json(
            &self.utils,
            &json!({ "indexed": indexed, "updated": updated, "removed": removed, "total": total }),
        );
        Ok(())
    }
}
