use crate::Node;
use crate::nodes::{HasError, HasId, HasNote, fail};
use crate::utils::{NoteStore, Out};
use std::convert::Infallible;

/// Where [`ExistsCheck`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The note was read and is now in the state.
    Found,
    /// No such note (or it could not be read); the error was printed.
    Missing,
}

/// Reads the note by id and attaches it, so downstream nodes don't read it again.
///
/// Shared by `get`, `update` and `delete`.
#[derive(Clone, Debug, Default)]
pub struct ExistsCheck<U> {
    utils: U,
}

impl<U> ExistsCheck<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for ExistsCheck<U>
where
    S: HasId + HasNote + HasError + Send,
    U: NoteStore + Out + Sync,
{
    type Outcome = Outcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<Outcome, Infallible> {
        let id = state.id().clone().unwrap_or_default();
        match self.utils.read(&id) {
            Ok(Some(note)) => {
                *state.note_mut() = Some(note);
                Ok(Outcome::Found)
            }
            Ok(None) => {
                fail(&self.utils, state, format!("note {id} not found"));
                Ok(Outcome::Missing)
            }
            Err(e) => {
                fail(&self.utils, state, format!("could not read note {id}: {e}"));
                Ok(Outcome::Missing)
            }
        }
    }
}
