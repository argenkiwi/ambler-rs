//! `azk help`: prints the general guide, or one verb's. See `specs/help.md`.

use crate::nodes::help::{HasTopic, Input, InputOutcome, Print};
use crate::nodes::usage::Usage;
use crate::utils::{Out, System};
use crate::{Node, Transition, Walk, impl_has};
use std::convert::Infallible;
use std::process::ExitCode;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub args: Vec<String>,
    pub usage: Option<String>,
    pub topic: String,
}

impl_has!(State: args, usage);

impl HasTopic for State {
    fn topic(&self) -> &str {
        &self.topic
    }
    fn set_topic(&mut self, topic: String) {
        self.topic = topic;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Input,
    Print,
    Usage,
}

pub struct Help<U = System> {
    input: Input,
    print: Print<U>,
    usage: Usage<U>,
}

impl Help {
    #[must_use]
    pub fn new() -> Self {
        Self::with_utils(System)
    }
}

impl Default for Help {
    fn default() -> Self {
        Self::new()
    }
}

impl<U: Clone> Help<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self {
            input: Input,
            print: Print::with_utils(utils.clone()),
            usage: Usage::with_utils(utils),
        }
    }
}

impl<U> Walk for Help<U>
where
    U: Out + Sync,
{
    type Id = Id;
    type State = State;
    type Error = Infallible;

    async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, Infallible> {
        Ok(match id {
            Id::Input => {
                let Ok(outcome) = self.input.run(state).await;
                match outcome {
                    InputOutcome::Known => Transition::Next(Id::Print),
                    InputOutcome::Unknown => Transition::Next(Id::Usage),
                }
            }
            Id::Print => {
                let Ok(()) = self.print.run(state).await;
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
    let Ok(()) = Help::new().run(Id::Input, &mut state).await;
    super::exit_code(&state.usage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fake;

    async fn help(fake: &Fake, args: &[&str]) -> State {
        let mut state = State {
            args: args.iter().map(ToString::to_string).collect(),
            ..State::default()
        };
        let Ok(()) = Help::with_utils(fake).run(Id::Input, &mut state).await;
        state
    }

    #[tokio::test]
    async fn no_argument_prints_the_general_guide_as_plain_text() {
        let fake = Fake::default();

        help(&fake, &[]).await;

        let printed = fake.printed();
        assert_eq!(printed.len(), 1);
        assert!(printed[0].starts_with("# azk\n"));
    }

    #[tokio::test]
    async fn a_verb_prints_its_guide_case_insensitively() {
        let fake = Fake::default();

        let state = help(&fake, &["SEARCH"]).await;

        assert_eq!(state.topic, "search");
        assert!(fake.printed()[0].starts_with("# azk search"));
    }

    #[tokio::test]
    async fn an_unknown_verb_lists_the_ones_with_guides_and_exits_one() {
        let fake = Fake::default();

        let state = help(&fake, &["frobnicate"]).await;

        let usage = state.usage.unwrap();
        assert!(usage.contains("search") && usage.contains("reindex"));
        assert!(fake.printed().is_empty());
        assert_eq!(fake.eprinted(), [usage]);
    }

    #[tokio::test]
    async fn every_verb_has_a_guide() {
        for verb in [
            "search", "create", "get", "update", "delete", "link", "reindex", "help",
        ] {
            let fake = Fake::default();
            let state = help(&fake, &[verb]).await;
            assert_eq!(state.usage, None, "{verb}");
            assert!(
                fake.printed()[0].starts_with(&format!("# azk {verb}")),
                "{verb}"
            );
        }
    }
}
