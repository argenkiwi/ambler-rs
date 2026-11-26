//! `cargo ambler <verb> …`: routes the verb to the nodes that scaffold or check the project. See
//! `specs/cli.md`.

use super::State;
use crate::nodes::help::{self, Help};
use crate::nodes::inspect::{self, Inspect};
use crate::nodes::plan_node::{self, PlanNode};
use crate::nodes::plan_util::{self, PlanUtil};
use crate::nodes::plan_walk::{self, PlanWalk};
use crate::nodes::report::{self, Report};
use crate::nodes::route::{self, Route};
use crate::nodes::verify::{self, Verify};
use crate::nodes::write::{self, Write};
use crate::{Node as _, Transition, Walk};
use std::process::ExitCode;

/// Why the program did not complete; [`Error::exit_code`] maps it to the protocol's exit code.
pub type Error = super::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id {
    Route,
    PlanNode,
    PlanWalk,
    PlanUtil,
    Write,
    Inspect,
    Verify,
    Help,
    Report,
}

#[derive(Default)]
#[allow(clippy::struct_field_names)]
struct Cli<
    A = plan_node::SystemUtils,
    B = plan_walk::SystemUtils,
    C = plan_util::SystemUtils,
    D = write::SystemUtils,
    E = inspect::SystemUtils,
    F = verify::SystemUtils,
    G = help::SystemUtils,
    H = report::SystemUtils,
> {
    route: Route,
    plan_node: PlanNode<A>,
    plan_walk: PlanWalk<B>,
    plan_util: PlanUtil<C>,
    write: Write<D>,
    inspect: Inspect<E>,
    verify: Verify<F>,
    help: Help<G>,
    report: Report<H>,
}

impl Cli {
    /// Production wiring: every node uses its `SystemUtils`.
    fn new() -> Self {
        Self::default()
    }
}

impl<A, B, C, D, E, F, G, H> Walk for Cli<A, B, C, D, E, F, G, H>
where
    A: plan_node::Utils + Sync,
    B: plan_walk::Utils + Sync,
    C: plan_util::Utils + Sync,
    D: write::Utils + Sync,
    E: inspect::Utils + Sync,
    F: verify::Utils + Sync,
    G: help::Utils + Sync,
    H: report::Utils + Sync,
{
    type Id = Id;
    type State = State;
    type Error = Error;

    async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, Error> {
        Ok(match id {
            Id::Route => {
                let Ok(outcome) = self.route.run(state).await;
                match outcome {
                    route::Outcome::Node => Transition::Next(Id::PlanNode),
                    route::Outcome::Walk => Transition::Next(Id::PlanWalk),
                    route::Outcome::Util => Transition::Next(Id::PlanUtil),
                    route::Outcome::Check => Transition::Next(Id::Inspect),
                    route::Outcome::Help => Transition::Next(Id::Help),
                    route::Outcome::Rejected => Transition::Next(Id::Report),
                }
            }
            Id::PlanNode => {
                let Ok(outcome) = self.plan_node.run(state).await;
                match outcome {
                    plan_node::Outcome::Planned => Transition::Next(Id::Write),
                    plan_node::Outcome::Rejected => Transition::Next(Id::Report),
                }
            }
            Id::PlanWalk => {
                let Ok(outcome) = self.plan_walk.run(state).await;
                match outcome {
                    plan_walk::Outcome::Planned => Transition::Next(Id::Write),
                    plan_walk::Outcome::Rejected => Transition::Next(Id::Report),
                }
            }
            Id::PlanUtil => {
                let Ok(outcome) = self.plan_util.run(state).await;
                match outcome {
                    plan_util::Outcome::Planned => Transition::Next(Id::Write),
                    plan_util::Outcome::Rejected => Transition::Next(Id::Report),
                }
            }
            Id::Write => {
                self.write.run(state).await?;
                Transition::Halt
            }
            Id::Inspect => match self.inspect.run(state).await? {
                inspect::Outcome::Clean if state.lint_only => Transition::Halt,
                inspect::Outcome::Clean => Transition::Next(Id::Verify),
                inspect::Outcome::Violations => Transition::Next(Id::Report),
            },
            Id::Verify => match self.verify.run(state).await? {
                verify::Outcome::Passed => Transition::Halt,
                verify::Outcome::Failed => Transition::Next(Id::Report),
            },
            Id::Help => {
                let Ok(outcome) = self.help.run(state).await;
                match outcome {
                    help::Outcome::Printed => Transition::Halt,
                    help::Outcome::Unknown => Transition::Next(Id::Report),
                }
            }
            Id::Report => {
                let Ok(()) = self.report.run(state).await;
                Transition::Halt
            }
        })
    }
}

