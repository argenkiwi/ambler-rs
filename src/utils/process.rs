//! Running other programs: the checks and the formatter.

use std::io;
use std::process::Command;

/// Edition the generated code is written for.
const EDITION: &str = "2024";

/// Runs `program` with `args`, sharing this process's terminal, and reports whether it succeeded.
///
/// # Errors
///
/// Returns the I/O error if the program could not be started.
pub fn run(program: &str, args: &[&str]) -> io::Result<bool> {
    Ok(Command::new(program).args(args).status()?.success())
}

/// Formats the Rust source files at `paths` in place with rustfmt.
///
/// Best effort: generated code is valid without it, so a missing formatter is not an error.
pub fn format(paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    let _ = Command::new("rustfmt")
        .args(["--edition", EDITION])
        .args(paths)
        .status();
}
