//! The `azk update` verb as a standalone walk. See `specs/update.md`.
//!
//! Run with `cargo run --example update -- <args>`; the walk itself lives in `ambler::azk::update`
//! so the `azk` binary can dispatch to it too.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ambler::azk::update::run(std::env::args().skip(1).collect()).await
}
