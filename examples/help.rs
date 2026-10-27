//! The `azk help` verb as a standalone walk. See `specs/help.md`.
//!
//! Run with `cargo run --example help -- <args>`; the walk itself lives in `ambler::azk::help`
//! so the `azk` binary can dispatch to it too.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ambler::azk::help::run(std::env::args().skip(1).collect()).await
}
