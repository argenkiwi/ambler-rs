//! The `azk` command line: runs the walk for the verb in `args[0]`; see `specs/*.md`.
//!
//! There is deliberately no routing walk: a [`Walk`](ambler::Walk) has one state, and the nodes
//! the verbs share have a different successor per verb. Choosing a verb is a one-off lookup.

use ambler::walks;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let verb = args.next().unwrap_or_default();
    let rest: Vec<String> = args.collect();
    match verb.to_lowercase().as_str() {
        "search" => exit_code(walks::search::run(rest).await),
        "create" => exit_code(walks::create::run(rest).await),
        "get" => exit_code(walks::get::run(rest).await),
        "update" => exit_code(walks::update::run(rest).await),
        "delete" => exit_code(walks::delete::run(rest).await),
        "link" => exit_code(walks::link::run(rest).await),
        "reindex" => exit_code(walks::reindex::run(rest).await),
        "help" => exit_code(walks::help::run(rest).await),
        "" => exit_code(walks::help::run(Vec::new()).await),
        _ => exit_code(walks::help::run(vec![verb]).await),
    }
}

/// Exit 1 for a bad invocation, 0 otherwise. The walk has already printed any usage message, so
/// the error is not printed again (returning it from `main` would print its `Debug` form).
fn exit_code<E>(result: Result<(), E>) -> ExitCode {
    if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
