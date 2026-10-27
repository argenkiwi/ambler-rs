//! The `azk get` verb as a standalone walk. See `specs/get.md`.
//!
//! Run with `cargo run --example get -- <args>`; the walk itself lives in `ambler::azk::get`
//! so the `azk` binary can dispatch to it too.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ambler::azk::get::run(std::env::args().skip(1).collect()).await
}
