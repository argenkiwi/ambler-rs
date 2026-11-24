//! The `cargo ambler` command line: runs the walk for the verb; see `specs/cli.md`.
//!
//! Cargo runs `cargo-ambler ambler <verb> …` for `cargo ambler <verb> …`, so a leading `ambler`
//! argument is dropped; the binary also runs directly as `cargo-ambler <verb> …`.

use crate::walks::{self, Error};
use std::process::ExitCode;

const HELP: &str = "cargo ambler: scaffolds and checks Ambler projects (Ambler Tooling Protocol)

usage:
  cargo ambler node <name> (--outcomes <a,b> | --terminal) [--fallible] [--state <field>[:<Type>]]...
  cargo ambler walk <name> --nodes <start,next,...> [--edge <node>.<Outcome>=<node|halt>]... [--no-runner]
  cargo ambler util <name>
  cargo ambler check [--lint-only]

exit codes: 0 success, 1 a check failed or the thing to create already exists, 2 invalid
arguments or an unexpected error";

/// Runs the CLI for `args` (the command line without the program name) and returns the exit
/// code of the Ambler Tooling Protocol. Embed it in a project-local binary, such as an `xtask`,
/// to run `cargo ambler` with nothing installed on the PATH.
pub async fn run(mut args: Vec<String>) -> ExitCode {
    if args.first().is_some_and(|arg| arg == "ambler") {
        args.remove(0);
    }
    let verb = if args.is_empty() {
        String::new()
    } else {
        args.remove(0)
    };
    match verb.as_str() {
        "node" => exit_code(walks::node::run(args).await),
        "walk" => exit_code(walks::walk::run(args).await),
        "util" => exit_code(walks::util::run(args).await),
        "check" => exit_code(walks::check::run(args).await),
        "help" | "--help" | "-h" => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        "" => {
            eprintln!("{HELP}");
            ExitCode::from(2)
        }
        _ => {
            eprintln!("Unknown command: {verb}\n\n{HELP}");
            ExitCode::from(2)
        }
    }
}

/// The exit code of the Ambler Tooling Protocol. A walk that failed or was rejected has already
/// said why; other errors are printed here.
fn exit_code(result: Result<(), Error>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if !matches!(error, Error::Failed | Error::Rejected) {
                eprintln!("cargo ambler: {error}");
            }
            ExitCode::from(error.exit_code())
        }
    }
}
