//! `azk update`: applies a partial edit to a note. See `specs/update.md`.

use crate::model::Note;
use crate::nodes::embed::Embed;
use crate::nodes::exists_check::{self, ExistsCheck};
use crate::nodes::id_arg::{self, IdArg};
use crate::nodes::index_upsert::{self, IndexUpsert};
use crate::nodes::update::{Finish, Input, InputOutcome, Merge};
use crate::nodes::usage::Usage;
use crate::nodes::write_note::{self, WriteNote};
use crate::utils::{Clock, Embedder, Index, NoteStore, Out, Stdin, System};
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
    pub title: Option<String>,
    pub body: Option<String>,
    pub tags: Option<Vec<String>>,
    pub note: Option<Note>,
    pub text_to_embed: Option<String>,
    pub embedding: Option<Vec<f32>>,
}

impl_has!(State: verb, args, usage, error, id, fields, note, embedding);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    IdArg,
    Input,
    ExistsCheck,
    Merge,
    Embed,
    WriteNote,
    IndexUpsert,
    Finish,
    Usage,
}

pub struct Update<U = System> {
    id_arg: IdArg,
    input: Input<U>,
    exists_check: ExistsCheck<U>,
    merge: Merge<U>,
    embed: Embed<U>,
    write_note: WriteNote<U>,
    index_upsert: IndexUpsert<U>,
    finish: Finish<U>,
    usage: Usage<U>,
}

impl Update {
    #[must_use]
    pub fn new() -> Self {
        Self::with_utils(System)
    }
}

impl Default for Update {
    fn default() -> Self {
        Self::new()
    }
}

impl<U: Clone> Update<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self {
            // `update` needs stdin as well as an id, so the default usage line won't do.
            id_arg: IdArg::with_usage(r#"Usage: echo '{"body":"..."}' | azk update <id>"#),
            input: Input::with_utils(utils.clone()),
            exists_check: ExistsCheck::with_utils(utils.clone()),
            merge: Merge::with_utils(utils.clone()),
            embed: Embed::with_utils(utils.clone()),
            write_note: WriteNote::with_utils(utils.clone()),
            index_upsert: IndexUpsert::with_utils(utils.clone()),
            finish: Finish::with_utils(utils.clone()),
            usage: Usage::with_utils(utils),
        }
    }
}

impl<U> Walk for Update<U>
where
    U: Stdin + Clock + Embedder + NoteStore + Index + Out + Sync,
{
    type Id = Id;
    type State = State;
    type Error = Infallible;

    async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, Infallible> {
        Ok(match id {
            Id::IdArg => {
                let Ok(outcome) = self.id_arg.run(state).await;
                match outcome {
                    id_arg::Outcome::Given => Transition::Next(Id::Input),
                    id_arg::Outcome::Missing => Transition::Next(Id::Usage),
                }
            }
            Id::Input => {
                let Ok(outcome) = self.input.run(state).await;
                match outcome {
                    InputOutcome::Parsed => Transition::Next(Id::ExistsCheck),
                    InputOutcome::Invalid => Transition::Next(Id::Usage),
                }
            }
            Id::ExistsCheck => {
                let Ok(outcome) = self.exists_check.run(state).await;
                match outcome {
                    exists_check::Outcome::Found => Transition::Next(Id::Merge),
                    exists_check::Outcome::Missing => Transition::Halt,
                }
            }
            Id::Merge => {
                let Ok(()) = self.merge.run(state).await;
                Transition::Next(Id::Embed)
            }
            Id::Embed => {
                let Ok(()) = self.embed.run(state).await;
                Transition::Next(Id::WriteNote)
            }
            Id::WriteNote => {
                let Ok(outcome) = self.write_note.run(state).await;
                match outcome {
                    write_note::Outcome::Written => Transition::Next(Id::IndexUpsert),
                    write_note::Outcome::Failed => Transition::Halt,
                }
            }
            Id::IndexUpsert => {
                let Ok(outcome) = self.index_upsert.run(state).await;
                match outcome {
                    index_upsert::Outcome::Indexed => Transition::Next(Id::Finish),
                    index_upsert::Outcome::Failed => Transition::Halt,
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
        verb: "update".into(),
        args,
        ..State::default()
    };
    let Ok(()) = Update::new().run(Id::IdArg, &mut state).await;
    super::exit_code(&state.usage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Fake, note};
    use crate::utils::Index;
    use serde_json::json;

    async fn update(fake: &Fake, args: &[&str]) -> State {
        let mut state = State {
            verb: "update".into(),
            args: args.iter().map(ToString::to_string).collect(),
            ..State::default()
        };
        let Ok(()) = Update::with_utils(fake).run(Id::IdArg, &mut state).await;
        state
    }

    fn fake(stdin: &str) -> Fake {
        let fake = Fake {
            stdin: Some(stdin.into()),
            vector: Some(vec![0.0, 1.0]),
            ..Fake::default()
        }
        .with_note(note("a", "Old", "old body"));
        (&fake)
            .upsert(&note("a", "Old", "old body"), Some(&[1.0, 0.0]))
            .unwrap();
        fake
    }

    #[tokio::test]
    async fn a_new_body_is_re_embedded_and_the_vector_replaced() {
        let fake = fake(r#"{"body":"new body"}"#);

        update(&fake, &["a"]).await;

        let written = fake.file("a").unwrap();
        assert_eq!(
            (written.title.as_str(), written.body.as_str()),
            ("Old", "new body")
        );
        assert_eq!(written.updated, "2026-02-02T00:00:00.000Z");
        assert_eq!(*fake.embedded.lock().unwrap(), ["new body"]);
        assert_eq!(fake.stored_vector("a"), Some(vec![0.0, 1.0]));
        assert_eq!(fake.printed_json(), json!({"id":"a","updated":true}));
    }

    #[tokio::test]
    async fn a_title_only_edit_neither_embeds_nor_discards_the_vector() {
        let fake = fake(r#"{"title":"New"}"#);

        update(&fake, &["a"]).await;

        assert_eq!(fake.file("a").unwrap().title, "New");
        assert!(fake.embedded.lock().unwrap().is_empty());
        assert_eq!(fake.stored_vector("a"), Some(vec![1.0, 0.0]));
    }

    #[tokio::test]
    async fn an_empty_object_is_a_no_op_update() {
        let fake = fake("{}");

        let state = update(&fake, &["a"]).await;

        assert_eq!(state.usage, None);
        assert_eq!(fake.file("a").unwrap().body, "old body");
        assert_eq!(fake.printed_json(), json!({"id":"a","updated":true}));
    }

    #[tokio::test]
    async fn a_missing_id_prints_its_own_usage_line() {
        let fake = fake("{}");

        let state = update(&fake, &[]).await;

        assert!(state.usage.unwrap().contains("azk update <id>"));
        assert_eq!(fake.eprinted().len(), 1);
    }

    #[tokio::test]
    async fn malformed_json_goes_to_usage() {
        let fake = fake("nope");

        let state = update(&fake, &["a"]).await;

        assert_eq!(
            state.usage.as_deref(),
            Some(r#"{"error":"invalid JSON on stdin"}"#)
        );
        assert_eq!(fake.file("a").unwrap().title, "Old");
    }

    #[tokio::test]
    async fn an_unknown_id_is_an_error_value() {
        let fake = fake("{}");

        let state = update(&fake, &["zzz"]).await;

        assert_eq!(state.usage, None);
        assert_eq!(fake.printed_json(), json!({"error":"note zzz not found"}));
    }
}
