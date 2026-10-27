//! `azk create`: writes a new note from JSON on stdin. See `specs/create.md`.

use crate::model::{Link, Note};
use crate::nodes::create::{Finish, Init, Input, InputOutcome};
use crate::nodes::create_links::{self, CreateLinks};
use crate::nodes::embed::Embed;
use crate::nodes::index_upsert::{self, IndexUpsert};
use crate::nodes::usage::Usage;
use crate::nodes::write_note::{self, WriteNote};
use crate::utils::{Clock, Embedder, Index, NoteStore, Out, Stdin, System};
use crate::{Node, Transition, Walk, impl_has};
use std::convert::Infallible;
use std::process::ExitCode;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub usage: Option<String>,
    pub error: Option<String>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub tags: Option<Vec<String>>,
    pub links: Vec<Link>,
    pub from_id: Option<String>,
    pub note: Option<Note>,
    pub text_to_embed: Option<String>,
    pub embedding: Option<Vec<f32>>,
}

impl_has!(State: usage, error, fields, links, note, embedding);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Input,
    Init,
    Embed,
    WriteNote,
    IndexUpsert,
    CreateLinks,
    Finish,
    Usage,
}

pub struct Create<U = System> {
    input: Input<U>,
    init: Init<U>,
    embed: Embed<U>,
    write_note: WriteNote<U>,
    index_upsert: IndexUpsert<U>,
    create_links: CreateLinks<U>,
    finish: Finish<U>,
    usage: Usage<U>,
}

impl Create {
    #[must_use]
    pub fn new() -> Self {
        Self::with_utils(System)
    }
}

impl Default for Create {
    fn default() -> Self {
        Self::new()
    }
}

impl<U: Clone> Create<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self {
            input: Input::with_utils(utils.clone()),
            init: Init::with_utils(utils.clone()),
            embed: Embed::with_utils(utils.clone()),
            write_note: WriteNote::with_utils(utils.clone()),
            index_upsert: IndexUpsert::with_utils(utils.clone()),
            create_links: CreateLinks::with_utils(utils.clone()),
            finish: Finish::with_utils(utils.clone()),
            usage: Usage::with_utils(utils),
        }
    }
}

impl<U> Walk for Create<U>
where
    U: Stdin + Clock + Embedder + NoteStore + Index + Out + Sync,
{
    type Id = Id;
    type State = State;
    type Error = Infallible;

    async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, Infallible> {
        Ok(match id {
            Id::Input => {
                let Ok(outcome) = self.input.run(state).await;
                match outcome {
                    InputOutcome::Valid => Transition::Next(Id::Init),
                    InputOutcome::Invalid => Transition::Next(Id::Usage),
                }
            }
            Id::Init => {
                let Ok(()) = self.init.run(state).await;
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
                    index_upsert::Outcome::Indexed => Transition::Next(Id::CreateLinks),
                    index_upsert::Outcome::Failed => Transition::Halt,
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

pub async fn run(_args: Vec<String>) -> ExitCode {
    let mut state = State::default();
    let Ok(()) = Create::new().run(Id::Input, &mut state).await;
    super::exit_code(&state.usage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Fake, note};
    use serde_json::json;

    async fn create(fake: &Fake) -> State {
        let mut state = State::default();
        let Ok(()) = Create::with_utils(fake).run(Id::Input, &mut state).await;
        state
    }

    fn with_stdin(text: &str) -> Fake {
        Fake {
            stdin: Some(text.into()),
            vector: Some(vec![1.0, 0.0]),
            ..Fake::default()
        }
    }

    #[tokio::test]
    async fn writes_file_index_links_then_prints_the_summary() {
        let fake = with_stdin(
            r#"{"title":"T","body":"B","tags":["x"],"links":[{"toId":"other","relation":"builds on"}]}"#,
        );

        let state = create(&fake).await;

        assert_eq!(state.usage, None);
        let written = fake.file("new1").unwrap();
        assert_eq!((written.title.as_str(), written.body.as_str()), ("T", "B"));
        assert_eq!(written.created, written.updated);
        assert_eq!(written.links[0].to, "other");
        assert_eq!(fake.stored_vector("new1"), Some(vec![1.0, 0.0]));
        assert_eq!(fake.links.lock().unwrap()[0].from_id, "new1");
        assert_eq!(
            fake.printed_json(),
            json!({"id":"new1","title":"T","tags":["x"],"created":written.created,
                   "links":[{"toId":"other","relation":"builds on"}]})
        );
    }

    #[tokio::test]
    async fn creates_without_a_vector_when_the_host_is_unreachable() {
        let fake = Fake {
            vector: None,
            ..with_stdin(r#"{"title":"T","body":"B"}"#)
        };

        let state = create(&fake).await;

        assert_eq!(state.usage, None);
        assert_eq!(fake.stored_vector("new1"), None);
        assert!(fake.indexed.lock().unwrap().contains_key("new1"));
    }

    #[tokio::test]
    async fn malformed_json_goes_to_usage() {
        let fake = with_stdin("not json");

        let state = create(&fake).await;

        assert_eq!(
            state.usage.as_deref(),
            Some(r#"{"error":"invalid JSON on stdin"}"#)
        );
        assert_eq!(fake.eprinted(), [r#"{"error":"invalid JSON on stdin"}"#]);
        assert!(fake.printed().is_empty());
    }

    #[tokio::test]
    async fn a_missing_body_goes_to_usage() {
        let fake = with_stdin(r#"{"title":"T"}"#);

        let state = create(&fake).await;

        assert!(state.usage.unwrap().contains("title and body are required"));
        assert!(fake.files.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_index_failure_prints_an_error_and_exits_zero() {
        let fake = Fake {
            fail_index: true,
            ..with_stdin(r#"{"title":"T","body":"B"}"#)
        };

        let state = create(&fake).await;

        assert_eq!(state.usage, None);
        assert_eq!(state.error.as_deref(), Some("index is broken"));
        assert_eq!(fake.printed_json(), json!({"error":"index is broken"}));
        assert!(
            fake.file("new1").is_some(),
            "file is written before the index"
        );
    }

    #[tokio::test]
    async fn dangling_links_are_not_validated() {
        let fake =
            with_stdin(r#"{"title":"T","body":"B","links":[{"toId":"nope","relation":"r"}]}"#)
                .with_note(note("x", "X", "x"));

        let state = create(&fake).await;

        assert_eq!(state.error, None);
        assert_eq!(fake.links.lock().unwrap().len(), 1);
    }
}
