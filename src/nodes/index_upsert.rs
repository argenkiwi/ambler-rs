use crate::Node;
use crate::nodes::{HasEmbedding, HasError, HasNote, fail};
use crate::utils::{Index, Out};
use std::convert::Infallible;

/// Where [`IndexUpsert`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Indexed,
    /// The index rejected the note; the error was printed.
    Failed,
}

/// Upserts the note into the index, hashing its body so a later reindex can tell it changed.
///
/// Shared by `create`, `update` and `reindex`. No embedding leaves any stored vector alone.
#[derive(Clone, Debug, Default)]
pub struct IndexUpsert<U> {
    utils: U,
}

impl<U> IndexUpsert<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for IndexUpsert<U>
where
    S: HasNote + HasEmbedding + HasError + Send,
    U: Index + Out + Sync,
{
    type Outcome = Outcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<Outcome, Infallible> {
        let Some(note) = state.note().clone() else {
            fail(&self.utils, state, "no note to index".into());
            return Ok(Outcome::Failed);
        };
        match self.utils.upsert(&note, state.embedding().as_deref()) {
            Ok(()) => Ok(Outcome::Indexed),
            Err(e) => {
                fail(&self.utils, state, e);
                Ok(Outcome::Failed)
            }
        }
    }
}
