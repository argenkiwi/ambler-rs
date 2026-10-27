use crate::Node;
use crate::nodes::{HasError, HasNote, fail, print_json};
use crate::utils::{Index, Out};
use serde_json::json;
use std::convert::Infallible;

/// Prints the note the existence check read, with every link touching it in either direction.
/// Terminal.
#[derive(Clone, Debug, Default)]
pub struct Get<U> {
    utils: U,
}

impl<U> Get<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Get<U>
where
    S: HasNote + HasError + Send,
    U: Index + Out + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        let Some(note) = state.note().clone() else {
            fail(&self.utils, state, "no note to get".into());
            return Ok(());
        };
        match self.utils.links_for(&note.id) {
            Ok(links) => print_json(
                &self.utils,
                &json!({
                    "id": note.id,
                    "title": note.title,
                    "tags": note.tags,
                    "created": note.created,
                    "updated": note.updated,
                    "body": note.body,
                    "links": links,
                }),
            ),
            Err(e) => fail(&self.utils, state, e),
        }
        Ok(())
    }
}
