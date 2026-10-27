//! The eight verbs of the `azk` command line, one walk each.
//!
//! There is deliberately no routing walk: a [`Walk`](crate::Walk) has one state, and the nodes
//! the verbs share have a different successor per verb. Choosing a verb is a one-off lookup, done
//! by [`dispatch`].

use std::process::ExitCode;

pub mod create;
pub mod delete;
pub mod get;
pub mod help;
pub mod link;
pub mod reindex;
pub mod search;
pub mod update;

/// Runs the verb named by `args[0]` with the rest as its arguments, and returns the exit code:
/// 1 for a bad invocation, 0 otherwise (failures inside a verb are printed as `{ "error" }`).
pub async fn dispatch(args: Vec<String>) -> ExitCode {
    let mut args = args.into_iter();
    let verb = args.next().unwrap_or_default();
    let rest: Vec<String> = args.collect();
    match verb.to_lowercase().as_str() {
        "search" => search::run(rest).await,
        "create" => create::run(rest).await,
        "get" => get::run(rest).await,
        "update" => update::run(rest).await,
        "delete" => delete::run(rest).await,
        "link" => link::run(rest).await,
        "reindex" => reindex::run(rest).await,
        "help" => help::run(rest).await,
        "" => help::run(Vec::new()).await,
        _ => help::run(vec![verb]).await,
    }
}

/// Exit 1 when a node queued a usage message, 0 otherwise.
pub(crate) fn exit_code(usage: &Option<String>) -> ExitCode {
    if usage.is_some() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
