//! The `azk create` verb as a standalone walk. See `specs/create.md`.
//!
//! Run with `cargo run --example create -- <args>`; the walk itself lives in `ambler::azk::create`
//! so the `azk` binary can dispatch to it too.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ambler::azk::create::run(std::env::args().skip(1).collect()).await
}
