//! The `azk search` verb as a standalone walk. See `specs/search.md`.
//!
//! Run with `cargo run --example search -- <args>`; the walk itself lives in `ambler::azk::search`
//! so the `azk` binary can dispatch to it too.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ambler::azk::search::run(std::env::args().skip(1).collect()).await
}
