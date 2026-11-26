use crate::Node;
use crate::guides;
use crate::nodes::{HasArgs, HasExitCode, HasMessage};
use std::convert::Infallible;

pub trait Utils {
    fn print(&self, message: &str);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemUtils;

impl Utils for SystemUtils {
    fn print(&self, message: &str) {
        println!("{message}");
    }
}

/// Where [`Help`] leaves the walk: the guide was printed, or there is none for the topic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Printed,
    Unknown,
}

/// Prints the guide for the topic in the arguments, or the general guide if there is none, on
/// standard output.
#[derive(Clone, Debug, Default)]
pub struct Help<U = SystemUtils> {
    utils: U,
}

impl Help {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<U> Help<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Help<U>
where
    S: HasArgs + HasMessage + HasExitCode + Send,
    U: Utils + Sync,
{
    type Outcome = Outcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<Outcome, Infallible> {
        let topic = state.args().first().map_or("", String::as_str);
        if let Some(text) = guides::guide(topic) {
            self.utils.print(text);
            return Ok(Outcome::Printed);
        }
        *state.message_mut() = Some(format!(
            "No guide for {topic}. Topics: {}",
            guides::TOPICS.join(", ")
        ));
        *state.exit_code_mut() = 1;
        Ok(Outcome::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guides::TOPICS;
    use crate::testing::{Fake, State};

    async fn help(fake: &Fake, args: &[&str]) -> (Outcome, State) {
        let mut state = State::with_args(args);
        let Ok(outcome) = Help::with_utils(fake).run(&mut state).await;
        (outcome, state)
    }

    #[tokio::test]
    async fn prints_the_general_guide_without_a_topic() {
        let fake = Fake::default();

        let (outcome, state) = help(&fake, &[]).await;

        assert_eq!(outcome, Outcome::Printed);
        assert_eq!(fake.printed(), [guides::guide("").unwrap()]);
        assert_eq!(state.message, None);
        assert_eq!(state.exit_code, 0);
    }

    #[tokio::test]
    async fn prints_the_guide_for_a_known_topic() {
        let fake = Fake::default();

        let (outcome, state) = help(&fake, &["node-design"]).await;

        assert_eq!(outcome, Outcome::Printed);
        assert_eq!(fake.printed(), [guides::guide("node-design").unwrap()]);
        assert_eq!(state.exit_code, 0);
    }

    #[tokio::test]
    async fn records_the_known_topics_for_an_unknown_one_and_prints_nothing() {
        let fake = Fake::default();

        let (outcome, state) = help(&fake, &["bogus"]).await;

        assert_eq!(outcome, Outcome::Unknown);
        assert!(fake.printed().is_empty());
        assert_eq!(
            state.message,
            Some(format!("No guide for bogus. Topics: {}", TOPICS.join(", ")))
        );
        assert_eq!(state.exit_code, 1);
    }
}
