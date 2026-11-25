//! The `cargo-ambler` binary: runs the `cli` walk, see `specs/cli.md`.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ambler::walks::cli::run(std::env::args().skip(1).collect()).await
}
