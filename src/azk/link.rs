//! `azk link`: connects two existing notes. See `specs/link.md`.

use crate::model::{Link, Note};
use crate::nodes::create_links::{self, CreateLinks};
use crate::nodes::link::{Finish, HasLinkArgs, Input, InputOutcome, Validate, ValidateOutcome};
use crate::nodes::usage::Usage;
use crate::nodes::write_note::{self, WriteNote};
use crate::utils::{Index, NoteStore, Out, System};
use crate::{Node, Transition, Walk, impl_has};
use std::convert::Infallible;
use std::process::ExitCode;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub args: Vec<String>,
    pub usage: Option<String>,
    pub error: Option<String>,
    pub from_id: Option<String>,
    pub to_id: Option<String>,
    pub relation: Option<String>,
    pub note: Option<Note>,
    pub links: Vec<Link>,
}

impl_has!(State: args, usage, error, links, note);

impl HasLinkArgs for State {
    fn to_id(&self) -> &Option<String> {
        &self.to_id
    }
    fn to_id_mut(&mut self) -> &mut Option<String> {
        &mut self.to_id
    }
    fn relation(&self) -> &Option<String> {
        &self.relation
    }
    fn relation_mut(&mut self) -> &mut Option<String> {
        &mut self.relation
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Input,
    Validate,
    WriteNote,
    CreateLinks,
    Finish,
    Usage,
}

pub struct LinkWalk<U = System> {
    input: Input,
    validate: Validate<U>,
    write_note: WriteNote<U>,
    create_links: CreateLinks<U>,
    finish: Finish<U>,
    usage: Usage<U>,
}

impl LinkWalk {
    #[must_use]
    pub fn new() -> Self {
        Self::with_utils(System)
    }
}

impl Default for LinkWalk {
    fn default() -> Self {
        Self::new()
    }
}

impl<U: Clone> LinkWalk<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self {
            input: Input,
            validate: Validate::with_utils(utils.clone()),
            write_note: WriteNote::with_utils(utils.clone()),
            create_links: CreateLinks::with_utils(utils.clone()),
            finish: Finish::with_utils(utils.clone()),
            usage: Usage::with_utils(utils),
        }
    }
}

impl<U> Walk for LinkWalk<U>
where
    U: NoteStore + Index + Out + Sync,
{
    type Id = Id;
    type State = State;
    type Error = Infallible;

    async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, Infallible> {
        Ok(match id {
            Id::Input => {
                let Ok(outcome) = self.input.run(state).await;
                match outcome {
                    InputOutcome::Given => Transition::Next(Id::Validate),
                    InputOutcome::Missing => Transition::Next(Id::Usage),
                }
            }
            Id::Validate => {
                let Ok(outcome) = self.validate.run(state).await;
                match outcome {
                    ValidateOutcome::Valid => Transition::Next(Id::WriteNote),
                    ValidateOutcome::Invalid => Transition::Halt,
                }
            }
            Id::WriteNote => {
                let Ok(outcome) = self.write_note.run(state).await;
                match outcome {
                    write_note::Outcome::Written => Transition::Next(Id::CreateLinks),
                    write_note::Outcome::Failed => Transition::Halt,
                }
            }
            Id::CreateLinks => {
                let Ok(outcome) = self.create_links.run(state).await;
                match outcome {
                    create_links::Outcome::Recorded => Transition::Next(Id::Finish),
                    create_links::Outcome::Failed => Transition::Halt,
                }
            }
            Id::Finish => {
                let Ok(()) = self.finish.run(state).await;
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
        args,
        ..State::default()
    };
    let Ok(()) = LinkWalk::new().run(Id::Input, &mut state).await;
    super::exit_code(&state.usage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Fake, note};
    use serde_json::json;

    async fn link(fake: &Fake, args: &[&str]) -> State {
        let mut state = State {
            args: args.iter().map(ToString::to_string).collect(),
            ..State::default()
        };
        let Ok(()) = LinkWalk::with_utils(fake).run(Id::Input, &mut state).await;
        state
    }

    fn fake() -> Fake {
        Fake::default()
            .with_note(note("a", "A", "x"))
            .with_note(note("b", "B", "y"))
    }

    #[tokio::test]
    async fn writes_the_frontmatter_and_the_graph_but_does_not_reindex() {
        let fake = fake();

        link(&fake, &["a", "b", "builds on"]).await;

        let source = fake.file("a").unwrap();
        assert_eq!(source.links[0].to, "b");
        assert_eq!(fake.links.lock().unwrap().len(), 1);
        assert!(fake.indexed.lock().unwrap().is_empty());
        assert!(fake.embedded.lock().unwrap().is_empty());
        assert_eq!(
            fake.printed_json(),
            json!({"fromId":"a","toId":"b","relation":"builds on","linked":true})
        );
    }

    #[tokio::test]
    async fn linking_twice_does_not_duplicate_the_link() {
        let fake = fake();

        link(&fake, &["a", "b", "r"]).await;
        link(&fake, &["a", "b", "r"]).await;

        assert_eq!(fake.file("a").unwrap().links.len(), 1);
        assert_eq!(fake.links.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn both_ends_must_exist() {
        for args in [["a", "zzz", "r"], ["zzz", "b", "r"]] {
            let fake = fake();

            let state = link(&fake, &args).await;

            assert_eq!(state.usage, None);
            assert_eq!(fake.printed_json(), json!({"error":"note zzz not found"}));
            assert!(fake.links.lock().unwrap().is_empty());
            assert!(fake.file("a").unwrap().links.is_empty());
        }
    }

    #[tokio::test]
    async fn fewer_than_three_arguments_print_usage() {
        let fake = fake();

        let state = link(&fake, &["a", "b"]).await;

        assert!(state.usage.unwrap().starts_with("Usage: azk link"));
        assert!(fake.printed().is_empty());
    }
}
