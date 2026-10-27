//! The nodes only the `link` walk uses.

use crate::Node;
use crate::model::{Link, NoteLink};
use crate::nodes::{HasArgs, HasError, HasLinks, HasNote, HasUsage, fail, print_json};
use crate::utils::{NoteStore, Out};
use serde_json::json;
use std::convert::Infallible;

/// The link being made: `from_id` (see [`HasLinks`]) points at `to_id` with `relation`.
pub trait HasLinkArgs {
    fn to_id(&self) -> &Option<String>;
    fn to_id_mut(&mut self) -> &mut Option<String>;
    fn relation(&self) -> &Option<String>;
    fn relation_mut(&mut self) -> &mut Option<String>;
}

/// Where [`Input`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputOutcome {
    Given,
    /// Fewer than three arguments; the usage message is queued.
    Missing,
}

/// Reads the source id, target id and relation phrase from the arguments.
#[derive(Clone, Debug, Default)]
pub struct Input;

impl<S> Node<S> for Input
where
    S: HasArgs + HasLinks + HasLinkArgs + HasUsage + Send,
{
    type Outcome = InputOutcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<InputOutcome, Infallible> {
        let [from, to, relation, ..] = state.args() else {
            *state.usage_mut() = Some(r#"Usage: azk link <fromId> <toId> "<relation>""#.into());
            return Ok(InputOutcome::Missing);
        };
        let (from, to, relation) = (from.clone(), to.clone(), relation.clone());
        *state.source_id_mut() = Some(from);
        *state.to_id_mut() = Some(to);
        *state.relation_mut() = Some(relation);
        Ok(InputOutcome::Given)
    }
}

/// Where [`Validate`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidateOutcome {
    Valid,
    /// An end of the link is missing; the error was printed.
    Invalid,
}

/// Requires both ends to exist, then appends the link to the source note's frontmatter and
/// restates it in the form the link graph wants.
#[derive(Clone, Debug, Default)]
pub struct Validate<U> {
    utils: U,
}

impl<U> Validate<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Validate<U>
where
    S: HasLinks + HasLinkArgs + HasNote + HasError + Send,
    U: NoteStore + Out + Sync,
{
    type Outcome = ValidateOutcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<ValidateOutcome, Infallible> {
        let from = state.source_id().clone().unwrap_or_default();
        let to = state.to_id().clone().unwrap_or_default();
        let relation = state.relation().clone().unwrap_or_default();
        let mut notes = Vec::new();
        for id in [&from, &to] {
            match self.utils.read(id) {
                Ok(Some(note)) => notes.push(note),
                Ok(None) => {
                    fail(&self.utils, state, format!("note {id} not found"));
                    return Ok(ValidateOutcome::Invalid);
                }
                Err(e) => {
                    fail(&self.utils, state, format!("could not read note {id}: {e}"));
                    return Ok(ValidateOutcome::Invalid);
                }
            }
        }
        let mut source = notes.swap_remove(0);
        let link = NoteLink {
            to: to.clone(),
            relation: relation.clone(),
        };
        if !source.links.contains(&link) {
            source.links.push(link);
        }
        *state.note_mut() = Some(source);
        *state.links_mut() = vec![Link {
            to_id: to,
            relation,
        }];
        Ok(ValidateOutcome::Valid)
    }
}

/// Prints the link confirmation. Terminal.
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
    S: HasLinks + HasLinkArgs + Send,
    U: Out + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        print_json(
            &self.utils,
            &json!({
                "fromId": state.source_id(),
                "toId": state.to_id(),
                "relation": state.relation(),
                "linked": true,
            }),
        );
        Ok(())
    }
}
