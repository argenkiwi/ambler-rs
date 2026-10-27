use crate::Node;
use crate::nodes::HasUsage;
use crate::utils::Out;
use std::convert::Infallible;

/// Prints the pending usage message to stderr. Terminal: the process then exits 1.
///
/// Shared by every walk that can be invoked badly.
#[derive(Clone, Debug, Default)]
pub struct Usage<U> {
    utils: U,
}

impl<U> Usage<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Usage<U>
where
    S: HasUsage + Send,
    U: Out + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        if let Some(message) = state.usage() {
            self.utils.eprint(message);
        }
        Ok(())
    }
}
