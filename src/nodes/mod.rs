//! Nodes: atomic steps, and the state traits they share.

pub mod help;
pub mod inspect;
pub mod plan_node;
pub mod plan_util;
pub mod plan_walk;
pub mod report;
pub mod route;
pub mod verify;
pub mod write;

use crate::scaffold::Plan;

/// A state that has the verb's command-line arguments.
pub trait HasArgs {
    fn args(&self) -> &[String];
    fn args_mut(&mut self) -> &mut Vec<String>;
}

/// A state that has whether `check` stops after the conventions.
pub trait HasLintOnly {
    fn lint_only(&self) -> bool;
    fn lint_only_mut(&mut self) -> &mut bool;
}

/// A state that has the exit code to end with when the program failed: 1 or 2.
pub trait HasExitCode {
    fn exit_code(&self) -> u8;
    fn exit_code_mut(&mut self) -> &mut u8;
}

/// A state that has the changes a verb is about to make.
pub trait HasPlan {
    fn plan(&self) -> &Plan;
    fn plan_mut(&mut self) -> &mut Plan;
}

/// A state that has the reason the verb failed, if it did.
pub trait HasMessage {
    fn message(&self) -> Option<&str>;
    fn message_mut(&mut self) -> &mut Option<String>;
}
