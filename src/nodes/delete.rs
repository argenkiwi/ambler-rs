use crate::Node;
use crate::nodes::{HasError, HasId, fail, print_json};
use crate::utils::{Index, NoteStore, Out};
use serde_json::json;
use std::convert::Infallible;

/// Removes the note's file, its index entry, and every link touching it, then confirms. Terminal.
///
/// Works from the id alone: the existence check upstream has ruled out a missing note.
#[derive(Clone, Debug, Default)]
pub struct Delete<U> {
    utils: U,
}

impl<U> Delete<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Delete<U>
where
    S: HasId + HasError + Send,
    U: Index + NoteStore + Out + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        let id = state.id().clone().unwrap_or_default();
        let removed = self
            .utils
            .remove(&id)
            .map_err(|e| format!("could not remove note {id}: {e}"))
            .and_then(|()| self.utils.delete(&id));
        match removed {
            Ok(()) => print_json(&self.utils, &json!({ "id": id, "deleted": true })),
            Err(e) => fail(&self.utils, state, e),
        }
        Ok(())
    }
}
