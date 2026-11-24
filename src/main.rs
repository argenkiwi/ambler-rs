//! The `cargo-ambler` binary: a thin shim over [`ambler::cli::run`].

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ambler::cli::run(std::env::args().skip(1).collect()).await
}
