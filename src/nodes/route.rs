use crate::Node;
use crate::nodes::{HasArgs, HasExitCode, HasLintOnly, HasMessage};
use std::convert::Infallible;

/// The usage of every verb and the exit codes.
pub const USAGE: &str = "cargo ambler: scaffolds and checks Ambler projects (Ambler Tooling Protocol)

usage:
  cargo ambler node <name> (--outcomes <a,b> | --terminal) [--fallible] [--state <field>[:<Type>]]...
  cargo ambler walk <name> --nodes <start,next,...> [--edge <node>.<Outcome>=<node|halt>]... [--no-runner]
  cargo ambler util <name>
  cargo ambler check [--lint-only]

exit codes: 0 success, 1 a check failed or the thing to create already exists, 2 invalid
arguments or an unexpected error";

/// The usage of `check`.
pub const CHECK_USAGE: &str = "usage: cargo ambler check [--lint-only]";

/// Where [`Route`] leaves the walk: the verb to run, or `Rejected` with the reason recorded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Node,
    Walk,
    Util,
    Check,
    Help,
    Rejected,
}

/// Takes the verb off the arguments and picks the step that runs it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Route;

impl Route {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl<S> Node<S> for Route
where
    S: HasArgs + HasLintOnly + HasMessage + HasExitCode + Send,
{
    type Outcome = Outcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<Outcome, Infallible> {
        let args = state.args_mut();
        if args.first().is_some_and(|arg| arg == "ambler") {
            args.remove(0);
        }
        if args.is_empty() {
            return Ok(reject(state, USAGE.to_string()));
        }
        let verb = args.remove(0);
        Ok(match verb.as_str() {
            "node" => Outcome::Node,
            "walk" => Outcome::Walk,
            "util" => Outcome::Util,
            "check" => {
                if !state.args().iter().all(|arg| arg == "--lint-only") {
                    return Ok(reject(state, CHECK_USAGE.to_string()));
                }
                *state.lint_only_mut() = !state.args().is_empty();
                Outcome::Check
            }
            "help" | "--help" | "-h" => Outcome::Help,
            _ => reject(state, format!("Unknown command: {verb}\n\n{USAGE}")),
        })
    }
}

/// Records why the invocation is invalid, to exit 2.
fn reject(state: &mut (impl HasMessage + HasExitCode), message: String) -> Outcome {
    *state.message_mut() = Some(message);
    *state.exit_code_mut() = 2;
    Outcome::Rejected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::State;

    async fn route(args: &[&str]) -> (Outcome, State) {
        let mut state = State::with_args(args);
        let Ok(outcome) = Route::new().run(&mut state).await;
        (outcome, state)
    }

    fn accepted(state: &State) {
        assert_eq!(state.message, None);
        assert_eq!(state.exit_code, 0);
    }

    fn rejected(state: &State, message: &str) {
        assert_eq!(state.message.as_deref(), Some(message));
        assert_eq!(state.exit_code, 2);
    }

    #[tokio::test]
    async fn routes_node_and_leaves_its_arguments() {
        let (outcome, state) = route(&["node", "count", "--terminal"]).await;

        assert_eq!(outcome, Outcome::Node);
        assert_eq!(state.args, ["count", "--terminal"]);
        accepted(&state);
    }

    #[tokio::test]
    async fn routes_walk_and_leaves_its_arguments() {
        let (outcome, state) = route(&["walk", "counter", "--nodes", "start"]).await;

        assert_eq!(outcome, Outcome::Walk);
        assert_eq!(state.args, ["counter", "--nodes", "start"]);
        accepted(&state);
    }

    #[tokio::test]
    async fn routes_util_and_leaves_its_arguments() {
        let (outcome, state) = route(&["util", "fs"]).await;

        assert_eq!(outcome, Outcome::Util);
        assert_eq!(state.args, ["fs"]);
        accepted(&state);
    }

    #[tokio::test]
    async fn drops_the_leading_ambler_that_cargo_passes() {
        let (outcome, state) = route(&["ambler", "util", "fs"]).await;

        assert_eq!(outcome, Outcome::Util);
        assert_eq!(state.args, ["fs"]);
    }

    #[tokio::test]
    async fn routes_check_through_to_the_checks_by_default() {
        let (outcome, state) = route(&["check"]).await;

        assert_eq!(outcome, Outcome::Check);
        assert!(!state.lint_only);
        accepted(&state);
    }

    #[tokio::test]
    async fn routes_check_with_lint_only() {
        let (outcome, state) = route(&["check", "--lint-only"]).await;

        assert_eq!(outcome, Outcome::Check);
        assert!(state.lint_only);
        accepted(&state);
    }

    #[tokio::test]
    async fn rejects_another_check_argument_with_the_check_usage() {
        let (outcome, state) = route(&["check", "--fix"]).await;

        assert_eq!(outcome, Outcome::Rejected);
        rejected(&state, CHECK_USAGE);
    }

    #[tokio::test]
    async fn routes_help_however_it_is_spelt() {
        for verb in ["help", "--help", "-h"] {
            let (outcome, state) = route(&[verb]).await;

            assert_eq!(outcome, Outcome::Help, "{verb}");
            accepted(&state);
        }
    }

    #[tokio::test]
    async fn rejects_no_verb_with_the_usage() {
        for args in [&[][..], &["ambler"][..]] {
            let (outcome, state) = route(args).await;

            assert_eq!(outcome, Outcome::Rejected, "{args:?}");
            rejected(&state, USAGE);
        }
    }

    #[tokio::test]
    async fn rejects_an_unknown_verb_naming_it() {
        let (outcome, state) = route(&["frobnicate", "x"]).await;

        assert_eq!(outcome, Outcome::Rejected);
        rejected(&state, &format!("Unknown command: frobnicate\n\n{USAGE}"));
    }
}
