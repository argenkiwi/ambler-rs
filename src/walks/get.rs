//! `azk get`: fetches one note and its links. See `specs/get.md`.

use crate::model::Note;
use crate::nodes::exists_check::{self, ExistsCheck};
use crate::nodes::get::Get;
use crate::nodes::id_arg::{self, IdArg};
use crate::nodes::usage::Usage;
use crate::utils::{Index, NoteStore, Out, System};
use crate::{Node, Transition, Walk, impl_has};
use std::convert::Infallible;

/// A bad invocation of the verb.
pub type Error = super::UsageError;

#[derive(Clone, Debug, Default)]
struct State {
    verb: String,
    args: Vec<String>,
    usage: Option<String>,
    error: Option<String>,
    id: Option<String>,
    note: Option<Note>,
}

impl_has!(State: verb, args, usage, error, id, note);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::enum_variant_names,
    reason = "variants mirror the node names in specs/"
)]
enum Id {
    IdArg,
    ExistsCheck,
    Get,
    Usage,
}

struct GetWalk<U = System> {
    id_arg: IdArg,
    exists_check: ExistsCheck<U>,
    get: Get<U>,
    usage: Usage<U>,
}

impl GetWalk {
    #[must_use]
    fn new() -> Self {
        Self::with_utils(System)
    }
}

impl Default for GetWalk {
    fn default() -> Self {
        Self::new()
    }
}

impl<U: Clone> GetWalk<U> {
    #[must_use]
    fn with_utils(utils: U) -> Self {
        Self {
            id_arg: IdArg::new(),
            exists_check: ExistsCheck::with_utils(utils.clone()),
            get: Get::with_utils(utils.clone()),
            usage: Usage::with_utils(utils),
        }
    }
}

impl<U> Walk for GetWalk<U>
where
    U: NoteStore + Index + Out + Sync,
{
    type Id = Id;
    type State = State;
    type Error = Infallible;

    async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, Infallible> {
        Ok(match id {
            Id::IdArg => {
                let Ok(outcome) = self.id_arg.run(state).await;
                match outcome {
                    id_arg::Outcome::Given => Transition::Next(Id::ExistsCheck),
                    id_arg::Outcome::Missing => Transition::Next(Id::Usage),
                }
            }
            Id::ExistsCheck => {
                let Ok(outcome) = self.exists_check.run(state).await;
                match outcome {
                    exists_check::Outcome::Found => Transition::Next(Id::Get),
                    exists_check::Outcome::Missing => Transition::Halt,
                }
            }
            Id::Get => {
                let Ok(()) = self.get.run(state).await;
                Transition::Halt
            }
            Id::Usage => {
                let Ok(()) = self.usage.run(state).await;
                Transition::Halt
            }
        })
    }
}

/// Runs `azk get <id>`.
///
/// A failure inside the verb (a missing note, an unreachable index) is printed as `{ "error" }`
/// on stdout and still returns `Ok`, as the spec requires.
///
/// # Errors
///
/// Returns [`Error`] when the invocation is bad; its usage message has already gone to stderr.
pub async fn run(args: Vec<String>) -> Result<(), Error> {
    let mut state = State {
        verb: "get".into(),
        args,
        ..State::default()
    };
    let Ok(()) = GetWalk::new().run(Id::IdArg, &mut state).await;
    super::UsageError::check(state.usage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::GraphLink;
    use crate::testing::{Fake, note};
    use serde_json::json;

    async fn get(fake: &Fake, args: &[&str]) -> State {
        let mut state = State {
            verb: "get".into(),
            args: args.iter().map(ToString::to_string).collect(),
            ..State::default()
        };
        let Ok(()) = GetWalk::with_utils(fake).run(Id::IdArg, &mut state).await;
        state
    }

    #[tokio::test]
    async fn prints_the_note_with_links_in_both_directions() {
        let fake = Fake::default().with_note(note("a", "A", "body"));
        fake.links.lock().unwrap().extend([
            GraphLink {
                from_id: "a".into(),
                to_id: "b".into(),
                relation: "r".into(),
            },
            GraphLink {
                from_id: "c".into(),
                to_id: "a".into(),
                relation: "s".into(),
            },
            GraphLink {
                from_id: "c".into(),
                to_id: "d".into(),
                relation: "t".into(),
            },
        ]);

        get(&fake, &["a"]).await;

        let out = fake.printed_json();
        assert_eq!(out["body"], "body");
        assert_eq!(out["links"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn a_missing_note_is_an_error_value_not_usage() {
        let fake = Fake::default();

        let state = get(&fake, &["nope"]).await;

        assert_eq!(state.usage, None);
        assert_eq!(fake.printed_json(), json!({"error":"note nope not found"}));
    }

    #[tokio::test]
    async fn no_id_prints_usage_to_stderr() {
        let fake = Fake::default();

        let state = get(&fake, &[]).await;

        assert_eq!(state.usage.as_deref(), Some("Usage: azk get <id>"));
        assert_eq!(fake.eprinted(), ["Usage: azk get <id>"]);
        assert!(fake.printed().is_empty());
    }
}
