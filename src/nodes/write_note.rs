use crate::Node;
use crate::nodes::{HasError, HasNote, fail};
use crate::utils::{NoteStore, Out};
use std::convert::Infallible;

/// Where [`WriteNote`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Written,
    /// The write failed; the error was printed.
    Failed,
}

/// Persists the note to its Markdown file — the source of truth, written before the index.
///
/// Shared by `create`, `update` and `link`.
#[derive(Clone, Debug, Default)]
pub struct WriteNote<U> {
    utils: U,
}

impl<U> WriteNote<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for WriteNote<U>
where
    S: HasNote + HasError + Send,
    U: NoteStore + Out + Sync,
{
    type Outcome = Outcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<Outcome, Infallible> {
        let Some(note) = state.note().clone() else {
            fail(&self.utils, state, "no note to write".into());
            return Ok(Outcome::Failed);
        };
        match self.utils.write(&note) {
            Ok(()) => Ok(Outcome::Written),
            Err(e) => {
                fail(
                    &self.utils,
                    state,
                    format!("could not write note {}: {e}", note.id),
                );
                Ok(Outcome::Failed)
            }
        }
    }
}
