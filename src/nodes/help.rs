//! The nodes only the `help` walk uses.

use crate::Node;
use crate::nodes::{HasArgs, HasUsage};
use crate::utils::Out;
use std::convert::Infallible;

/// `general`, or the verb whose guide to print.
pub trait HasTopic {
    fn topic(&self) -> &str;
    fn set_topic(&mut self, topic: String);
}

/// The guides are compiled in, so an installed binary needs no access to its source directory.
const GUIDES: [(&str, &str); 9] = [
    ("general", include_str!("../guides/general.md")),
    ("search", include_str!("../guides/search.md")),
    ("create", include_str!("../guides/create.md")),
    ("get", include_str!("../guides/get.md")),
    ("update", include_str!("../guides/update.md")),
    ("delete", include_str!("../guides/delete.md")),
    ("link", include_str!("../guides/link.md")),
    ("reindex", include_str!("../guides/reindex.md")),
    ("help", include_str!("../guides/help.md")),
];

fn guide(topic: &str) -> Option<&'static str> {
    GUIDES.iter().find(|(t, _)| *t == topic).map(|(_, g)| *g)
}

/// Where [`Input`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputOutcome {
    Known,
    /// A verb azk doesn't have; the usage message is queued.
    Unknown,
}

/// Reads the optional verb from the first argument, case-insensitively.
#[derive(Clone, Debug, Default)]
pub struct Input;

impl<S> Node<S> for Input
where
    S: HasArgs + HasTopic + HasUsage + Send,
{
    type Outcome = InputOutcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<InputOutcome, Infallible> {
        let topic = state
            .args()
            .first()
            .map_or_else(|| "general".to_string(), |verb| verb.to_lowercase());
        if guide(&topic).is_some() {
            state.set_topic(topic);
            Ok(InputOutcome::Known)
        } else {
            let verbs: Vec<_> = GUIDES.iter().skip(1).map(|(t, _)| *t).collect();
            *state.usage_mut() = Some(format!("Usage: azk help [{}]", verbs.join("|")));
            Ok(InputOutcome::Unknown)
        }
    }
}

/// Prints the resolved guide as plain text. Terminal.
#[derive(Clone, Debug, Default)]
pub struct Print<U> {
    utils: U,
}

impl<U> Print<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Print<U>
where
    S: HasTopic + Send,
    U: Out + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        if let Some(text) = guide(state.topic()) {
            self.utils.print(text.trim_end());
        }
        Ok(())
    }
}
