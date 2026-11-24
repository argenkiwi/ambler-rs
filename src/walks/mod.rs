//! Walks: one per verb, each a small state machine of nodes.

pub mod check;
pub mod node;
pub mod util;
pub mod walk;

use crate::nodes::{HasArgs, HasMessage, HasPlan, inspect, verify, write};
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

/// The state every verb's walk shares.
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub args: Vec<String>,
    pub plan: Plan,
    pub message: Option<String>,
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

    /// A scaffolding walk that left a message was rejected, and has reported it. Creating what
    /// already exists is a failure (exit code 1); anything else is a bad invocation (2).
    fn finish(self) -> Result<(), Error> {
        match self.message {
            Some(message) if message.contains("already") => Err(Error::Failed),
            Some(_) => Err(Error::Rejected),
            None => Ok(()),
        }
    }

    /// A check that left a message failed, and has reported it.
    fn finish_check(self) -> Result<(), Error> {
        match self.message {
            Some(_) => Err(Error::Failed),
            None => Ok(()),
        }
    }
}

impl HasArgs for State {
    fn args(&self) -> &[String] {
        &self.args
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