/// Runs `cargo ambler <verb> …` for `args` (the command line without the program name) and
/// returns the exit code of the Ambler Tooling Protocol: 0 on success, 1 if a check failed or the
/// thing to create already exists, 2 for an invalid invocation or an unexpected error.
pub async fn run(args: Vec<String>) -> ExitCode {
    let mut state = State::new(args);
    let result = Cli::new().run(Id::Route, &mut state).await;
    match result.and_then(|()| finish(&state)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // A failed or rejected walk has already said why.
            if !matches!(error, Error::Failed | Error::Rejected) {
                eprintln!("cargo ambler: {error}");
            }
            ExitCode::from(error.exit_code())
        }
    }
}

/// The outcome of a walk that ran to the end: a node that failed recorded its exit code.
fn finish(state: &State) -> Result<(), Error> {
    match state.exit_code {
        0 => Ok(()),
        1 => Err(Error::Failed),
        _ => Err(Error::Rejected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::route::USAGE;
    use crate::testing::Fake;

    const PROJECT: &[(&str, &str)] = &[
        ("Cargo.toml", "[package]\nname = \"app\"\n"),
        ("src/nodes/mod.rs", "//! Nodes.\n\npub mod stop;\n"),
        (
            "src/nodes/stop.rs",
            include_str!("../../templates/rust/example/stop.rs"),
        ),
        ("src/walks/mod.rs", "//! Walks.\n"),
        ("src/utils/mod.rs", "pub mod terminal;\n"),
        ("src/utils/terminal.rs", "//! Terminal.\n"),
    ];

    /// The final state and how the program ends, run over `fake`.
    async fn cli(fake: &Fake, args: &[&str]) -> (State, Result<(), Error>) {
        let walk = Cli {
            route: Route::new(),
            plan_node: PlanNode::with_utils(fake),
            plan_walk: PlanWalk::with_utils(fake),
            plan_util: PlanUtil::with_utils(fake),
            write: Write::with_utils(fake),
            inspect: Inspect::with_utils(fake),
            verify: Verify::with_utils(fake),
            help: Help::with_utils(fake),
            report: Report::with_utils(fake),
        };
        let mut state = State::with_args(args);
        walk.run(Id::Route, &mut state).await.unwrap();
        let result = finish(&state);
        (state, result)
    }

    #[tokio::test]
    async fn node_writes_the_node_registers_it_and_formats_it() {
        let fake = Fake::with_files(PROJECT);

        let (_, result) = cli(
            &fake,
            &[
                "ambler",
                "node",
                "ask",
                "--outcomes",
                "answered",
                "--state",
                "answer:String",
            ],
        )
        .await;

        assert!(result.is_ok());
        assert!(
            fake.file("src/nodes/ask.rs")
                .unwrap()
                .contains("pub struct Ask<U = SystemUtils>")
        );
        let modules = fake.file("src/nodes/mod.rs").unwrap();
        assert!(modules.contains("pub mod ask;\npub mod stop;\n"));
        assert!(modules.contains("pub trait HasAnswer {"));
        assert_eq!(*fake.formatted.lock().unwrap(), ["src/nodes/ask.rs"]);
        assert!(fake.eprinted().is_empty());
    }

    #[tokio::test]
    async fn walk_writes_the_walk_its_spec_and_its_runner() {
        let fake = Fake::with_files(PROJECT);

        let (_, result) = cli(&fake, &["walk", "finish", "--nodes", "stop"]).await;

        assert!(result.is_ok());
        assert!(
            fake.file("src/walks/finish.rs")
                .unwrap()
                .contains("struct Finish<A = stop::SystemUtils>")
        );
        assert_eq!(
            fake.file("src/walks/mod.rs").as_deref(),
            Some("//! Walks.\n\npub mod finish;\n")
        );
        assert!(fake.file("specs/finish.md").unwrap().contains("### Stop"));
        assert!(
            fake.file("src/main.rs")
                .unwrap()
                .contains("app::walks::finish::run()")
        );
    }

    #[tokio::test]
    async fn util_writes_the_module_and_registers_it() {
        let fake = Fake::with_files(PROJECT);

        let (_, result) = cli(&fake, &["util", "journal"]).await;

        assert!(result.is_ok());
        assert!(
            fake.file("src/utils/journal.rs")
                .unwrap()
                .starts_with("//! ")
        );
        assert_eq!(
            fake.file("src/utils/mod.rs").as_deref(),
            Some("pub mod journal;\npub mod terminal;\n")
        );
    }

    #[tokio::test]
    async fn a_rejected_scaffold_is_reported_writes_nothing_and_exits_2() {
        let fake = Fake::with_files(PROJECT);

        let (_, result) = cli(&fake, &["walk", "finish", "--nodes", "start,stop"]).await;

        assert!(matches!(result, Err(Error::Rejected)));
        assert!(fake.eprinted()[0].contains("cargo ambler node start"));
        assert_eq!(fake.paths().len(), PROJECT.len());
    }

    #[tokio::test]
    async fn creating_what_already_exists_is_reported_and_exits_1() {
        let fake = Fake::with_files(PROJECT);

        let (_, result) = cli(&fake, &["util", "terminal"]).await;

        assert!(matches!(result, Err(Error::Failed)));
        assert!(fake.eprinted()[0].contains("already exists"));
        assert_eq!(fake.paths().len(), PROJECT.len());
    }

    #[tokio::test]
    async fn check_goes_on_to_cargo_for_a_clean_project_and_passes() {
        let fake = Fake::with_files(PROJECT);

        let (_, result) = cli(&fake, &["check"]).await;

        assert!(result.is_ok());
        assert_eq!(fake.printed().last().unwrap(), "$ cargo test");
        assert!(fake.eprinted().is_empty());
    }

    #[tokio::test]
    async fn check_with_lint_only_skips_cargo() {
        let fake = Fake::with_files(PROJECT);

        let (_, result) = cli(&fake, &["check", "--lint-only"]).await;

        assert!(result.is_ok());
        assert!(fake.printed().is_empty());
    }

    #[tokio::test]
    async fn check_reports_violations_without_running_cargo_and_exits_1() {
        let mut files = PROJECT.to_vec();
        files.push(("src/nodes/ask.rs", "pub struct Ask;\n"));
        let fake = Fake::with_files(&files);

        let (_, result) = cli(&fake, &["check"]).await;

        assert!(matches!(result, Err(Error::Failed)));
        assert!(
            fake.printed()
                .iter()
                .all(|line| line.starts_with("src/nodes/ask.rs:1: "))
        );
        assert!(fake.eprinted()[0].contains("convention violation"));
    }

    #[tokio::test]
    async fn check_reports_a_failing_cargo_command_and_exits_1() {
        let fake = Fake {
            failing: Some("test"),
            ..Fake::with_files(PROJECT)
        };

        let (_, result) = cli(&fake, &["check"]).await;

        assert!(matches!(result, Err(Error::Failed)));
        assert_eq!(fake.eprinted(), ["`cargo test` failed"]);
    }

    #[tokio::test]
    async fn help_prints_the_general_guide() {
        let fake = Fake::default();

        let (_, result) = cli(&fake, &["--help"]).await;

        assert!(result.is_ok());
        assert_eq!(fake.printed(), [crate::guides::guide("").unwrap()]);
    }

    #[tokio::test]
    async fn help_prints_a_topic() {
        let fake = Fake::default();

        let (_, result) = cli(&fake, &["ambler", "help", "walk"]).await;

        assert!(result.is_ok());
        assert_eq!(fake.printed(), [crate::guides::guide("walk").unwrap()]);
    }

    #[tokio::test]
    async fn help_for_an_unknown_topic_is_reported_and_exits_1() {
        let fake = Fake::default();

        let (state, result) = cli(&fake, &["help", "bogus"]).await;

        assert!(matches!(result, Err(Error::Failed)));
        assert_eq!(state.exit_code, 1);
        assert!(fake.printed().is_empty());
        assert_eq!(fake.eprinted().len(), 1);
    }

    #[tokio::test]
    async fn an_unknown_verb_is_reported_with_the_usage_and_exits_2() {
        let fake = Fake::default();

        let (state, result) = cli(&fake, &["frobnicate"]).await;

        assert!(matches!(result, Err(Error::Rejected)));
        assert_eq!(state.exit_code, 2);
        assert_eq!(
            fake.eprinted(),
            [format!("Unknown command: frobnicate\n\n{USAGE}")]
        );
    }
}
