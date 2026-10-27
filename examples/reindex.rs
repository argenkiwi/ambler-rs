//! The `azk reindex` verb as a standalone walk. See `specs/reindex.md`.
//!
//! Run with `cargo run --example reindex -- <args>`; the walk itself lives in `ambler::azk::reindex`
//! so the `azk` binary can dispatch to it too.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ambler::azk::reindex::run(std::env::args().skip(1).collect()).await
}
