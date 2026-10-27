//! The nodes only the `update` walk uses.

use crate::Node;
use crate::nodes::{HasEmbedding, HasFields, HasId, HasNote, HasUsage, print_json};
use crate::utils::{Clock, Out, Stdin};
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;

/// Where [`Input`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputOutcome {
    Parsed,
    /// Malformed JSON; the usage message is queued.
    Invalid,
}

#[derive(Deserialize)]
struct Patch {
    title: Option<String>,
    body: Option<String>,
    tags: Option<Vec<String>>,
}

/// Reads a partial `{ title?, body?, tags? }` as JSON on stdin. `{}` is a valid no-op.
#[derive(Clone, Debug, Default)]
pub struct Input<U> {
    utils: U,
}

impl<U> Input<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Input<U>
where
    S: HasFields + HasUsage + Send,
    U: Stdin + Sync,
{
    type Outcome = InputOutcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<InputOutcome, Infallible> {
        let patch = self
            .utils
            .read_stdin()
            .ok()
            .and_then(|text| serde_json::from_str::<Patch>(&text).ok());
        let Some(patch) = patch else {
            *state.usage_mut() = Some(r#"{"error":"invalid JSON on stdin"}"#.into());
            return Ok(InputOutcome::Invalid);
        };
        *state.title_mut() = patch.title;
        *state.body_mut() = patch.body;
        *state.tags_mut() = patch.tags;
        Ok(InputOutcome::Parsed)
    }
}

/// Applies the supplied subset over the existing note and stamps it updated.
///
/// Queues the body for embedding **only** when one was supplied: a title or tag change would pay
/// a network round trip for a vector that wouldn't move.
#[derive(Clone, Debug, Default)]
pub struct Merge<U> {
    utils: U,
}

impl<U> Merge<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Merge<U>
where
    S: HasFields + HasNote + HasEmbedding + Send,
    U: Clock + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        let (title, body, tags) = (
            state.title().clone(),
            state.body().clone(),
            state.tags().clone(),
        );
        if let Some(note) = state.note_mut() {
            if let Some(title) = title {
                note.title = title;
            }
            if let Some(tags) = tags {
                note.tags = tags;
            }
            if let Some(body) = &body {
                note.body.clone_from(body);
            }
            note.updated = self.utils.timestamp();
        }
        *state.text_to_embed_mut() = body;
        Ok(())
    }
}

/// Prints the update confirmation. Terminal.
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
    S: HasId + Send,
    U: Out + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        let id = state.id().clone().unwrap_or_default();
        print_json(&self.utils, &json!({ "id": id, "updated": true }));
        Ok(())
    }
}
