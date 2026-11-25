//! The `cargo ambler` command line, for project-local binaries such as an `xtask`: see
//! `specs/cli.md`.

/// Runs `cargo ambler <verb> …` and returns the exit code of the Ambler Tooling Protocol. Embed
/// it in a project-local binary to run `cargo ambler` with nothing installed on the PATH.
pub use crate::walks::cli::run;
