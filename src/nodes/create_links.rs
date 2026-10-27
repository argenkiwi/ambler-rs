use crate::Node;
use crate::nodes::{HasError, HasLinks, fail};
use crate::utils::{Index, Out};
use std::convert::Infallible;

/// Where [`CreateLinks`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Recorded, or there was nothing to record.
    Recorded,
    /// The index rejected the links; the error was printed.
    Failed,
}

/// Records the pending links in the index's link graph, pointing out of `from_id`.
///
/// Shared by `create` and `link`.
#[derive(Clone, Debug, Default)]
pub struct CreateLinks<U> {
    utils: U,
}

impl<U> CreateLinks<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for CreateLinks<U>
where
    S: HasLinks + HasError + Send,
    U: Index + Out + Sync,
{
    type Outcome = Outcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<Outcome, Infallible> {
        let Some(from) = state.source_id().clone() else {
            fail(&self.utils, state, "no source note for the links".into());
            return Ok(Outcome::Failed);
        };
        match self.utils.add_links(&from, state.links()) {
            Ok(()) => Ok(Outcome::Recorded),
            Err(e) => {
                fail(&self.utils, state, e);
                Ok(Outcome::Failed)
            }
        }
    }
}
