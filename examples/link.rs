//! The `azk link` verb as a standalone walk. See `specs/link.md`.
//!
//! Run with `cargo run --example link -- <args>`; the walk itself lives in `ambler::azk::link`
//! so the `azk` binary can dispatch to it too.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ambler::azk::link::run(std::env::args().skip(1).collect()).await
}
