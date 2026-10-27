//! The `azk` command line. Dispatches to one walk per verb; see `specs/*.md`.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    ambler::azk::dispatch(std::env::args().skip(1).collect()).await
}
