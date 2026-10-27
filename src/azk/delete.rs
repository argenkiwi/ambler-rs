//! `azk delete`: removes a note and every link touching it. See `specs/delete.md`.

use crate::model::Note;
use crate::nodes::delete::Delete;
use crate::nodes::exists_check::{self, ExistsCheck};
use crate::nodes::id_arg::{self, IdArg};
use crate::nodes::usage::Usage;
use crate::utils::{Index, NoteStore, Out, System};
use crate::{Node, Transition, Walk, impl_has};
use std::convert::Infallible;
use std::process::ExitCode;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub verb: String,
    pub args: Vec<String>,
    pub usage: Option<String>,
    pub error: Option<String>,
    pub id: Option<String>,
    pub note: Option<Note>,
}

impl_has!(State: verb, args, usage, error, id, note);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    IdArg,
    ExistsCheck,
    Delete,
    Usage,
}

pub struct DeleteWalk<U = System> {
    id_arg: IdArg,
    exists_check: ExistsCheck<U>,
    delete: Delete<U>,
    usage: Usage<U>,
}

impl DeleteWalk {
    #[must_use]
    pub fn new() -> Self {
        Self::with_utils(System)
    }
}

impl Default for DeleteWalk {
    fn default() -> Self {
        Self::new()
    }
}

impl<U: Clone> DeleteWalk<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self {
            id_arg: IdArg::new(),
            exists_check: ExistsCheck::with_utils(utils.clone()),
            delete: Delete::with_utils(utils.clone()),
            usage: Usage::with_utils(utils),
        }
    }
}

impl<U> Walk for DeleteWalk<U>
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
                    exists_check::Outcome::Found => Transition::Next(Id::Delete),
                    exists_check::Outcome::Missing => Transition::Halt,
                }
            }
            Id::Delete => {
                let Ok(()) = self.delete.run(state).await;
                Transition::Halt
            }
            Id::Usage => {
                let Ok(()) = self.usage.run(state).await;
                Transition::Halt
            }
        })
    }
}

pub async fn run(args: Vec<String>) -> ExitCode {
    let mut state = State {
        verb: "delete".into(),
        args,
        ..State::default()
    };
    let Ok(()) = DeleteWalk::new().run(Id::IdArg, &mut state).await;
    super::exit_code(&state.usage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GraphLink, Link};
    use crate::testing::{Fake, note};
    use serde_json::json;

    async fn delete(fake: &Fake, args: &[&str]) -> State {
        let mut state = State {
            verb: "delete".into(),
            args: args.iter().map(ToString::to_string).collect(),
            ..State::default()
        };
        let Ok(()) = DeleteWalk::with_utils(fake)
            .run(Id::IdArg, &mut state)
            .await;
        state
    }

    #[tokio::test]
    async fn removes_file_entry_and_links_in_both_directions() {
        let fake = Fake::default().with_note(note("a", "A", "x"));
        let link = |to: &str| Link {
            to_id: to.into(),
            relation: "r".into(),
        };
        {
            use crate::utils::Index;
            (&fake).upsert(&note("a", "A", "x"), None).unwrap();
            (&fake).add_links("a", &[link("b")]).unwrap();
            (&fake).add_links("c", &[link("a")]).unwrap();
            (&fake).add_links("c", &[link("d")]).unwrap();
        }

        delete(&fake, &["a"]).await;

        assert!(fake.file("a").is_none());
        assert!(!fake.indexed.lock().unwrap().contains_key("a"));
        assert_eq!(
            *fake.links.lock().unwrap(),
            [GraphLink {
                from_id: "c".into(),
                to_id: "d".into(),
                relation: "r".into()
            }]
        );
        assert_eq!(fake.printed_json(), json!({"id":"a","deleted":true}));
    }

    #[tokio::test]
    async fn a_missing_note_is_an_error_value() {
        let fake = Fake::default();

        let state = delete(&fake, &["nope"]).await;

        assert_eq!(state.usage, None);
        assert_eq!(fake.printed_json(), json!({"error":"note nope not found"}));
    }

    #[tokio::test]
    async fn no_id_prints_usage() {
        let fake = Fake::default();

        let state = delete(&fake, &[]).await;

        assert_eq!(state.usage.as_deref(), Some("Usage: azk delete <id>"));
    }
}
