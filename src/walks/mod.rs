//! Walks: `cli`, the state machine of nodes behind every verb.

pub mod cli;

use crate::nodes::{
    HasArgs, HasExitCode, HasLintOnly, HasMessage, HasPlan, inspect, verify, write,
};
use crate::scaffold::Plan;

/// Why a verb did not complete. Each maps to an exit code of the Ambler Tooling Protocol.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A check failed, or the thing to create already exists. The walk has already said why.
    /// Exit code 1.
    #[error("the command failed")]
    Failed,
    /// The invocation was invalid, or this is not an Ambler project. The walk has already said
    /// why. Exit code 2.
    #[error("the command was rejected")]
    Rejected,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Write(#[from] write::Error),
    #[error(transparent)]
    Inspect(#[from] inspect::Error),
    #[error(transparent)]
    Verify(#[from] verify::Error),
}

impl Error {
    /// The process exit code: 1 for a failure, 2 for a rejected invocation or anything
    /// unexpected.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Failed => 1,
            _ => 2,
        }
    }
}

/// The state of the `cli` walk.
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub args: Vec<String>,
    pub lint_only: bool,
    pub plan: Plan,
    pub message: Option<String>,
    pub exit_code: u8,
}

impl State {
    fn new(args: Vec<String>) -> Self {
        Self {
            args,
            ..Self::default()
        }
    }

    #[cfg(test)]
    pub fn with_args(args: &[&str]) -> Self {
        Self::new(args.iter().map(ToString::to_string).collect())
    }
}

impl HasArgs for State {
    fn args(&self) -> &[String] {
        &self.args
    }
    fn args_mut(&mut self) -> &mut Vec<String> {
        &mut self.args
    }
}

impl HasLintOnly for State {
    fn lint_only(&self) -> bool {
        self.lint_only
    }
    fn lint_only_mut(&mut self) -> &mut bool {
        &mut self.lint_only
    }
}

impl HasExitCode for State {
    fn exit_code(&self) -> u8 {
        self.exit_code
    }
    fn exit_code_mut(&mut self) -> &mut u8 {
        &mut self.exit_code
    }
}

impl HasPlan for State {
    fn plan(&self) -> &Plan {
        &self.plan
    }
    fn plan_mut(&mut self) -> &mut Plan {
        &mut self.plan
    }
}

impl HasMessage for State {
    fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
    fn message_mut(&mut self) -> &mut Option<String> {
        &mut self.message
    }
}
