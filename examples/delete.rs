//! The `azk delete` verb as a standalone walk. See `specs/delete.md`.
//!
//! Run with `cargo run --example delete -- <args>`; the walk itself lives in `ambler::azk::delete`
//! so the `azk` binary can dispatch to it too.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ambler::azk::delete::run(std::env::args().skip(1).collect()).await
}
