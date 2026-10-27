//! The nodes only the `create` walk uses.

use crate::Node;
use crate::model::{Link, Note, NoteLink};
use crate::nodes::{HasEmbedding, HasFields, HasLinks, HasNote, HasUsage, print_json};
use crate::utils::{Clock, Out, Stdin};
use serde_json::{Value, json};
use std::convert::Infallible;

/// Where [`Input`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputOutcome {
    Valid,
    /// Malformed JSON, or no `title` or `body`; the usage message is queued.
    Invalid,
}

/// Reads `{ title, body, tags?, links? }` as JSON on stdin.
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

const INVALID_JSON: &str = r#"{"error":"invalid JSON on stdin"}"#;
const REQUIRED: &str = r#"{"error":"title and body are required"}"#;

fn strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(str::to_string))
        .collect()
}

impl<S, U> Node<S> for Input<U>
where
    S: HasFields + HasLinks + HasUsage + Send,
    U: Stdin + Sync,
{
    type Outcome = InputOutcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<InputOutcome, Infallible> {
        let parsed = self
            .utils
            .read_stdin()
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .filter(Value::is_object);
        let Some(json) = parsed else {
            *state.usage_mut() = Some(INVALID_JSON.into());
            return Ok(InputOutcome::Invalid);
        };
        let (Some(title), Some(body)) = (json["title"].as_str(), json["body"].as_str()) else {
            *state.usage_mut() = Some(REQUIRED.into());
            return Ok(InputOutcome::Invalid);
        };
        let links = match json.get("links") {
            None | Some(Value::Null) => Some(Vec::new()),
            Some(v) => serde_json::from_value::<Vec<Link>>(v.clone()).ok(),
        };
        let Some(links) = links else {
            *state.usage_mut() = Some(INVALID_JSON.into());
            return Ok(InputOutcome::Invalid);
        };
        *state.title_mut() = Some(title.to_string());
        *state.body_mut() = Some(body.to_string());
        *state.tags_mut() = Some(json.get("tags").and_then(strings).unwrap_or_default());
        *state.links_mut() = links;
        Ok(InputOutcome::Valid)
    }
}

/// Assembles the new note in memory: fresh id, matching timestamps, links in frontmatter form.
///
/// Links are not validated here, unlike the `link` walk: a missing target is a dangling link.
#[derive(Clone, Debug, Default)]
pub struct Init<U> {
    utils: U,
}

impl<U> Init<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Init<U>
where
    S: HasFields + HasLinks + HasNote + HasEmbedding + Send,
    U: Clock + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        let id = self.utils.new_id();
        let now = self.utils.timestamp();
        let body = state.body().clone().unwrap_or_default();
        *state.note_mut() = Some(Note {
            id: id.clone(),
            title: state.title().clone().unwrap_or_default(),
            tags: state.tags().clone().unwrap_or_default(),
            created: now.clone(),
            updated: now,
            links: state.links().iter().map(NoteLink::from).collect(),
            body: body.clone(),
        });
        *state.source_id_mut() = Some(id);
        *state.text_to_embed_mut() = Some(body);
        Ok(())
    }
}

/// Prints the created note's summary. Terminal.
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
    S: HasNote + HasLinks + Send,
    U: Out + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        if let Some(note) = state.note() {
            print_json(
                &self.utils,
                &json!({
                    "id": note.id,
                    "title": note.title,
                    "tags": note.tags,
                    "created": note.created,
                    "links": state.links(),
                }),
            );
        }
        Ok(())
    }
}
